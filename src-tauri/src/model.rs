//! Serializable application models shared by the Tauri command layer, the
//! runtime and the TypeScript UI.
//!
//! Rust field names deliberately follow the usual `snake_case` convention.
//! `serde(rename_all = "camelCase")` keeps the wire representation aligned with
//! `src/types.ts` without leaking TypeScript naming conventions into Rust code.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Stable identifier of an InputMesh device.
pub type DeviceId = Uuid;

/// Stable identifier of a screen. Screen identifiers are platform supplied (or
/// derived from a platform supplied identifier), so they are not assumed to be
/// UUIDs.
pub type ScreenId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OsKind {
    Macos,
    Windows,
    Linux,
    Unknown,
}

impl OsKind {
    /// Returns the OS selected for the current compilation target.
    pub const fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::Macos
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else {
            Self::Unknown
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ServiceStatus {
    #[default]
    Stopped,
    Starting,
    Running,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PeerStatus {
    Discovered,
    Pairing,
    Connected,
    Offline,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub id: DeviceId,
    pub name: String,
    pub os: OsKind,
    /// Human-readable operating-system version.
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenInfo {
    /// Globally unique identifier used by InputMesh.
    pub id: ScreenId,
    /// Identifier reported by the owning operating system.
    pub native_id: String,
    pub owner_device_id: DeviceId,
    pub owner_name: String,
    pub name: String,
    /// Native pixel width and height. Zero-sized screens are rejected by the
    /// topology module.
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    /// User-configurable position in the InputMesh topology canvas.
    pub x: i32,
    pub y: i32,
    pub primary: bool,
    pub enabled: bool,
    pub online: bool,
}

impl ScreenInfo {
    pub fn is_available(&self) -> bool {
        self.enabled && self.online && self.width > 0 && self.height > 0
    }

    /// Tests a point in topology (global canvas) coordinates. Rectangles are
    /// half-open, matching pixel coordinate ranges: `[x, x + width)`.
    pub fn contains_global_point(&self, x: f64, y: f64) -> bool {
        if !x.is_finite() || !y.is_finite() || self.width == 0 || self.height == 0 {
            return false;
        }

        let left = f64::from(self.x);
        let top = f64::from(self.y);
        x >= left
            && x < left + f64::from(self.width)
            && y >= top
            && y < top + f64::from(self.height)
    }
}

/// Peer fields intentionally mirror the flattened `PeerInfo extends DeviceInfo`
/// TypeScript interface. No nested `device` object appears on the wire.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerInfo {
    pub id: DeviceId,
    pub name: String,
    pub os: OsKind,
    pub version: String,
    pub status: PeerStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pairing_code: Option<String>,
    /// Unix timestamp in milliseconds.
    pub last_seen_at: u64,
    #[serde(default)]
    pub screens: Vec<ScreenInfo>,
}

impl PeerInfo {
    pub fn device_info(&self) -> DeviceInfo {
        DeviceInfo {
            id: self.id,
            name: self.name.clone(),
            os: self.os,
            version: self.version.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PermissionState {
    pub accessibility: bool,
    pub input_monitoring: bool,
    pub requires_action: bool,
    pub help_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SharingSettings {
    pub switch_delay_ms: u32,
    pub edge_resistance_px: u32,
    pub take_control_on_local_input: bool,
    pub launch_at_login: bool,
}

impl Default for SharingSettings {
    fn default() -> Self {
        Self {
            switch_delay_ms: 150,
            edge_resistance_px: 12,
            take_control_on_local_input: true,
            launch_at_login: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityLog {
    pub id: String,
    /// Unix timestamp in milliseconds.
    pub timestamp: u64,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub local_device: DeviceInfo,
    pub service_status: ServiceStatus,
    pub sharing_enabled: bool,
    pub active_controller_id: DeviceId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_screen_id: Option<ScreenId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub listen_port: Option<u16>,
    pub permission: PermissionState,
    #[serde(default)]
    pub screens: Vec<ScreenInfo>,
    #[serde(default)]
    pub peers: Vec<PeerInfo>,
    pub settings: SharingSettings,
    #[serde(default)]
    pub logs: Vec<ActivityLog>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn device_id() -> Uuid {
        Uuid::parse_str("12345678-1234-5678-9abc-def012345678").unwrap()
    }

    fn screen() -> ScreenInfo {
        ScreenInfo {
            id: "12345678-1234-5678-9abc-def012345678:display-1".into(),
            native_id: "display-1".into(),
            owner_device_id: device_id(),
            owner_name: "Studio Mac".into(),
            name: "Built-in Display".into(),
            width: 2560,
            height: 1600,
            scale_factor: 2.0,
            x: -2560,
            y: 120,
            primary: true,
            enabled: true,
            online: true,
        }
    }

    #[test]
    fn models_serialize_with_the_frontend_contract() {
        let local_device = DeviceInfo {
            id: device_id(),
            name: "Studio Mac".into(),
            os: OsKind::Macos,
            version: "15.6".into(),
        };
        let snapshot = AppSnapshot {
            local_device: local_device.clone(),
            service_status: ServiceStatus::Running,
            sharing_enabled: true,
            active_controller_id: device_id(),
            active_screen_id: Some(screen().id.clone()),
            listen_port: Some(24_800),
            permission: PermissionState {
                accessibility: true,
                input_monitoring: false,
                requires_action: true,
                help_text: "Allow Input Monitoring".into(),
            },
            screens: vec![screen()],
            peers: vec![PeerInfo {
                id: Uuid::parse_str("aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee").unwrap(),
                name: "Gaming PC".into(),
                os: OsKind::Windows,
                version: "11".into(),
                status: PeerStatus::Connected,
                address: Some("192.0.2.10:24800".into()),
                latency_ms: Some(3),
                fingerprint: Some("AB:CD".into()),
                pairing_code: None,
                last_seen_at: 1_725_000_000_000,
                screens: Vec::new(),
            }],
            settings: SharingSettings::default(),
            logs: vec![ActivityLog {
                id: "log-1".into(),
                timestamp: 1_725_000_000_000,
                level: LogLevel::Warning,
                message: "Permission required".into(),
            }],
        };

        let value = serde_json::to_value(snapshot).unwrap();
        assert_eq!(value["localDevice"]["os"], "macos");
        assert_eq!(value["serviceStatus"], "running");
        assert_eq!(value["activeControllerId"], device_id().to_string());
        assert_eq!(value["listenPort"], 24_800);
        assert_eq!(value["permission"]["inputMonitoring"], false);
        assert_eq!(value["screens"][0]["nativeId"], "display-1");
        assert_eq!(
            value["screens"][0]["ownerDeviceId"],
            device_id().to_string()
        );
        assert_eq!(value["peers"][0]["status"], "connected");
        assert_eq!(value["peers"][0]["latencyMs"], 3);
        assert!(value["peers"][0].get("pairingCode").is_none());
        assert_eq!(value["settings"]["switchDelayMs"], 150);
        assert_eq!(value["logs"][0]["level"], "warning");

        let object = value.as_object().unwrap();
        for expected in [
            "localDevice",
            "serviceStatus",
            "sharingEnabled",
            "activeControllerId",
            "activeScreenId",
            "listenPort",
            "permission",
            "screens",
            "peers",
            "settings",
            "logs",
        ] {
            assert!(object.contains_key(expected), "missing key {expected}");
        }
        assert!(!object.contains_key("local_device"));
    }

    #[test]
    fn absent_optional_snapshot_and_peer_fields_are_omitted() {
        let value = serde_json::to_value(AppSnapshot {
            local_device: DeviceInfo {
                id: device_id(),
                name: "Mac".into(),
                os: OsKind::Macos,
                version: "15".into(),
            },
            service_status: ServiceStatus::Stopped,
            sharing_enabled: false,
            active_controller_id: device_id(),
            active_screen_id: None,
            listen_port: None,
            permission: PermissionState::default(),
            screens: Vec::new(),
            peers: vec![PeerInfo {
                id: device_id(),
                name: "Peer".into(),
                os: OsKind::Windows,
                version: "11".into(),
                status: PeerStatus::Offline,
                address: None,
                latency_ms: None,
                fingerprint: None,
                pairing_code: None,
                last_seen_at: 0,
                screens: Vec::new(),
            }],
            settings: SharingSettings::default(),
            logs: Vec::new(),
        })
        .unwrap();

        assert!(value.get("activeScreenId").is_none());
        assert!(value.get("listenPort").is_none());
        let peer = value["peers"][0].as_object().unwrap();
        for optional in ["address", "latencyMs", "fingerprint", "pairingCode"] {
            assert!(!peer.contains_key(optional));
        }
    }

    #[test]
    fn enums_round_trip_all_frontend_values() {
        let os_values = json!(["macos", "windows", "linux", "unknown"]);
        let parsed: Vec<OsKind> = serde_json::from_value(os_values.clone()).unwrap();
        assert_eq!(
            parsed,
            vec![
                OsKind::Macos,
                OsKind::Windows,
                OsKind::Linux,
                OsKind::Unknown
            ]
        );
        assert_eq!(serde_json::to_value(parsed).unwrap(), os_values);

        let statuses: Value = json!(["discovered", "pairing", "connected", "offline", "rejected"]);
        let parsed: Vec<PeerStatus> = serde_json::from_value(statuses.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), statuses);
    }

    #[test]
    fn screen_hit_test_uses_half_open_bounds_and_rejects_non_finite_points() {
        let screen = screen();
        assert!(screen.contains_global_point(-2560.0, 120.0));
        assert!(screen.contains_global_point(-0.001, 1719.999));
        assert!(!screen.contains_global_point(0.0, 200.0));
        assert!(!screen.contains_global_point(-1.0, 1720.0));
        assert!(!screen.contains_global_point(f64::NAN, 500.0));
    }
}
