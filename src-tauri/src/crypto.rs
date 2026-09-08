//! Noise XX handshake and bounded framed transport for InputMesh.

use std::{error::Error, fmt, io};

use snow::{Builder, HandshakeState, TransportState, params::NoiseParams};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::protocol::{ProtocolError, WireMessage};

pub const NOISE_PATTERN: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";
pub const NOISE_PROLOGUE: &[u8] = b"InputMesh/v1";
pub const NOISE_KEY_LEN: usize = 32;
pub const NOISE_HASH_LEN: usize = 32;
pub const NOISE_TAG_LEN: usize = 16;
pub const MAX_NOISE_FRAME_LEN: usize = 65_535;
pub const MAX_NOISE_PLAINTEXT_LEN: usize = MAX_NOISE_FRAME_LEN - NOISE_TAG_LEN;

#[derive(Clone, PartialEq, Eq)]
pub struct NoiseStaticKeypair {
    pub private: [u8; NOISE_KEY_LEN],
    pub public: [u8; NOISE_KEY_LEN],
}

impl fmt::Debug for NoiseStaticKeypair {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NoiseStaticKeypair")
            .field("private", &"[REDACTED]")
            .field("public", &self.public)
            .finish()
    }
}

/// Authenticated peer details and the directional Noise transport state.
pub struct NoiseSession {
    transport: TransportState,
    handshake_hash: [u8; NOISE_HASH_LEN],
    remote_static_key: [u8; NOISE_KEY_LEN],
    pairing_code: String,
}

impl fmt::Debug for NoiseSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NoiseSession")
            .field("handshake_hash", &self.handshake_hash)
            .field("remote_static_key", &self.remote_static_key)
            .field("pairing_code", &self.pairing_code)
            .finish_non_exhaustive()
    }
}

impl NoiseSession {
    #[must_use]
    pub fn handshake_hash(&self) -> &[u8; NOISE_HASH_LEN] {
        &self.handshake_hash
    }

    #[must_use]
    pub fn remote_static_key(&self) -> &[u8; NOISE_KEY_LEN] {
        &self.remote_static_key
    }

    #[must_use]
    pub fn pairing_code(&self) -> &str {
        &self.pairing_code
    }

    #[must_use]
    pub fn transport_mut(&mut self) -> &mut TransportState {
        &mut self.transport
    }

    #[must_use]
    pub fn into_transport(self) -> TransportState {
        self.transport
    }

    /// Serializes and encrypts without doing I/O. This lets a writer task hold
    /// the session lock only while advancing the Noise send nonce.
    pub fn encode_wire(&mut self, message: &WireMessage) -> Result<Vec<u8>, CryptoError> {
        let plaintext = message.to_json_vec()?;
        encrypt_transport_message(&mut self.transport, &plaintext)
    }

    /// Authenticates, decrypts, and parses a frame that was read separately.
    /// A read loop can await the socket before briefly acquiring the session.
    pub fn decode_wire(&mut self, ciphertext: &[u8]) -> Result<WireMessage, CryptoError> {
        let plaintext = decrypt_transport_message(&mut self.transport, ciphertext)?;
        WireMessage::from_json_slice(&plaintext).map_err(CryptoError::Protocol)
    }
}

pub fn generate_static_keypair() -> Result<NoiseStaticKeypair, CryptoError> {
    let parameters = noise_parameters()?;
    let keypair = Builder::new(parameters).generate_keypair()?;
    let private =
        keypair
            .private
            .as_slice()
            .try_into()
            .map_err(|_| CryptoError::InvalidKeyLength {
                kind: "generated private",
                length: keypair.private.len(),
            })?;
    let public =
        keypair
            .public
            .as_slice()
            .try_into()
            .map_err(|_| CryptoError::InvalidKeyLength {
                kind: "generated public",
                length: keypair.public.len(),
            })?;
    Ok(NoiseStaticKeypair { private, public })
}

/// Runs the three-message XX initiator flow over a u32 big-endian framed stream.
pub async fn perform_initiator_handshake<S>(
    stream: &mut S,
    local_private_key: &[u8; NOISE_KEY_LEN],
) -> Result<NoiseSession, CryptoError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut handshake = build_handshake(local_private_key, true)?;

    write_handshake_message(stream, &mut handshake).await?;
    read_handshake_message(stream, &mut handshake).await?;
    write_handshake_message(stream, &mut handshake).await?;

    finish_handshake(handshake)
}

/// Runs the three-message XX responder flow over a u32 big-endian framed stream.
pub async fn perform_responder_handshake<S>(
    stream: &mut S,
    local_private_key: &[u8; NOISE_KEY_LEN],
) -> Result<NoiseSession, CryptoError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut handshake = build_handshake(local_private_key, false)?;

    read_handshake_message(stream, &mut handshake).await?;
    write_handshake_message(stream, &mut handshake).await?;
    read_handshake_message(stream, &mut handshake).await?;

    finish_handshake(handshake)
}

/// Turns the handshake hash into a short authentication string. Reduction uses
/// every hash byte, retaining essentially uniform distribution over 1,000,000 values.
#[must_use]
pub fn pairing_code_from_handshake_hash(hash: &[u8; NOISE_HASH_LEN]) -> String {
    let value = hash.iter().fold(0_u32, |value, byte| {
        ((u64::from(value) * 256 + u64::from(*byte)) % 1_000_000) as u32
    });
    format!("{value:06}")
}

/// Writes one u32 big-endian length-prefixed frame after checking the caller's limit.
pub async fn write_length_prefixed<W>(
    writer: &mut W,
    payload: &[u8],
    maximum: usize,
) -> Result<(), CryptoError>
where
    W: AsyncWrite + Unpin,
{
    if payload.len() > maximum || payload.len() > u32::MAX as usize {
        return Err(CryptoError::FrameTooLarge {
            length: payload.len(),
            maximum: maximum.min(u32::MAX as usize),
        });
    }

    let length = payload.len() as u32;
    writer.write_all(&length.to_be_bytes()).await?;
    writer.write_all(payload).await?;
    writer.flush().await?;
    Ok(())
}

/// Reads one bounded u32 big-endian length-prefixed frame. The bound is checked
/// before allocation, so an attacker cannot force an oversized allocation.
pub async fn read_length_prefixed<R>(reader: &mut R, maximum: usize) -> Result<Vec<u8>, CryptoError>
where
    R: AsyncRead + Unpin,
{
    let mut encoded_length = [0_u8; 4];
    reader.read_exact(&mut encoded_length).await?;
    let length = u32::from_be_bytes(encoded_length) as usize;
    if length > maximum {
        return Err(CryptoError::FrameTooLarge { length, maximum });
    }

    let mut payload = vec![0_u8; length];
    reader.read_exact(&mut payload).await?;
    Ok(payload)
}

pub async fn write_encrypted_frame<W>(
    session: &mut NoiseSession,
    writer: &mut W,
    plaintext: &[u8],
) -> Result<(), CryptoError>
where
    W: AsyncWrite + Unpin,
{
    let ciphertext = encrypt_transport_message(&mut session.transport, plaintext)?;
    write_length_prefixed(writer, &ciphertext, MAX_NOISE_FRAME_LEN).await
}

pub async fn read_encrypted_frame<R>(
    session: &mut NoiseSession,
    reader: &mut R,
) -> Result<Vec<u8>, CryptoError>
where
    R: AsyncRead + Unpin,
{
    let ciphertext = read_length_prefixed(reader, MAX_NOISE_FRAME_LEN).await?;
    decrypt_transport_message(&mut session.transport, &ciphertext)
}

pub async fn write_wire_message<W>(
    session: &mut NoiseSession,
    writer: &mut W,
    message: &WireMessage,
) -> Result<(), CryptoError>
where
    W: AsyncWrite + Unpin,
{
    let ciphertext = session.encode_wire(message)?;
    write_length_prefixed(writer, &ciphertext, MAX_NOISE_FRAME_LEN).await
}

pub async fn read_wire_message<R>(
    session: &mut NoiseSession,
    reader: &mut R,
) -> Result<WireMessage, CryptoError>
where
    R: AsyncRead + Unpin,
{
    let ciphertext = read_length_prefixed(reader, MAX_NOISE_FRAME_LEN).await?;
    session.decode_wire(&ciphertext)
}

fn noise_parameters() -> Result<NoiseParams, CryptoError> {
    NOISE_PATTERN.parse().map_err(CryptoError::Noise)
}

fn build_handshake(
    local_private_key: &[u8; NOISE_KEY_LEN],
    initiator: bool,
) -> Result<HandshakeState, CryptoError> {
    let builder = Builder::new(noise_parameters()?)
        .local_private_key(local_private_key)?
        .prologue(NOISE_PROLOGUE)?;
    if initiator {
        builder.build_initiator().map_err(CryptoError::Noise)
    } else {
        builder.build_responder().map_err(CryptoError::Noise)
    }
}

async fn write_handshake_message<W>(
    writer: &mut W,
    handshake: &mut HandshakeState,
) -> Result<(), CryptoError>
where
    W: AsyncWrite + Unpin,
{
    let mut message = vec![0_u8; MAX_NOISE_FRAME_LEN];
    let length = handshake.write_message(&[], &mut message)?;
    write_length_prefixed(writer, &message[..length], MAX_NOISE_FRAME_LEN).await
}

async fn read_handshake_message<R>(
    reader: &mut R,
    handshake: &mut HandshakeState,
) -> Result<(), CryptoError>
where
    R: AsyncRead + Unpin,
{
    let message = read_length_prefixed(reader, MAX_NOISE_FRAME_LEN).await?;
    let mut payload = vec![0_u8; message.len()];
    let payload_length = handshake.read_message(&message, &mut payload)?;
    if payload_length != 0 {
        return Err(CryptoError::UnexpectedHandshakePayload(payload_length));
    }
    Ok(())
}

fn finish_handshake(handshake: HandshakeState) -> Result<NoiseSession, CryptoError> {
    if !handshake.is_handshake_finished() {
        return Err(CryptoError::HandshakeIncomplete);
    }

    let handshake_hash: [u8; NOISE_HASH_LEN] =
        handshake.get_handshake_hash().try_into().map_err(|_| {
            CryptoError::InvalidHandshakeHashLength(handshake.get_handshake_hash().len())
        })?;
    let remote = handshake
        .get_remote_static()
        .ok_or(CryptoError::MissingRemoteStatic)?;
    let remote_static_key = remote
        .try_into()
        .map_err(|_| CryptoError::InvalidKeyLength {
            kind: "remote public",
            length: remote.len(),
        })?;
    let pairing_code = pairing_code_from_handshake_hash(&handshake_hash);
    let transport = handshake.into_transport_mode()?;

    Ok(NoiseSession {
        transport,
        handshake_hash,
        remote_static_key,
        pairing_code,
    })
}

fn encrypt_transport_message(
    transport: &mut TransportState,
    plaintext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    if plaintext.len() > MAX_NOISE_PLAINTEXT_LEN {
        return Err(CryptoError::PlaintextTooLarge {
            length: plaintext.len(),
            maximum: MAX_NOISE_PLAINTEXT_LEN,
        });
    }
    let mut ciphertext = vec![0_u8; plaintext.len() + NOISE_TAG_LEN];
    let length = transport.write_message(plaintext, &mut ciphertext)?;
    ciphertext.truncate(length);
    Ok(ciphertext)
}

fn decrypt_transport_message(
    transport: &mut TransportState,
    ciphertext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    if ciphertext.len() < NOISE_TAG_LEN {
        return Err(CryptoError::FrameTooShort {
            length: ciphertext.len(),
            minimum: NOISE_TAG_LEN,
        });
    }
    if ciphertext.len() > MAX_NOISE_FRAME_LEN {
        return Err(CryptoError::FrameTooLarge {
            length: ciphertext.len(),
            maximum: MAX_NOISE_FRAME_LEN,
        });
    }
    let mut plaintext = vec![0_u8; ciphertext.len() - NOISE_TAG_LEN];
    let length = transport.read_message(ciphertext, &mut plaintext)?;
    plaintext.truncate(length);
    Ok(plaintext)
}

#[derive(Debug)]
pub enum CryptoError {
    Io(io::Error),
    Noise(snow::Error),
    Protocol(ProtocolError),
    FrameTooLarge { length: usize, maximum: usize },
    FrameTooShort { length: usize, minimum: usize },
    PlaintextTooLarge { length: usize, maximum: usize },
    InvalidKeyLength { kind: &'static str, length: usize },
    InvalidHandshakeHashLength(usize),
    UnexpectedHandshakePayload(usize),
    MissingRemoteStatic,
    HandshakeIncomplete,
}

impl fmt::Display for CryptoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "transport I/O failed: {error}"),
            Self::Noise(_) => formatter.write_str("Noise operation failed"),
            Self::Protocol(error) => write!(formatter, "protocol operation failed: {error}"),
            Self::FrameTooLarge { length, maximum } => {
                write!(formatter, "frame is {length} bytes; maximum is {maximum}")
            }
            Self::FrameTooShort { length, minimum } => {
                write!(
                    formatter,
                    "encrypted frame is {length} bytes; minimum is {minimum}"
                )
            }
            Self::PlaintextTooLarge { length, maximum } => {
                write!(
                    formatter,
                    "plaintext is {length} bytes; maximum is {maximum}"
                )
            }
            Self::InvalidKeyLength { kind, length } => {
                write!(formatter, "{kind} key has invalid length {length}")
            }
            Self::InvalidHandshakeHashLength(length) => {
                write!(formatter, "handshake hash has invalid length {length}")
            }
            Self::UnexpectedHandshakePayload(length) => {
                write!(
                    formatter,
                    "handshake contained an unexpected {length}-byte payload"
                )
            }
            Self::MissingRemoteStatic => {
                formatter.write_str("Noise XX handshake did not reveal a remote static key")
            }
            Self::HandshakeIncomplete => formatter.write_str("Noise handshake is incomplete"),
        }
    }
}

impl Error for CryptoError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Noise(error) => Some(error),
            Self::Protocol(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for CryptoError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<snow::Error> for CryptoError {
    fn from(error: snow::Error) -> Self {
        Self::Noise(error)
    }
}

impl From<ProtocolError> for CryptoError {
    fn from(error: ProtocolError) -> Self {
        Self::Protocol(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{Message, Ping, Pong};
    use tokio::io::{AsyncWriteExt, duplex};

    async fn connected_pair() -> (
        tokio::io::DuplexStream,
        NoiseSession,
        NoiseStaticKeypair,
        tokio::io::DuplexStream,
        NoiseSession,
        NoiseStaticKeypair,
    ) {
        let initiator_keys = generate_static_keypair().unwrap();
        let responder_keys = generate_static_keypair().unwrap();
        let (mut initiator_io, mut responder_io) = duplex(256 * 1024);

        let (initiator_session, responder_session) = tokio::join!(
            perform_initiator_handshake(&mut initiator_io, &initiator_keys.private),
            perform_responder_handshake(&mut responder_io, &responder_keys.private),
        );

        (
            initiator_io,
            initiator_session.unwrap(),
            initiator_keys,
            responder_io,
            responder_session.unwrap(),
            responder_keys,
        )
    }

    #[tokio::test]
    async fn duplex_handshake_and_bidirectional_encrypted_messages_succeed() {
        let (
            mut initiator_io,
            mut initiator_session,
            initiator_keys,
            mut responder_io,
            mut responder_session,
            responder_keys,
        ) = connected_pair().await;

        assert_eq!(
            initiator_session.handshake_hash(),
            responder_session.handshake_hash()
        );
        assert_eq!(
            initiator_session.pairing_code(),
            responder_session.pairing_code()
        );
        assert_eq!(initiator_session.pairing_code().len(), 6);
        assert!(
            initiator_session
                .pairing_code()
                .bytes()
                .all(|byte| byte.is_ascii_digit())
        );
        assert_eq!(
            initiator_session.remote_static_key(),
            &responder_keys.public
        );
        assert_eq!(
            responder_session.remote_static_key(),
            &initiator_keys.public
        );

        let direct_ping = WireMessage::new(Message::Ping(Ping { nonce: 41 }));
        let ciphertext = initiator_session.encode_wire(&direct_ping).unwrap();
        assert_ne!(ciphertext, direct_ping.to_json_vec().unwrap());
        assert_eq!(
            responder_session.decode_wire(&ciphertext).unwrap(),
            direct_ping
        );

        let ping = WireMessage::new(Message::Ping(Ping { nonce: 42 }));
        write_wire_message(&mut initiator_session, &mut initiator_io, &ping)
            .await
            .unwrap();
        assert_eq!(
            read_wire_message(&mut responder_session, &mut responder_io)
                .await
                .unwrap(),
            ping
        );

        let pong = WireMessage::new(Message::Pong(Pong { nonce: 42 }));
        write_wire_message(&mut responder_session, &mut responder_io, &pong)
            .await
            .unwrap();
        assert_eq!(
            read_wire_message(&mut initiator_session, &mut initiator_io)
                .await
                .unwrap(),
            pong
        );
    }

    #[tokio::test]
    async fn rejects_oversized_and_truncated_length_prefixed_frames() {
        let (mut writer, mut reader) = duplex(64);
        let bad_length = (MAX_NOISE_FRAME_LEN as u32 + 1).to_be_bytes();
        writer.write_all(&bad_length).await.unwrap();
        let error = read_length_prefixed(&mut reader, MAX_NOISE_FRAME_LEN)
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            CryptoError::FrameTooLarge {
                length,
                maximum: MAX_NOISE_FRAME_LEN
            } if length == MAX_NOISE_FRAME_LEN + 1
        ));

        let oversized = vec![0_u8; MAX_NOISE_FRAME_LEN + 1];
        let error = write_length_prefixed(&mut writer, &oversized, MAX_NOISE_FRAME_LEN)
            .await
            .unwrap_err();
        assert!(matches!(error, CryptoError::FrameTooLarge { .. }));

        let (mut writer, mut reader) = duplex(64);
        writer.write_all(&8_u32.to_be_bytes()).await.unwrap();
        writer.write_all(b"cut").await.unwrap();
        drop(writer);
        let error = read_length_prefixed(&mut reader, MAX_NOISE_FRAME_LEN)
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            CryptoError::Io(error) if error.kind() == io::ErrorKind::UnexpectedEof
        ));
    }

    #[tokio::test]
    async fn rejects_a_tampered_encrypted_frame() {
        let (
            mut initiator_io,
            mut initiator_session,
            _,
            mut responder_io,
            mut responder_session,
            _,
        ) = connected_pair().await;

        let mut ciphertext =
            encrypt_transport_message(&mut initiator_session.transport, b"authenticated").unwrap();
        let last = ciphertext.len() - 1;
        ciphertext[last] ^= 0x80;
        write_length_prefixed(&mut initiator_io, &ciphertext, MAX_NOISE_FRAME_LEN)
            .await
            .unwrap();

        let error = read_encrypted_frame(&mut responder_session, &mut responder_io)
            .await
            .unwrap_err();
        assert!(matches!(error, CryptoError::Noise(snow::Error::Decrypt)));
    }
}
