use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;

pub const SERVICE_TYPE: &str = "_inputmesh._tcp.local.";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryRecord {
    pub device_id: String,
    pub name: String,
    pub platform: String,
    pub version: String,
    pub port: u16,
    pub addresses: Vec<IpAddr>,
    pub fullname: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryEvent {
    Resolved(DiscoveryRecord),
    Removed { device_id: String },
    Error(String),
}

pub struct DiscoveryHandle {
    daemon: ServiceDaemon,
    fullname: String,
    stopped: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl DiscoveryHandle {
    pub fn start(
        device_id: &str,
        name: &str,
        platform: &str,
        version: &str,
        port: u16,
        events: mpsc::UnboundedSender<DiscoveryEvent>,
    ) -> Result<Self, String> {
        let daemon = ServiceDaemon::new().map_err(|error| error.to_string())?;
        let safe_host = dns_label(name);
        let host = format!("{safe_host}.local.");
        let instance_name = format!(
            "{safe_host}-{short_id}",
            short_id = &device_id[..device_id.len().min(8)]
        );
        let properties = HashMap::from([
            ("id".to_string(), device_id.to_string()),
            ("name".to_string(), name.to_string()),
            ("platform".to_string(), platform.to_string()),
            ("version".to_string(), version.to_string()),
            ("protocol".to_string(), "1".to_string()),
        ]);
        let info = ServiceInfo::new(SERVICE_TYPE, &instance_name, &host, "", port, properties)
            .map_err(|error| error.to_string())?
            .enable_addr_auto();
        let fullname = info.get_fullname().to_string();
        daemon.register(info).map_err(|error| error.to_string())?;

        let receiver = daemon
            .browse(SERVICE_TYPE)
            .map_err(|error| error.to_string())?;
        let stopped = Arc::new(AtomicBool::new(false));
        let worker_stopped = stopped.clone();
        let local_id = device_id.to_string();
        let worker = std::thread::Builder::new()
            .name("inputmesh-mdns".into())
            .spawn(move || {
                let mut service_ids: HashMap<String, String> = HashMap::new();
                while !worker_stopped.load(Ordering::Acquire) {
                    let event = match receiver.recv_timeout(Duration::from_millis(500)) {
                        Ok(event) => event,
                        Err(mdns_sd::RecvTimeoutError::Timeout) => continue,
                        Err(error) => {
                            let _ = events.send(DiscoveryEvent::Error(error.to_string()));
                            break;
                        }
                    };
                    match event {
                        ServiceEvent::ServiceResolved(info) => {
                            let Some(id) = info.get_property_val_str("id") else {
                                continue;
                            };
                            if id == local_id
                                || Uuid::parse_str(id).is_err()
                                || info.get_property_val_str("protocol") != Some("1")
                                || info.get_port() == 0
                            {
                                continue;
                            }
                            let mut addresses: Vec<IpAddr> = info
                                .get_addresses_v4()
                                .into_iter()
                                .filter(|address| {
                                    !address.is_loopback()
                                        && *address != Ipv4Addr::UNSPECIFIED
                                        && (address.is_private() || address.is_link_local())
                                })
                                .map(IpAddr::V4)
                                .collect();
                            addresses.sort_by_key(ToString::to_string);
                            let record = DiscoveryRecord {
                                device_id: id.to_string(),
                                name: limited_text(
                                    info.get_property_val_str("name")
                                        .unwrap_or("Nearby computer"),
                                    128,
                                ),
                                platform: limited_text(
                                    info.get_property_val_str("platform").unwrap_or("unknown"),
                                    16,
                                ),
                                version: limited_text(
                                    info.get_property_val_str("version").unwrap_or("unknown"),
                                    64,
                                ),
                                port: info.get_port(),
                                addresses,
                                fullname: info.get_fullname().to_string(),
                            };
                            service_ids.insert(record.fullname.clone(), record.device_id.clone());
                            let _ = events.send(DiscoveryEvent::Resolved(record));
                        }
                        ServiceEvent::ServiceRemoved(_, fullname) => {
                            if let Some(device_id) = service_ids.remove(&fullname) {
                                let _ = events.send(DiscoveryEvent::Removed { device_id });
                            }
                        }
                        _ => {}
                    }
                }
            })
            .map_err(|error| error.to_string())?;

        Ok(Self {
            daemon,
            fullname,
            stopped,
            worker: Some(worker),
        })
    }
}

impl Drop for DiscoveryHandle {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        let _ = self.daemon.stop_browse(SERVICE_TYPE);
        let _ = self.daemon.unregister(&self.fullname);
        let _ = self.daemon.shutdown();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn dns_label(value: &str) -> String {
    let mut output: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    output = output.trim_matches('-').to_string();
    if output.is_empty() {
        output.push_str("inputmesh");
    }
    output.truncate(48);
    output
}

fn limited_text(value: &str, maximum_chars: usize) -> String {
    value.chars().take(maximum_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::dns_label;

    #[test]
    fn creates_a_safe_dns_label() {
        assert_eq!(dns_label("Henry's Mac mini"), "henry-s-mac-mini");
        assert_eq!(dns_label("中文电脑"), "inputmesh");
    }
}
