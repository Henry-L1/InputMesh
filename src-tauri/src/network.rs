use std::{
    cmp::Reverse,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket},
    sync::Arc,
    time::Duration,
};

use dashmap::{DashMap, DashSet};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream, tcp::OwnedReadHalf},
    sync::{Semaphore, mpsc, watch},
    task::JoinHandle,
    time::{MissedTickBehavior, timeout},
};
use uuid::Uuid;

use crate::{
    crypto::{
        MAX_NOISE_FRAME_LEN, NoiseSession, perform_initiator_handshake,
        perform_responder_handshake, read_length_prefixed, write_length_prefixed,
        write_wire_message,
    },
    discovery::{DiscoveryEvent, DiscoveryHandle, DiscoveryRecord},
    protocol::{ErrorMessage, Hello, Message, Ping, Pong, WireMessage},
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(4);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(8);
const FRAME_TIMEOUT: Duration = Duration::from_secs(20);
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const OUTBOUND_QUEUE_CAPACITY: usize = 512;
const PORT_SEARCH_ATTEMPTS: u16 = 16;
const MAX_ACTIVE_CONNECTIONS: usize = 64;

#[derive(Debug, Clone)]
pub struct LocalNode {
    pub device_id: String,
    pub name: String,
    pub platform: crate::protocol::Platform,
    pub os_version: String,
    pub app_version: String,
    pub private_key: [u8; 32],
    pub preferred_port: u16,
}

impl LocalNode {
    fn hello(&self) -> WireMessage {
        WireMessage::new(Message::Hello(Hello {
            device_id: self.device_id.clone(),
            device_name: self.name.clone(),
            platform: self.platform,
            os_version: self.os_version.clone(),
            app_version: self.app_version.clone(),
            capabilities: vec![
                "screens-v1".into(),
                "layout-v1".into(),
                "input-v1".into(),
                "multi-origin-lease-v1".into(),
            ],
        }))
    }

    fn platform_name(&self) -> &'static str {
        match self.platform {
            crate::protocol::Platform::Macos => "macos",
            crate::protocol::Platform::Windows => "windows",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionMetadata {
    pub connection_id: Uuid,
    pub device_id: String,
    pub name: String,
    pub platform: crate::protocol::Platform,
    pub os_version: String,
    pub app_version: String,
    pub capabilities: Vec<String>,
    pub remote_static_key: [u8; 32],
    pub pairing_code: String,
    pub address: SocketAddr,
}

#[derive(Debug, Clone)]
pub enum NetworkEvent {
    Discovered(DiscoveryRecord),
    DiscoveryRemoved {
        device_id: String,
    },
    ConnectionReady(ConnectionMetadata),
    Message {
        connection_id: Uuid,
        device_id: String,
        message: WireMessage,
    },
    Disconnected {
        connection_id: Uuid,
        device_id: String,
        reason: String,
    },
    Error(String),
}

#[derive(Debug)]
enum ConnectionCommand {
    Send(WireMessage),
}

#[derive(Clone)]
struct ConnectionSlot {
    connection_id: Uuid,
    sender: mpsc::Sender<ConnectionCommand>,
    close: watch::Sender<Option<String>>,
}

pub struct NetworkService {
    port: u16,
    connections: Arc<DashMap<String, ConnectionSlot>>,
    shutdown: watch::Sender<bool>,
    discovery: Option<DiscoveryHandle>,
    tasks: Vec<JoinHandle<()>>,
}

impl NetworkService {
    pub async fn start(
        local: LocalNode,
        events: mpsc::UnboundedSender<NetworkEvent>,
    ) -> Result<Self, String> {
        let listener = bind_listener(local.preferred_port).await?;
        let port = listener
            .local_addr()
            .map_err(|error| error.to_string())?
            .port();
        let (shutdown, shutdown_rx) = watch::channel(false);
        let connections = Arc::new(DashMap::new());
        let dialing = Arc::new(DashSet::new());
        let known_records = Arc::new(DashMap::new());
        let connection_limit = Arc::new(Semaphore::new(MAX_ACTIVE_CONNECTIONS));

        let (discovery_tx, mut discovery_rx) = mpsc::unbounded_channel();
        let discovery = DiscoveryHandle::start(
            &local.device_id,
            &local.name,
            local.platform_name(),
            &local.os_version,
            port,
            discovery_tx,
        )?;

        let mut tasks = Vec::new();

        let listener_local = local.clone();
        let listener_events = events.clone();
        let listener_connections = connections.clone();
        let listener_shutdown = shutdown_rx.clone();
        let listener_limit = connection_limit.clone();
        tasks.push(tokio::spawn(async move {
            run_listener(
                listener,
                listener_local,
                listener_connections,
                listener_events,
                listener_shutdown,
                listener_limit,
            )
            .await;
        }));

        let discovery_local = local;
        let discovery_events = events;
        let discovery_connections = connections.clone();
        let discovery_dialing = dialing;
        let mut discovery_shutdown = shutdown_rx;
        let discovery_limit = connection_limit;
        let discovery_records = known_records;
        tasks.push(tokio::spawn(async move {
            let mut reconnect = tokio::time::interval(Duration::from_secs(2));
            reconnect.set_missed_tick_behavior(MissedTickBehavior::Delay);
            loop {
                tokio::select! {
                    changed = discovery_shutdown.changed() => {
                        if changed.is_err() || *discovery_shutdown.borrow() {
                            break;
                        }
                    }
                    Some(event) = discovery_rx.recv() => {
                        match event {
                            DiscoveryEvent::Resolved(record) => {
                                let _ = discovery_events.send(NetworkEvent::Discovered(record.clone()));
                                discovery_records.insert(record.device_id.clone(), record.clone());
                                try_spawn_dial(
                                    record,
                                    &discovery_local,
                                    &discovery_connections,
                                    &discovery_events,
                                    &discovery_dialing,
                                    &discovery_shutdown,
                                    &discovery_limit,
                                );
                            }
                            DiscoveryEvent::Removed { device_id } => {
                                discovery_records.remove(&device_id);
                                let _ = discovery_events.send(NetworkEvent::DiscoveryRemoved { device_id });
                            }
                            DiscoveryEvent::Error(error) => {
                                let _ = discovery_events.send(NetworkEvent::Error(format!("mDNS: {error}")));
                            }
                        }
                    }
                    _ = reconnect.tick() => {
                        let records: Vec<_> = discovery_records
                            .iter()
                            .map(|entry| entry.value().clone())
                            .collect();
                        for record in records {
                            try_spawn_dial(
                                record,
                                &discovery_local,
                                &discovery_connections,
                                &discovery_events,
                                &discovery_dialing,
                                &discovery_shutdown,
                                &discovery_limit,
                            );
                        }
                    }
                }
            }
        }));

        Ok(Self {
            port,
            connections,
            shutdown,
            discovery: Some(discovery),
            tasks,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn send(&self, peer_id: &str, message: WireMessage) -> Result<(), String> {
        let sender = self
            .connections
            .get(peer_id)
            .ok_or_else(|| format!("peer {peer_id} is not connected"))?
            .sender
            .clone();
        sender
            .try_send(ConnectionCommand::Send(message))
            .map_err(|error| format!("peer {peer_id} outbound queue unavailable: {error}"))
    }

    pub fn broadcast(&self, message: WireMessage) {
        let senders: Vec<_> = self
            .connections
            .iter()
            .map(|entry| entry.sender.clone())
            .collect();
        for sender in senders {
            let _ = sender.try_send(ConnectionCommand::Send(message.clone()));
        }
    }

    pub fn disconnect(&self, peer_id: &str, reason: impl Into<String>) {
        if let Some(slot) = self.connections.get(peer_id) {
            let _ = slot.close.send(Some(reason.into()));
        }
    }
}

fn try_spawn_dial(
    record: DiscoveryRecord,
    local: &LocalNode,
    connections: &Arc<DashMap<String, ConnectionSlot>>,
    events: &mpsc::UnboundedSender<NetworkEvent>,
    dialing: &Arc<DashSet<String>>,
    shutdown: &watch::Receiver<bool>,
    connection_limit: &Arc<Semaphore>,
) {
    let should_dial = local.device_id < record.device_id
        && !record.addresses.is_empty()
        && !connections.contains_key(&record.device_id)
        && dialing.insert(record.device_id.clone());
    if !should_dial {
        return;
    }
    let Ok(permit) = connection_limit.clone().try_acquire_owned() else {
        dialing.remove(&record.device_id);
        return;
    };
    let local = local.clone();
    let connections = connections.clone();
    let events = events.clone();
    let dialing = dialing.clone();
    let shutdown = shutdown.clone();
    tokio::spawn(async move {
        let _permit = permit;
        let device_id = record.device_id.clone();
        dial_peer(record, local, connections, events, shutdown).await;
        dialing.remove(&device_id);
    });
}

impl Drop for NetworkService {
    fn drop(&mut self) {
        let _ = self.shutdown.send(true);
        self.connections.clear();
        self.discovery.take();
        for task in self.tasks.drain(..) {
            task.abort();
        }
    }
}

async fn bind_listener(preferred_port: u16) -> Result<TcpListener, String> {
    let mut last_error = None;
    for offset in 0..PORT_SEARCH_ATTEMPTS {
        let Some(port) = preferred_port.checked_add(offset) else {
            break;
        };
        match TcpListener::bind(("0.0.0.0", port)).await {
            Ok(listener) => return Ok(listener),
            Err(error) => last_error = Some(error),
        }
    }
    Err(format!(
        "could not bind an InputMesh port near {preferred_port}: {}",
        last_error
            .map(|error| error.to_string())
            .unwrap_or_else(|| "no valid port".into())
    ))
}

async fn run_listener(
    listener: TcpListener,
    local: LocalNode,
    connections: Arc<DashMap<String, ConnectionSlot>>,
    events: mpsc::UnboundedSender<NetworkEvent>,
    mut shutdown: watch::Receiver<bool>,
    connection_limit: Arc<Semaphore>,
) {
    loop {
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    break;
                }
            }
            accepted = listener.accept() => match accepted {
                Ok((stream, address)) => {
                    let Ok(permit) = connection_limit.clone().try_acquire_owned() else {
                        tracing::warn!(%address, "connection limit reached; dropping inbound peer");
                        continue;
                    };
                    let local = local.clone();
                    let connections = connections.clone();
                    let events = events.clone();
                    let shutdown = shutdown.clone();
                    tokio::spawn(async move {
                        let _permit = permit;
                        run_connection(stream, address, false, None, local, connections, events, shutdown).await;
                    });
                }
                Err(error) => {
                    let _ = events.send(NetworkEvent::Error(format!("TCP accept failed: {error}")));
                    tokio::time::sleep(Duration::from_millis(250)).await;
                }
            }
        }
    }
}

async fn dial_peer(
    record: DiscoveryRecord,
    local: LocalNode,
    connections: Arc<DashMap<String, ConnectionSlot>>,
    events: mpsc::UnboundedSender<NetworkEvent>,
    shutdown: watch::Receiver<bool>,
) {
    for address in preferred_addresses(&record.addresses) {
        let socket = SocketAddr::new(address, record.port);
        match timeout(CONNECT_TIMEOUT, TcpStream::connect(socket)).await {
            Ok(Ok(stream)) => {
                run_connection(
                    stream,
                    socket,
                    true,
                    Some(record.device_id),
                    local,
                    connections,
                    events,
                    shutdown,
                )
                .await;
                return;
            }
            Ok(Err(error)) => {
                tracing::debug!(%socket, %error, "peer connection failed");
            }
            Err(_) => {
                tracing::debug!(%socket, "peer connection timed out");
            }
        }
    }
}

fn preferred_addresses(addresses: &[IpAddr]) -> Vec<IpAddr> {
    let mut scored: Vec<_> = addresses
        .iter()
        .copied()
        .map(|address| {
            let scope = match address {
                IpAddr::V4(value) if value.is_private() => 0,
                IpAddr::V6(value) if value.is_unique_local() => 1,
                IpAddr::V4(_) => 2,
                IpAddr::V6(_) => 3,
            };
            (address, scope, route_prefix_len(address))
        })
        .collect();
    scored.sort_by_key(|(address, scope, prefix)| (*scope, Reverse(*prefix), address.to_string()));
    scored.into_iter().map(|(address, _, _)| address).collect()
}

/// Returns how many leading address bits the peer shares with the local
/// interface Windows/macOS would route toward it. This makes a directly
/// attached LAN win over broad VPN ranges when both are advertised by mDNS.
fn route_prefix_len(remote: IpAddr) -> u32 {
    let bind_address = match remote {
        IpAddr::V4(_) => SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
        IpAddr::V6(_) => SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0),
    };
    let Ok(socket) = UdpSocket::bind(bind_address) else {
        return 0;
    };
    if socket.connect(SocketAddr::new(remote, 9)).is_err() {
        return 0;
    }
    socket
        .local_addr()
        .ok()
        .map(|local| shared_prefix_len(local.ip(), remote))
        .unwrap_or(0)
}

fn shared_prefix_len(left: IpAddr, right: IpAddr) -> u32 {
    match (left, right) {
        (IpAddr::V4(left), IpAddr::V4(right)) => {
            (u32::from(left) ^ u32::from(right)).leading_zeros()
        }
        (IpAddr::V6(left), IpAddr::V6(right)) => {
            (u128::from(left) ^ u128::from(right)).leading_zeros()
        }
        _ => 0,
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_connection(
    mut stream: TcpStream,
    address: SocketAddr,
    initiator: bool,
    expected_peer_id: Option<String>,
    local: LocalNode,
    connections: Arc<DashMap<String, ConnectionSlot>>,
    events: mpsc::UnboundedSender<NetworkEvent>,
    shutdown: watch::Receiver<bool>,
) {
    if let Err(error) = stream.set_nodelay(true) {
        let _ = events.send(NetworkEvent::Error(format!(
            "could not enable low-latency input transport for {address}: {error}"
        )));
        return;
    }
    tracing::debug!(%address, "enabled TCP_NODELAY for input transport");

    let handshake = if initiator {
        timeout(
            HANDSHAKE_TIMEOUT,
            perform_initiator_handshake(&mut stream, &local.private_key),
        )
        .await
    } else {
        timeout(
            HANDSHAKE_TIMEOUT,
            perform_responder_handshake(&mut stream, &local.private_key),
        )
        .await
    };
    let mut session = match handshake {
        Ok(Ok(session)) => session,
        Ok(Err(error)) => {
            let _ = events.send(NetworkEvent::Error(format!(
                "secure handshake with {address} failed: {error}"
            )));
            return;
        }
        Err(_) => {
            let _ = events.send(NetworkEvent::Error(format!(
                "secure handshake with {address} timed out"
            )));
            return;
        }
    };

    let hello_write = timeout(
        WRITE_TIMEOUT,
        write_wire_message(&mut session, &mut stream, &local.hello()),
    )
    .await;
    if let Err(error) = hello_write
        .map_err(|_| "write timed out".to_string())
        .and_then(|result| result.map_err(|error| error.to_string()))
    {
        let _ = events.send(NetworkEvent::Error(format!(
            "could not introduce this device to {address}: {error}"
        )));
        return;
    }
    let remote_hello = match timeout(
        HANDSHAKE_TIMEOUT,
        crate::crypto::read_wire_message(&mut session, &mut stream),
    )
    .await
    {
        Ok(Ok(WireMessage {
            message: Message::Hello(hello),
            ..
        })) => hello,
        Ok(Ok(_)) => {
            let _ = events.send(NetworkEvent::Error(format!(
                "peer {address} did not send hello first"
            )));
            return;
        }
        Ok(Err(error)) => {
            let _ = events.send(NetworkEvent::Error(format!(
                "could not read peer identity from {address}: {error}"
            )));
            return;
        }
        Err(_) => {
            let _ = events.send(NetworkEvent::Error(format!(
                "peer identity from {address} timed out"
            )));
            return;
        }
    };

    if remote_hello.device_id == local.device_id
        || expected_peer_id
            .as_ref()
            .is_some_and(|expected| expected != &remote_hello.device_id)
    {
        let _ = events.send(NetworkEvent::Error(format!(
            "peer identity mismatch from {address}"
        )));
        return;
    }

    let connection_id = Uuid::new_v4();
    let peer_id = remote_hello.device_id.clone();
    let (command_tx, command_rx) = mpsc::channel(OUTBOUND_QUEUE_CAPACITY);
    let (close_tx, close_rx) = watch::channel(None);
    let slot = ConnectionSlot {
        connection_id,
        sender: command_tx,
        close: close_tx,
    };
    if let Some(previous) = connections.insert(peer_id.clone(), slot) {
        let _ = previous
            .close
            .send(Some("replaced by a newer connection".into()));
    }

    let metadata = ConnectionMetadata {
        connection_id,
        device_id: peer_id.clone(),
        name: remote_hello.device_name,
        platform: remote_hello.platform,
        os_version: remote_hello.os_version,
        app_version: remote_hello.app_version,
        capabilities: remote_hello.capabilities,
        remote_static_key: *session.remote_static_key(),
        pairing_code: session.pairing_code().to_string(),
        address,
    };
    let _ = events.send(NetworkEvent::ConnectionReady(metadata));

    let reason = connection_loop(
        stream,
        session,
        connection_id,
        peer_id.clone(),
        command_rx,
        close_rx,
        events.clone(),
        shutdown,
    )
    .await;

    let remove = connections
        .get(&peer_id)
        .is_some_and(|current| current.connection_id == connection_id);
    if remove {
        connections.remove(&peer_id);
    }
    if remove {
        let _ = events.send(NetworkEvent::Disconnected {
            connection_id,
            device_id: peer_id,
            reason,
        });
    }
}

#[allow(clippy::too_many_arguments)]
async fn connection_loop(
    stream: TcpStream,
    mut session: NoiseSession,
    connection_id: Uuid,
    peer_id: String,
    mut commands: mpsc::Receiver<ConnectionCommand>,
    mut close: watch::Receiver<Option<String>>,
    events: mpsc::UnboundedSender<NetworkEvent>,
    mut shutdown: watch::Receiver<bool>,
) -> String {
    let (reader, mut writer) = stream.into_split();
    let (frames_tx, mut frames_rx) = mpsc::channel::<Result<Vec<u8>, String>>(32);
    let reader_task = tokio::spawn(read_frames(reader, frames_tx));
    let mut heartbeat = tokio::time::interval(HEARTBEAT_INTERVAL);
    heartbeat.set_missed_tick_behavior(MissedTickBehavior::Delay);

    let reason = loop {
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    break "service stopped".into();
                }
            }
            changed = close.changed() => {
                if changed.is_err() {
                    break "connection close channel dropped".into();
                }
                let requested_reason = close.borrow().clone();
                if let Some(requested_reason) = requested_reason {
                    let farewell = WireMessage::new(Message::Error(ErrorMessage {
                        code: "connection_closed".into(),
                        message: requested_reason.clone(),
                        fatal: true,
                    }));
                    let _ = send_encrypted(&mut session, &mut writer, &farewell).await;
                    break requested_reason;
                }
            }
            maybe_frame = frames_rx.recv() => match maybe_frame {
                Some(Ok(ciphertext)) => {
                    let message = match session.decode_wire(&ciphertext) {
                        Ok(message) => message,
                        Err(error) => break format!("invalid encrypted message: {error}"),
                    };
                    match message.message {
                        Message::Ping(Ping { nonce }) => {
                            let reply = WireMessage::new(Message::Pong(Pong { nonce }));
                            if let Err(error) = send_encrypted(&mut session, &mut writer, &reply).await {
                                break error;
                            }
                        }
                        _ => {
                            let _ = events.send(NetworkEvent::Message {
                                connection_id,
                                device_id: peer_id.clone(),
                                message,
                            });
                        }
                    }
                }
                Some(Err(error)) => break error,
                None => break "peer closed the connection".into(),
            },
            maybe_command = commands.recv() => match maybe_command {
                Some(ConnectionCommand::Send(message)) => {
                    if let Err(error) = send_encrypted(&mut session, &mut writer, &message).await {
                        break error;
                    }
                }
                None => break "connection command channel closed".into(),
            },
            _ = heartbeat.tick() => {
                let message = WireMessage::new(Message::Ping(Ping { nonce: unix_millis() }));
                if let Err(error) = send_encrypted(&mut session, &mut writer, &message).await {
                    break error;
                }
            }
        }
    };
    reader_task.abort();
    let _ = writer.shutdown().await;
    reason
}

async fn read_frames(mut reader: OwnedReadHalf, frames: mpsc::Sender<Result<Vec<u8>, String>>) {
    loop {
        let result = timeout(
            FRAME_TIMEOUT,
            read_length_prefixed(&mut reader, MAX_NOISE_FRAME_LEN),
        )
        .await;
        let frame = match result {
            Ok(Ok(frame)) => frame,
            Ok(Err(error)) => {
                let _ = frames.send(Err(error.to_string())).await;
                return;
            }
            Err(_) => {
                let _ = frames.send(Err("peer frame timed out".into())).await;
                return;
            }
        };
        if frames.send(Ok(frame)).await.is_err() {
            return;
        }
    }
}

async fn send_encrypted(
    session: &mut NoiseSession,
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    message: &WireMessage,
) -> Result<(), String> {
    let ciphertext = session
        .encode_wire(message)
        .map_err(|error| error.to_string())?;
    timeout(
        WRITE_TIMEOUT,
        write_length_prefixed(writer, &ciphertext, MAX_NOISE_FRAME_LEN),
    )
    .await
    .map_err(|_| "peer write timed out".to_string())?
    .map_err(|error| error.to_string())
}

fn unix_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::{preferred_addresses, shared_prefix_len};
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use tokio::net::{TcpListener, TcpStream};

    #[test]
    fn private_addresses_are_preferred() {
        let addresses = preferred_addresses(&[
            IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1)),
            IpAddr::V6(Ipv6Addr::LOCALHOST),
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 8)),
        ]);
        assert_eq!(addresses[0], IpAddr::V4(Ipv4Addr::new(192, 168, 1, 8)));
    }

    #[tokio::test]
    async fn tcp_nodelay_can_be_enabled_for_both_connection_ends() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        let client = TcpStream::connect(address);
        let server = listener.accept();
        let (client, server) = tokio::join!(client, server);
        let client = client.unwrap();
        let (server, _) = server.unwrap();

        client.set_nodelay(true).unwrap();
        server.set_nodelay(true).unwrap();

        assert!(client.nodelay().unwrap());
        assert!(server.nodelay().unwrap());
    }

    #[test]
    fn direct_subnet_has_more_route_affinity_than_a_vpn_range() {
        let local_lan = IpAddr::V4(Ipv4Addr::new(192, 168, 5, 38));
        let peer_lan = IpAddr::V4(Ipv4Addr::new(192, 168, 5, 37));
        let local_vpn = IpAddr::V4(Ipv4Addr::new(172, 28, 22, 120));
        let peer_vpn = IpAddr::V4(Ipv4Addr::new(172, 28, 76, 142));

        assert!(shared_prefix_len(local_lan, peer_lan) > shared_prefix_len(local_vpn, peer_vpn));
    }
}
