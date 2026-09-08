//! Versioned JSON messages exchanged inside an authenticated Noise transport.

use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u16 = 1;
pub const MAX_WIRE_MESSAGE_LEN: usize = 60 * 1024;
pub const MAX_SCREENS_PER_DEVICE: usize = 64;
pub const MAX_LAYOUT_PLACEMENTS: usize = 128;

/// A complete application message. The `type` discriminator and its `payload`
/// are flattened next to `protocolVersion` on the wire.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WireMessage {
    pub protocol_version: u16,
    #[serde(flatten)]
    pub message: Message,
}

impl WireMessage {
    #[must_use]
    pub fn new(message: Message) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            message,
        }
    }

    pub fn to_json_vec(&self) -> Result<Vec<u8>, ProtocolError> {
        self.validate()?;
        let encoded = serde_json::to_vec(self).map_err(ProtocolError::Json)?;
        if encoded.len() > MAX_WIRE_MESSAGE_LEN {
            return Err(ProtocolError::MessageTooLarge {
                length: encoded.len(),
                maximum: MAX_WIRE_MESSAGE_LEN,
            });
        }
        Ok(encoded)
    }

    pub fn from_json_slice(encoded: &[u8]) -> Result<Self, ProtocolError> {
        if encoded.len() > MAX_WIRE_MESSAGE_LEN {
            return Err(ProtocolError::MessageTooLarge {
                length: encoded.len(),
                maximum: MAX_WIRE_MESSAGE_LEN,
            });
        }
        let message: Self = serde_json::from_slice(encoded).map_err(ProtocolError::Json)?;
        message.validate()?;
        Ok(message)
    }

    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.protocol_version != PROTOCOL_VERSION {
            return Err(ProtocolError::UnsupportedVersion {
                received: self.protocol_version,
                supported: PROTOCOL_VERSION,
            });
        }

        match &self.message {
            Message::Screens(payload) if payload.screens.len() > MAX_SCREENS_PER_DEVICE => {
                Err(ProtocolError::TooManyScreens {
                    count: payload.screens.len(),
                    maximum: MAX_SCREENS_PER_DEVICE,
                })
            }
            Message::Layout(payload) if payload.placements.len() > MAX_LAYOUT_PLACEMENTS => {
                Err(ProtocolError::TooManyScreens {
                    count: payload.placements.len(),
                    maximum: MAX_LAYOUT_PLACEMENTS,
                })
            }
            Message::Focus(payload) => {
                validate_normalized(payload.normalized_x, "focus.normalizedX")?;
                validate_normalized(payload.normalized_y, "focus.normalizedY")
            }
            Message::Input(payload) => payload.event.validate(),
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum Message {
    Hello(Hello),
    Screens(Screens),
    Layout(Layout),
    PairRequest(PairRequest),
    PairApproval(PairApproval),
    PairReject(PairReject),
    ControlClaim(ControlClaim),
    ControlLease(ControlLease),
    ControlRelease(ControlRelease),
    Focus(Focus),
    Input(Input),
    Ping(Ping),
    Pong(Pong),
    Error(ErrorMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Hello {
    pub device_id: String,
    pub device_name: String,
    pub platform: Platform,
    #[serde(default)]
    pub os_version: String,
    pub app_version: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Platform {
    Windows,
    Macos,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Screens {
    /// Increases whenever this device's screen set or local arrangement changes.
    pub revision: u64,
    pub screens: Vec<ScreenDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScreenDescriptor {
    pub screen_id: String,
    pub name: String,
    pub origin_x: i32,
    pub origin_y: i32,
    pub width_px: u32,
    pub height_px: u32,
    pub scale_factor: f64,
    pub primary: bool,
    pub enabled: bool,
}

/// User-defined positions for every known screen. This is deliberately
/// separate from `Screens`, whose origins describe a device's native display
/// arrangement rather than the shared InputMesh canvas.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    pub revision: u64,
    pub source_device_id: String,
    pub placements: Vec<LayoutPlacement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LayoutPlacement {
    pub screen_id: String,
    pub owner_device_id: String,
    pub x: i32,
    pub y: i32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PairRequest {
    pub request_id: String,
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PairApproval {
    pub request_id: String,
    pub peer_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PairReject {
    pub request_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ControlClaim {
    pub claim_id: String,
    pub controller_device_id: String,
    /// Identifies the physical keyboard/mouse set currently active locally.
    pub input_set_id: String,
    pub requested_lease_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ControlLease {
    pub claim_id: String,
    pub lease_id: String,
    pub holder_device_id: String,
    pub input_set_id: String,
    /// Relative lifetime avoids depending on synchronized wall clocks.
    pub ttl_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ControlRelease {
    pub lease_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Focus {
    pub lease_id: String,
    pub screen_id: String,
    pub normalized_x: f64,
    pub normalized_y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Input {
    pub lease_id: String,
    /// Monotonically increasing within a lease; stale or duplicate input is dropped.
    pub sequence: u64,
    pub event: InputEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum InputEvent {
    PointerMoveNormalized(PointerMoveNormalized),
    PointerMoveRelative(PointerMoveRelative),
    Button(ButtonInput),
    Wheel(WheelInput),
    Key(KeyInput),
}

impl InputEvent {
    fn validate(&self) -> Result<(), ProtocolError> {
        match self {
            Self::PointerMoveNormalized(event) => {
                validate_normalized(event.x, "input.pointerMoveNormalized.x")?;
                validate_normalized(event.y, "input.pointerMoveNormalized.y")
            }
            Self::Wheel(event) if !event.delta_x.is_finite() || !event.delta_y.is_finite() => Err(
                ProtocolError::InvalidMessage("wheel deltas must be finite".into()),
            ),
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PointerMoveNormalized {
    pub screen_id: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PointerMoveRelative {
    pub delta_x: i32,
    pub delta_y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ButtonInput {
    pub button: PointerButton,
    pub state: ButtonState,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PointerButton {
    Left,
    Middle,
    Right,
    Back,
    Forward,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ButtonState {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WheelInput {
    pub delta_x: f64,
    pub delta_y: f64,
    pub unit: WheelUnit,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WheelUnit {
    Pixels,
    Lines,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KeyInput {
    pub code: CanonicalKeyCode,
    pub state: KeyState,
    #[serde(default)]
    pub repeat: bool,
    #[serde(default)]
    pub modifiers: KeyModifiers,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum KeyState {
    Pressed,
    Released,
}

/// Windows virtual-key/scancode is preferred for the default keyboard layout;
/// USB HID usage remains available as a platform-neutral canonical form.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "namespace", content = "code", rename_all = "camelCase")]
pub enum CanonicalKeyCode {
    Windows(WindowsKeyCode),
    Hid(HidKeyCode),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WindowsKeyCode {
    pub virtual_key: u16,
    pub scan_code: u16,
    pub extended: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HidKeyCode {
    pub usage_page: u16,
    pub usage: u16,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KeyModifiers {
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub control: bool,
    #[serde(default)]
    pub alt: bool,
    #[serde(default)]
    pub meta: bool,
    #[serde(default)]
    pub caps_lock: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Ping {
    pub nonce: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Pong {
    pub nonce: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ErrorMessage {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub fatal: bool,
}

fn validate_normalized(value: f64, field: &'static str) -> Result<(), ProtocolError> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(ProtocolError::InvalidMessage(format!(
            "{field} must be between 0 and 1"
        )))
    }
}

#[derive(Debug)]
pub enum ProtocolError {
    Json(serde_json::Error),
    UnsupportedVersion { received: u16, supported: u16 },
    MessageTooLarge { length: usize, maximum: usize },
    TooManyScreens { count: usize, maximum: usize },
    InvalidMessage(String),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(formatter, "invalid protocol JSON: {error}"),
            Self::UnsupportedVersion {
                received,
                supported,
            } => write!(
                formatter,
                "unsupported protocol version {received}; this build supports {supported}"
            ),
            Self::MessageTooLarge { length, maximum } => {
                write!(
                    formatter,
                    "protocol message is {length} bytes; maximum is {maximum}"
                )
            }
            Self::TooManyScreens { count, maximum } => {
                write!(
                    formatter,
                    "screen message contains {count} screens; maximum is {maximum}"
                )
            }
            Self::InvalidMessage(reason) => write!(formatter, "invalid protocol message: {reason}"),
        }
    }
}

impl Error for ProtocolError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_round_trip_is_versioned_and_camel_case() {
        let message = WireMessage::new(Message::Input(Input {
            lease_id: "lease-1".into(),
            sequence: 7,
            event: InputEvent::Key(KeyInput {
                code: CanonicalKeyCode::Windows(WindowsKeyCode {
                    virtual_key: 0x41,
                    scan_code: 0x1e,
                    extended: false,
                }),
                state: KeyState::Pressed,
                repeat: false,
                modifiers: KeyModifiers {
                    shift: true,
                    ..KeyModifiers::default()
                },
            }),
        }));

        let encoded = message.to_json_vec().unwrap();
        let json = std::str::from_utf8(&encoded).unwrap();
        assert!(json.contains("\"protocolVersion\":1"));
        assert!(json.contains("\"type\":\"input\""));
        assert!(json.contains("\"virtualKey\":65"));
        assert_eq!(WireMessage::from_json_slice(&encoded).unwrap(), message);
    }

    #[test]
    fn rejects_an_unknown_protocol_version() {
        let encoded = br#"{"protocolVersion":2,"type":"ping","payload":{"nonce":1}}"#;
        assert!(matches!(
            WireMessage::from_json_slice(encoded),
            Err(ProtocolError::UnsupportedVersion {
                received: 2,
                supported: PROTOCOL_VERSION
            })
        ));
    }

    #[test]
    fn rejects_out_of_bounds_normalized_coordinates() {
        let message = WireMessage::new(Message::Focus(Focus {
            lease_id: "lease-1".into(),
            screen_id: "screen-1".into(),
            normalized_x: 1.01,
            normalized_y: 0.5,
        }));
        assert!(matches!(
            message.to_json_vec(),
            Err(ProtocolError::InvalidMessage(_))
        ));
    }

    #[test]
    fn layout_round_trip_keeps_shared_canvas_coordinates() {
        let message = WireMessage::new(Message::Layout(Layout {
            revision: 42,
            source_device_id: "00000000-0000-0000-0000-000000000001".into(),
            placements: vec![LayoutPlacement {
                screen_id: "display-1".into(),
                owner_device_id: "00000000-0000-0000-0000-000000000001".into(),
                x: 3024,
                y: 0,
                enabled: true,
            }],
        }));
        let encoded = message.to_json_vec().unwrap();
        assert_eq!(WireMessage::from_json_slice(&encoded).unwrap(), message);
    }
}
