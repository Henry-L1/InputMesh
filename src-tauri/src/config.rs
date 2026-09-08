//! Persistent InputMesh configuration.
//!
//! The configuration contains the local Noise static private key and must be
//! treated as a secret. [`AppConfig::save_atomic`] writes through a temporary
//! file in the destination directory, synchronizes it, and then renames it over
//! the destination. On Unix, newly created and final files are restricted to
//! mode `0600` (subject only to the process umask making them stricter).

use std::{
    collections::HashSet,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use thiserror::Error;
#[cfg(test)]
use uuid::Uuid;

use crate::model::{DeviceId, SharingSettings};

pub const CURRENT_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_LISTEN_PORT: u16 = 42_424;

/// A Noise protocol static private or public key.
pub type NoiseStaticKey = [u8; 32];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustedPeerConfig {
    pub device_id: DeviceId,
    pub name: String,
    #[serde(with = "hex_key")]
    pub noise_static_public_key: NoiseStaticKey,
    /// User-facing fingerprint captured when pairing was confirmed.
    pub fingerprint: String,
    /// Unix timestamp in milliseconds.
    pub trusted_at: u64,
    /// Unix timestamp in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_known_address: Option<String>,
}

/// Persisted position and enabled state for one screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenPlacement {
    pub screen_id: String,
    pub owner_device_id: DeviceId,
    pub x: i32,
    pub y: i32,
    pub enabled: bool,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    #[serde(default = "current_schema_version")]
    pub schema_version: u32,
    pub device_id: DeviceId,
    /// Serialized as 64 lowercase hexadecimal characters rather than a JSON
    /// byte array. `Debug` intentionally redacts this field.
    #[serde(with = "hex_key")]
    pub noise_static_private_key: NoiseStaticKey,
    #[serde(default)]
    pub trusted_peers: Vec<TrustedPeerConfig>,
    #[serde(default)]
    pub settings: SharingSettings,
    #[serde(default)]
    pub screen_layout: Vec<ScreenPlacement>,
    #[serde(default)]
    pub sharing_enabled: bool,
    #[serde(default = "default_listen_port")]
    pub listen_port: u16,
}

impl std::fmt::Debug for AppConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AppConfig")
            .field("schema_version", &self.schema_version)
            .field("device_id", &self.device_id)
            .field("noise_static_private_key", &"[REDACTED]")
            .field("trusted_peers", &self.trusted_peers)
            .field("settings", &self.settings)
            .field("screen_layout", &self.screen_layout)
            .field("sharing_enabled", &self.sharing_enabled)
            .field("listen_port", &self.listen_port)
            .finish()
    }
}

impl AppConfig {
    pub fn new(device_id: DeviceId, noise_static_private_key: NoiseStaticKey) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            device_id,
            noise_static_private_key,
            trusted_peers: Vec::new(),
            settings: SharingSettings::default(),
            screen_layout: Vec::new(),
            sharing_enabled: false,
            listen_port: DEFAULT_LISTEN_PORT,
        }
    }

    /// Validates security-sensitive invariants and uniqueness constraints.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(ConfigError::UnsupportedSchema {
                found: self.schema_version,
                supported: CURRENT_SCHEMA_VERSION,
            });
        }
        if self.device_id.is_nil() {
            return Err(ConfigError::InvalidConfig(
                "deviceId must not be the nil UUID".into(),
            ));
        }
        if self.noise_static_private_key.iter().all(|byte| *byte == 0) {
            return Err(ConfigError::InvalidConfig(
                "noiseStaticPrivateKey must not be all zeroes".into(),
            ));
        }
        if self.listen_port == 0 {
            return Err(ConfigError::InvalidConfig(
                "listenPort must be between 1 and 65535".into(),
            ));
        }

        let mut peer_ids = HashSet::with_capacity(self.trusted_peers.len());
        let mut peer_keys = HashSet::with_capacity(self.trusted_peers.len());
        for peer in &self.trusted_peers {
            if peer.device_id.is_nil() {
                return Err(ConfigError::InvalidConfig(
                    "trusted peer deviceId must not be the nil UUID".into(),
                ));
            }
            if peer.device_id == self.device_id {
                return Err(ConfigError::InvalidConfig(
                    "the local device cannot be listed as a trusted peer".into(),
                ));
            }
            if !peer_ids.insert(peer.device_id) {
                return Err(ConfigError::InvalidConfig(format!(
                    "duplicate trusted peer deviceId {}",
                    peer.device_id
                )));
            }
            if peer.noise_static_public_key.iter().all(|byte| *byte == 0) {
                return Err(ConfigError::InvalidConfig(format!(
                    "trusted peer {} has an all-zero Noise public key",
                    peer.device_id
                )));
            }
            if !peer_keys.insert(peer.noise_static_public_key) {
                return Err(ConfigError::InvalidConfig(
                    "two trusted peers have the same Noise public key".into(),
                ));
            }
            if peer.name.trim().is_empty() {
                return Err(ConfigError::InvalidConfig(format!(
                    "trusted peer {} has an empty name",
                    peer.device_id
                )));
            }
            if peer.fingerprint.trim().is_empty() {
                return Err(ConfigError::InvalidConfig(format!(
                    "trusted peer {} has an empty fingerprint",
                    peer.device_id
                )));
            }
        }

        let mut screen_ids = HashSet::with_capacity(self.screen_layout.len());
        for screen in &self.screen_layout {
            if screen.screen_id.trim().is_empty() {
                return Err(ConfigError::InvalidConfig(
                    "screenLayout contains an empty screenId".into(),
                ));
            }
            if screen.owner_device_id.is_nil() {
                return Err(ConfigError::InvalidConfig(format!(
                    "screen {} has a nil ownerDeviceId",
                    screen.screen_id
                )));
            }
            if !screen_ids.insert(screen.screen_id.as_str()) {
                return Err(ConfigError::InvalidConfig(format!(
                    "duplicate screenId {}",
                    screen.screen_id
                )));
            }
        }

        Ok(())
    }

    pub fn trusted_peer(&self, device_id: DeviceId) -> Option<&TrustedPeerConfig> {
        self.trusted_peers
            .iter()
            .find(|peer| peer.device_id == device_id)
    }

    /// Inserts or replaces a trusted peer by device UUID.
    pub fn upsert_trusted_peer(&mut self, peer: TrustedPeerConfig) {
        if let Some(existing) = self
            .trusted_peers
            .iter_mut()
            .find(|existing| existing.device_id == peer.device_id)
        {
            *existing = peer;
        } else {
            self.trusted_peers.push(peer);
        }
    }

    pub fn remove_trusted_peer(&mut self, device_id: DeviceId) -> bool {
        let original_len = self.trusted_peers.len();
        self.trusted_peers
            .retain(|peer| peer.device_id != device_id);
        original_len != self.trusted_peers.len()
    }

    /// Loads and validates a configuration file. On Unix, this also makes a
    /// best-effort attempt to tighten an overly broad existing mode to `0600`.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let mut file = File::open(path).map_err(|source| ConfigError::Io {
            operation: "open",
            path: path.to_path_buf(),
            source,
        })?;

        // Failure to chmod an existing readable file should not make the
        // application forget its identity. Newly written files are always
        // created privately by save_atomic.
        let _ = set_private_permissions(path);

        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|source| ConfigError::Io {
                operation: "read",
                path: path.to_path_buf(),
                source,
            })?;
        let config: Self = serde_json::from_slice(&bytes).map_err(|source| ConfigError::Json {
            path: path.to_path_buf(),
            source,
        })?;
        config.validate()?;
        Ok(config)
    }

    /// Loads a configuration, or creates and atomically saves one when the
    /// file does not exist. Other I/O and parsing errors are preserved.
    pub fn load_or_create_with(
        path: impl AsRef<Path>,
        create: impl FnOnce() -> Self,
    ) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        match Self::load(path) {
            Ok(config) => Ok(config),
            Err(error) if error.io_kind() == Some(io::ErrorKind::NotFound) => {
                let config = create();
                config.save_atomic(path)?;
                Ok(config)
            }
            Err(error) => Err(error),
        }
    }

    /// Atomically persists the configuration.
    ///
    /// The temporary file lives next to the destination so the final rename
    /// stays on one filesystem. If serialization, validation, writing, syncing
    /// or renaming fails, the prior destination is left intact.
    pub fn save_atomic(&self, path: impl AsRef<Path>) -> Result<(), ConfigError> {
        self.validate()?;
        let path = path.as_ref();
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let file_name = path.file_name().ok_or_else(|| {
            ConfigError::InvalidConfig(format!(
                "configuration path {} has no file name",
                path.display()
            ))
        })?;

        fs::create_dir_all(parent).map_err(|source| ConfigError::Io {
            operation: "create parent directory",
            path: parent.to_path_buf(),
            source,
        })?;

        let mut bytes = serde_json::to_vec_pretty(self).map_err(|source| ConfigError::Json {
            path: path.to_path_buf(),
            source,
        })?;
        bytes.push(b'\n');

        let (temporary_path, mut temporary_file) = create_secure_temporary(parent, file_name)?;
        let save_result = (|| {
            temporary_file
                .write_all(&bytes)
                .map_err(|source| ConfigError::Io {
                    operation: "write temporary file",
                    path: temporary_path.clone(),
                    source,
                })?;
            temporary_file
                .sync_all()
                .map_err(|source| ConfigError::Io {
                    operation: "sync temporary file",
                    path: temporary_path.clone(),
                    source,
                })?;
            drop(temporary_file);

            fs::rename(&temporary_path, path).map_err(|source| ConfigError::Io {
                operation: "rename temporary file",
                path: path.to_path_buf(),
                source,
            })?;

            // Creation mode is already 0600 on Unix. Applying it once more to
            // the final name protects against unusual platform rename behavior.
            let _ = set_private_permissions(path);
            sync_directory_best_effort(parent);
            Ok(())
        })();

        if save_result.is_err() {
            let _ = fs::remove_file(&temporary_path);
        }
        save_result
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("unsupported configuration schema {found}; this build supports {supported}")]
    UnsupportedSchema { found: u32, supported: u32 },
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("could not {operation} {path}: {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("invalid JSON configuration at {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

impl ConfigError {
    pub fn io_kind(&self) -> Option<io::ErrorKind> {
        match self {
            Self::Io { source, .. } => Some(source.kind()),
            _ => None,
        }
    }
}

const fn current_schema_version() -> u32 {
    CURRENT_SCHEMA_VERSION
}

const fn default_listen_port() -> u16 {
    DEFAULT_LISTEN_PORT
}

static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn create_secure_temporary(
    parent: &Path,
    destination_name: &std::ffi::OsStr,
) -> Result<(PathBuf, File), ConfigError> {
    for _ in 0..64 {
        let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let mut temporary_name = OsString::from(".");
        temporary_name.push(destination_name);
        temporary_name.push(format!(".tmp-{}-{sequence}", std::process::id()));
        let temporary_path = parent.join(temporary_name);

        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }

        match options.open(&temporary_path) {
            Ok(file) => return Ok((temporary_path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(source) => {
                return Err(ConfigError::Io {
                    operation: "create temporary file",
                    path: temporary_path,
                    source,
                });
            }
        }
    }

    Err(ConfigError::Io {
        operation: "create a unique temporary file",
        path: parent.to_path_buf(),
        source: io::Error::new(
            io::ErrorKind::AlreadyExists,
            "temporary file name collision limit reached",
        ),
    })
}

#[cfg(unix)]
fn set_private_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn set_private_permissions(_path: &Path) -> io::Result<()> {
    Ok(())
}

fn sync_directory_best_effort(path: &Path) {
    // Directory fsync makes the rename durable on Unix filesystems. Some
    // supported platforms reject opening/syncing directories, hence best effort.
    if let Ok(directory) = File::open(path) {
        let _ = directory.sync_all();
    }
}

mod hex_key {
    use super::*;
    use std::fmt::Write as _;

    pub fn serialize<S>(key: &NoiseStaticKey, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut encoded = String::with_capacity(64);
        for byte in key {
            // Writing into a String is infallible.
            write!(&mut encoded, "{byte:02x}").expect("formatting a byte into String cannot fail");
        }
        serializer.serialize_str(&encoded)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NoiseStaticKey, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded = String::deserialize(deserializer)?;
        decode(&encoded).map_err(de::Error::custom)
    }

    fn decode(encoded: &str) -> Result<NoiseStaticKey, &'static str> {
        if encoded.len() != 64 {
            return Err("Noise key must contain exactly 64 hexadecimal characters");
        }

        let bytes = encoded.as_bytes();
        let mut decoded = [0_u8; 32];
        for (index, output) in decoded.iter_mut().enumerate() {
            let high =
                nibble(bytes[index * 2]).ok_or("Noise key contains a non-hexadecimal character")?;
            let low = nibble(bytes[index * 2 + 1])
                .ok_or("Noise key contains a non-hexadecimal character")?;
            *output = (high << 4) | low;
        }
        Ok(decoded)
    }

    const fn nibble(byte: u8) -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            b'A'..=b'F' => Some(byte - b'A' + 10),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use tempfile::tempdir;

    fn local_id() -> Uuid {
        Uuid::parse_str("11111111-2222-4333-8444-555555555555").unwrap()
    }

    fn peer_id() -> Uuid {
        Uuid::parse_str("aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee").unwrap()
    }

    fn valid_config() -> AppConfig {
        let mut config = AppConfig::new(local_id(), [0x11; 32]);
        config.sharing_enabled = true;
        config.trusted_peers.push(TrustedPeerConfig {
            device_id: peer_id(),
            name: "Windows workstation".into(),
            noise_static_public_key: [0x22; 32],
            fingerprint: "ABCD EFGH".into(),
            trusted_at: 1_725_000_000_000,
            last_seen_at: Some(1_725_000_100_000),
            last_known_address: Some("192.0.2.20:24800".into()),
        });
        config.screen_layout.push(ScreenPlacement {
            screen_id: "peer:display-1".into(),
            owner_device_id: peer_id(),
            x: 2560,
            y: -180,
            enabled: true,
        });
        config
    }

    #[test]
    fn config_uses_camel_case_and_hex_encoded_keys() {
        let value = serde_json::to_value(valid_config()).unwrap();
        assert_eq!(value["schemaVersion"], CURRENT_SCHEMA_VERSION);
        assert_eq!(value["deviceId"], local_id().to_string());
        assert_eq!(
            value["noiseStaticPrivateKey"],
            "1111111111111111111111111111111111111111111111111111111111111111"
        );
        assert_eq!(value["trustedPeers"][0]["deviceId"], peer_id().to_string());
        assert_eq!(
            value["trustedPeers"][0]["noiseStaticPublicKey"],
            "2222222222222222222222222222222222222222222222222222222222222222"
        );
        assert_eq!(value["screenLayout"][0]["screenId"], "peer:display-1");
        assert_eq!(value["settings"]["switchDelayMs"], 150);
        assert!(value.get("noise_static_private_key").is_none());
    }

    #[test]
    fn key_decoder_accepts_uppercase_but_rejects_wrong_length_and_characters() {
        let mut value = serde_json::to_value(valid_config()).unwrap();
        value["noiseStaticPrivateKey"] = Value::String(
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".into(),
        );
        let decoded: AppConfig = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(decoded.noise_static_private_key, [0xaa; 32]);

        value["noiseStaticPrivateKey"] = Value::String("aa".into());
        assert!(serde_json::from_value::<AppConfig>(value.clone()).is_err());

        value["noiseStaticPrivateKey"] = Value::String("z".repeat(64));
        assert!(serde_json::from_value::<AppConfig>(value).is_err());
    }

    #[test]
    fn save_and_load_round_trip_and_leave_no_temporary_file() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("nested/inputmesh.json");
        let config = valid_config();

        config.save_atomic(&path).unwrap();
        assert_eq!(AppConfig::load(&path).unwrap(), config);

        let entries: Vec<_> = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(entries, vec![OsString::from("inputmesh.json")]);
    }

    #[test]
    fn a_second_atomic_save_replaces_the_complete_document() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("inputmesh.json");
        let first = valid_config();
        first.save_atomic(&path).unwrap();

        let mut second = first.clone();
        second.sharing_enabled = false;
        second.settings.edge_resistance_px = 27;
        second.trusted_peers.clear();
        second.save_atomic(&path).unwrap();

        assert_eq!(AppConfig::load(&path).unwrap(), second);
        let raw = fs::read_to_string(path).unwrap();
        assert!(!raw.contains("Windows workstation"));
    }

    #[test]
    fn validation_failure_does_not_replace_an_existing_file() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("inputmesh.json");
        let original = valid_config();
        original.save_atomic(&path).unwrap();

        let mut invalid = original.clone();
        invalid.noise_static_private_key = [0; 32];
        assert!(matches!(
            invalid.save_atomic(&path),
            Err(ConfigError::InvalidConfig(_))
        ));
        assert_eq!(AppConfig::load(&path).unwrap(), original);
    }

    #[test]
    fn load_or_create_only_creates_for_a_missing_file() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("inputmesh.json");
        let expected = valid_config();
        let loaded = AppConfig::load_or_create_with(&path, || expected.clone()).unwrap();
        assert_eq!(loaded, expected);

        let loaded_again = AppConfig::load_or_create_with(&path, || {
            panic!("creator must not run when the file exists")
        })
        .unwrap();
        assert_eq!(loaded_again, expected);

        fs::write(&path, b"not json").unwrap();
        assert!(matches!(
            AppConfig::load_or_create_with(&path, valid_config),
            Err(ConfigError::Json { .. })
        ));
    }

    #[test]
    fn validates_duplicate_and_security_sensitive_values() {
        let mut config = valid_config();
        config.trusted_peers.push(config.trusted_peers[0].clone());
        assert!(matches!(
            config.validate(),
            Err(ConfigError::InvalidConfig(_))
        ));

        let mut config = valid_config();
        config.screen_layout.push(config.screen_layout[0].clone());
        assert!(matches!(
            config.validate(),
            Err(ConfigError::InvalidConfig(_))
        ));

        let mut config = valid_config();
        config.device_id = Uuid::nil();
        assert!(matches!(
            config.validate(),
            Err(ConfigError::InvalidConfig(_))
        ));

        let mut config = valid_config();
        config.schema_version = CURRENT_SCHEMA_VERSION + 1;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::UnsupportedSchema { .. })
        ));
    }

    #[test]
    fn upsert_and_remove_trusted_peer_use_the_device_uuid() {
        let mut config = valid_config();
        let mut replacement = config.trusted_peers[0].clone();
        replacement.name = "Renamed PC".into();
        config.upsert_trusted_peer(replacement);
        assert_eq!(config.trusted_peers.len(), 1);
        assert_eq!(config.trusted_peer(peer_id()).unwrap().name, "Renamed PC");
        assert!(config.remove_trusted_peer(peer_id()));
        assert!(!config.remove_trusted_peer(peer_id()));
    }

    #[test]
    fn debug_output_redacts_the_private_key() {
        let rendered = format!("{:?}", valid_config());
        assert!(rendered.contains("[REDACTED]"));
        assert!(
            !rendered.contains("1111111111111111111111111111111111111111111111111111111111111111")
        );
    }

    #[cfg(unix)]
    #[test]
    fn saved_file_is_owner_read_write_only() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempdir().unwrap();
        let path = directory.path().join("inputmesh.json");
        valid_config().save_atomic(&path).unwrap();
        let mode = fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}
