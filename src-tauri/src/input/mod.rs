use std::{
    collections::{HashSet, VecDeque},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

const EXPECTED_EVENT_TTL: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum NativeInputEvent {
    PointerMoved {
        x: f64,
        y: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        delta_x: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        delta_y: Option<f64>,
    },
    PointerButton {
        button: String,
        pressed: bool,
    },
    Wheel {
        delta_x: i64,
        delta_y: i64,
    },
    Key {
        code: String,
        pressed: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformPermission {
    pub accessibility: bool,
    pub input_monitoring: bool,
    pub requires_action: bool,
    pub help_text: String,
}

pub struct InputBridge {
    enabled: AtomicBool,
    installed: AtomicBool,
    #[cfg(target_os = "macos")]
    cursor_hidden: AtomicBool,
    injected_events: Mutex<VecDeque<(Instant, NativeInputEvent)>>,
}

impl InputBridge {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            enabled: AtomicBool::new(false),
            installed: AtomicBool::new(false),
            #[cfg(target_os = "macos")]
            cursor_hidden: AtomicBool::new(false),
            injected_events: Mutex::new(VecDeque::new()),
        })
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Release);
    }

    /// Hides the controller's local macOS cursor while its logical pointer is
    /// on a remote screen. The atomic guard keeps CoreGraphics' hide/show
    /// counter balanced across repeated focus messages. The caller dispatches
    /// this method to the macOS main thread after the event-tap callback.
    #[cfg(target_os = "macos")]
    pub fn set_cursor_hidden(&self, hidden: bool) {
        let previous = self.cursor_hidden.swap(hidden, Ordering::AcqRel);
        if previous == hidden {
            return;
        }
        let result = unsafe {
            if hidden {
                CGDisplayHideCursor(CGMainDisplayID())
            } else {
                CGDisplayShowCursor(CGMainDisplayID())
            }
        };
        if result != 0 {
            self.cursor_hidden.store(previous, Ordering::Release);
            tracing::warn!(hidden, result, "could not update local cursor visibility");
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn set_cursor_hidden(&self, _hidden: bool) {}

    /// Installs the operating-system input tap once. The handler returns true
    /// only while an event must be withheld from the local desktop.
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    pub fn install(
        self: &Arc<Self>,
        handler: Arc<dyn Fn(NativeInputEvent) -> bool + Send + Sync + 'static>,
        emergency_handler: Arc<dyn Fn() + Send + Sync + 'static>,
    ) -> Result<(), String> {
        if self.installed.swap(true, Ordering::AcqRel) {
            return Ok(());
        }
        if query_permission().requires_action {
            self.installed.store(false, Ordering::Release);
            return Err("系统尚未授予辅助功能/输入监控权限".into());
        }

        let bridge = self.clone();
        let (started_tx, started_rx) = std::sync::mpsc::sync_channel::<Result<(), String>>(1);
        std::thread::Builder::new()
            .name("inputmesh-input-tap".into())
            .spawn(move || {
                let pressed_keys = Mutex::new(HashSet::new());
                let startup_signal = Mutex::new(Some(started_tx));
                let callback_bridge = bridge.clone();
                let result = rdev::grab(move |event| {
                    if let Some(signal) = startup_signal.lock().take() {
                        let _ = signal.send(Ok(()));
                    }
                    if event.is_synthetic {
                        return Some(event);
                    }
                    let Some(native) = from_rdev(&event) else {
                        return Some(event);
                    };

                    if callback_bridge.consume_injected(&native) {
                        return Some(event);
                    }

                    let mut pressed = pressed_keys.lock();
                    update_pressed_keys(&native, &mut pressed);
                    if is_emergency_release(&native, &pressed) {
                        callback_bridge.enabled.store(false, Ordering::Release);
                        emergency_handler();
                        return Some(event);
                    }
                    drop(pressed);

                    let suppress =
                        callback_bridge.enabled.load(Ordering::Acquire) && handler(native);
                    if suppress { None } else { Some(event) }
                });
                if let Err(error) = result {
                    bridge.installed.store(false, Ordering::Release);
                    tracing::error!(?error, "global input tap stopped");
                }
            })
            .map_err(|error| {
                self.installed.store(false, Ordering::Release);
                error.to_string()
            })?;

        match started_rx.recv_timeout(Duration::from_millis(350)) {
            Ok(result) => result,
            // A working rdev grab blocks before its first event; no early error
            // is therefore also a successful installation.
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    pub fn install(
        self: &Arc<Self>,
        _handler: Arc<dyn Fn(NativeInputEvent) -> bool + Send + Sync + 'static>,
        _emergency_handler: Arc<dyn Fn() + Send + Sync + 'static>,
    ) -> Result<(), String> {
        Err("InputMesh currently supports input capture on macOS and Windows".into())
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    pub fn inject(&self, event: NativeInputEvent) -> Result<(), String> {
        let Some(rdev_event) = to_rdev(&event) else {
            return Err("unsupported input event".into());
        };
        {
            let mut expected = self.injected_events.lock();
            prune_expected(&mut expected);
            expected.push_back((Instant::now(), event));
        }
        rdev::simulate(&rdev_event).map_err(|error| format!("input injection failed: {error:?}"))
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    pub fn inject(&self, _event: NativeInputEvent) -> Result<(), String> {
        Err("InputMesh currently supports input injection on macOS and Windows".into())
    }

    fn consume_injected(&self, event: &NativeInputEvent) -> bool {
        let mut expected = self.injected_events.lock();
        prune_expected(&mut expected);
        if let Some(index) = expected
            .iter()
            .position(|(_, candidate)| events_match(candidate, event))
        {
            expected.remove(index);
            true
        } else {
            false
        }
    }
}

fn prune_expected(expected: &mut VecDeque<(Instant, NativeInputEvent)>) {
    while expected
        .front()
        .is_some_and(|(created_at, _)| created_at.elapsed() > EXPECTED_EVENT_TTL)
    {
        expected.pop_front();
    }
}

fn events_match(left: &NativeInputEvent, right: &NativeInputEvent) -> bool {
    match (left, right) {
        (
            NativeInputEvent::PointerMoved {
                x: left_x,
                y: left_y,
                ..
            },
            NativeInputEvent::PointerMoved {
                x: right_x,
                y: right_y,
                ..
            },
        ) => (left_x - right_x).abs() < 1.0 && (left_y - right_y).abs() < 1.0,
        _ => left == right,
    }
}

fn update_pressed_keys(event: &NativeInputEvent, pressed: &mut HashSet<String>) {
    if let NativeInputEvent::Key {
        code,
        pressed: down,
    } = event
    {
        if *down {
            pressed.insert(code.clone());
        } else {
            pressed.remove(code);
        }
    }
}

fn is_emergency_release(event: &NativeInputEvent, pressed: &HashSet<String>) -> bool {
    matches!(event, NativeInputEvent::Key { code, pressed: true } if code == "Escape")
        && pressed.iter().any(|key| key.starts_with("Control"))
        && pressed
            .iter()
            .any(|key| key.starts_with("Alt") || key == "Option")
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn from_rdev(event: &rdev::Event) -> Option<NativeInputEvent> {
    use rdev::EventType;
    match &event.event_type {
        EventType::MouseMove { x, y } => Some(NativeInputEvent::PointerMoved {
            x: *x,
            y: *y,
            delta_x: event.relative_delta.map(|(x, _)| x as f64),
            delta_y: event.relative_delta.map(|(_, y)| y as f64),
        }),
        EventType::ButtonPress(button) => Some(NativeInputEvent::PointerButton {
            button: enum_name(button)?,
            pressed: true,
        }),
        EventType::ButtonRelease(button) => Some(NativeInputEvent::PointerButton {
            button: enum_name(button)?,
            pressed: false,
        }),
        EventType::Wheel { delta_x, delta_y } => Some(NativeInputEvent::Wheel {
            delta_x: *delta_x,
            delta_y: *delta_y,
        }),
        EventType::KeyPress(key) => Some(NativeInputEvent::Key {
            code: enum_name(key)?,
            pressed: true,
        }),
        EventType::KeyRelease(key) => Some(NativeInputEvent::Key {
            code: enum_name(key)?,
            pressed: false,
        }),
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn to_rdev(event: &NativeInputEvent) -> Option<rdev::EventType> {
    use rdev::EventType;
    match event {
        NativeInputEvent::PointerMoved { x, y, .. } => Some(EventType::MouseMove { x: *x, y: *y }),
        NativeInputEvent::PointerButton { button, pressed } => {
            let button = enum_from_name(button)?;
            Some(if *pressed {
                EventType::ButtonPress(button)
            } else {
                EventType::ButtonRelease(button)
            })
        }
        NativeInputEvent::Wheel { delta_x, delta_y } => Some(EventType::Wheel {
            delta_x: *delta_x,
            delta_y: *delta_y,
        }),
        NativeInputEvent::Key { code, pressed } => {
            let key = enum_from_name(code)?;
            Some(if *pressed {
                EventType::KeyPress(key)
            } else {
                EventType::KeyRelease(key)
            })
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn enum_name<T: Serialize>(value: &T) -> Option<String> {
    serde_json::to_value(value)
        .ok()?
        .as_str()
        .map(ToOwned::to_owned)
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn enum_from_name<T: for<'de> Deserialize<'de>>(value: &str) -> Option<T> {
    serde_json::from_value(serde_json::Value::String(value.to_string())).ok()
}

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn CGPreflightListenEventAccess() -> bool;
    fn CGRequestListenEventAccess() -> bool;
    fn CGMainDisplayID() -> u32;
    fn CGDisplayHideCursor(display: u32) -> i32;
    fn CGDisplayShowCursor(display: u32) -> i32;
}

pub fn query_permission() -> PlatformPermission {
    #[cfg(target_os = "macos")]
    {
        // SAFETY: These parameterless CoreGraphics functions are stable system APIs.
        let accessibility = unsafe { AXIsProcessTrusted() };
        // SAFETY: This is a read-only permission preflight call.
        let input_monitoring = unsafe { CGPreflightListenEventAccess() };
        PlatformPermission {
            accessibility,
            input_monitoring,
            requires_action: !(accessibility && input_monitoring),
            help_text: if accessibility && input_monitoring {
                "键鼠权限已就绪".into()
            } else {
                "请在“隐私与安全性”中允许 InputMesh 的辅助功能与输入监控权限".into()
            },
        }
    }

    #[cfg(target_os = "windows")]
    {
        PlatformPermission {
            accessibility: true,
            input_monitoring: true,
            requires_action: false,
            help_text: "键鼠权限已就绪；控制管理员窗口时需以管理员身份运行".into(),
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        PlatformPermission {
            accessibility: false,
            input_monitoring: false,
            requires_action: true,
            help_text: "当前平台尚未实现系统级键鼠权限".into(),
        }
    }
}

pub fn open_permission_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        // This call may show the OS Input Monitoring consent prompt. It is only
        // reached after the user presses the permission button in the app.
        // SAFETY: Parameterless documented CoreGraphics permission request.
        let _ = unsafe { CGRequestListenEventAccess() };
        std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn()
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("当前平台尚未提供权限设置入口".into())
    }
}

/// Converts rdev's physical key names to USB HID keyboard usages. Using the
/// physical usage keeps shortcuts stable across different input languages and
/// lets Windows apply its normal keyboard layout on the receiving side.
pub fn key_name_to_hid(name: &str) -> Option<u16> {
    if let Some(letter) = name.strip_prefix("Key") {
        let byte = letter.as_bytes();
        if byte.len() == 1 && byte[0].is_ascii_uppercase() {
            return Some(0x04 + u16::from(byte[0] - b'A'));
        }
    }
    if let Some(number) = name.strip_prefix("Num") {
        let index = number.parse::<u16>().ok()?;
        return match index {
            1..=9 => Some(0x1e + index - 1),
            0 => Some(0x27),
            _ => None,
        };
    }
    if let Some(function) = name.strip_prefix('F') {
        if let Ok(index) = function.parse::<u16>() {
            if (1..=12).contains(&index) {
                return Some(0x3a + index - 1);
            }
        }
    }
    if let Some(number) = name.strip_prefix("Kp") {
        if let Ok(index) = number.parse::<u16>() {
            return match index {
                1..=9 => Some(0x59 + index - 1),
                0 => Some(0x62),
                _ => None,
            };
        }
    }
    Some(match name {
        "Return" => 0x28,
        "Escape" => 0x29,
        "Backspace" => 0x2a,
        "Tab" => 0x2b,
        "Space" => 0x2c,
        "Minus" => 0x2d,
        "Equal" => 0x2e,
        "LeftBracket" => 0x2f,
        "RightBracket" => 0x30,
        "BackSlash" => 0x31,
        "SemiColon" => 0x33,
        "Quote" => 0x34,
        "BackQuote" => 0x35,
        "Comma" => 0x36,
        "Dot" => 0x37,
        "Slash" => 0x38,
        "CapsLock" => 0x39,
        "PrintScreen" => 0x46,
        "ScrollLock" => 0x47,
        "Pause" => 0x48,
        "Insert" => 0x49,
        "Home" => 0x4a,
        "PageUp" => 0x4b,
        "Delete" => 0x4c,
        "End" => 0x4d,
        "PageDown" => 0x4e,
        "RightArrow" => 0x4f,
        "LeftArrow" => 0x50,
        "DownArrow" => 0x51,
        "UpArrow" => 0x52,
        "NumLock" => 0x53,
        "KpDivide" => 0x54,
        "KpMultiply" => 0x55,
        "KpMinus" => 0x56,
        "KpPlus" => 0x57,
        "KpReturn" => 0x58,
        "KpDelete" => 0x63,
        "IntlBackslash" => 0x64,
        "ControlLeft" => 0xe0,
        "ShiftLeft" => 0xe1,
        "Alt" => 0xe2,
        "MetaLeft" => 0xe3,
        "ControlRight" => 0xe4,
        "ShiftRight" => 0xe5,
        "AltGr" => 0xe6,
        "MetaRight" => 0xe7,
        _ => return None,
    })
}

pub fn hid_to_key_name(usage: u16) -> Option<String> {
    if (0x04..=0x1d).contains(&usage) {
        let letter = char::from(b'A' + u8::try_from(usage - 0x04).ok()?);
        return Some(format!("Key{letter}"));
    }
    if (0x1e..=0x26).contains(&usage) {
        return Some(format!("Num{}", usage - 0x1e + 1));
    }
    if usage == 0x27 {
        return Some("Num0".into());
    }
    if (0x3a..=0x45).contains(&usage) {
        return Some(format!("F{}", usage - 0x3a + 1));
    }
    if (0x59..=0x61).contains(&usage) {
        return Some(format!("Kp{}", usage - 0x59 + 1));
    }
    if usage == 0x62 {
        return Some("Kp0".into());
    }
    Some(
        match usage {
            0x28 => "Return",
            0x29 => "Escape",
            0x2a => "Backspace",
            0x2b => "Tab",
            0x2c => "Space",
            0x2d => "Minus",
            0x2e => "Equal",
            0x2f => "LeftBracket",
            0x30 => "RightBracket",
            0x31 => "BackSlash",
            0x33 => "SemiColon",
            0x34 => "Quote",
            0x35 => "BackQuote",
            0x36 => "Comma",
            0x37 => "Dot",
            0x38 => "Slash",
            0x39 => "CapsLock",
            0x46 => "PrintScreen",
            0x47 => "ScrollLock",
            0x48 => "Pause",
            0x49 => "Insert",
            0x4a => "Home",
            0x4b => "PageUp",
            0x4c => "Delete",
            0x4d => "End",
            0x4e => "PageDown",
            0x4f => "RightArrow",
            0x50 => "LeftArrow",
            0x51 => "DownArrow",
            0x52 => "UpArrow",
            0x53 => "NumLock",
            0x54 => "KpDivide",
            0x55 => "KpMultiply",
            0x56 => "KpMinus",
            0x57 => "KpPlus",
            0x58 => "KpReturn",
            0x63 => "KpDelete",
            0x64 => "IntlBackslash",
            0xe0 => "ControlLeft",
            0xe1 => "ShiftLeft",
            0xe2 => "Alt",
            0xe3 => "MetaLeft",
            0xe4 => "ControlRight",
            0xe5 => "ShiftRight",
            0xe6 => "AltGr",
            0xe7 => "MetaRight",
            _ => return None,
        }
        .into(),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        NativeInputEvent, events_match, hid_to_key_name, is_emergency_release, key_name_to_hid,
        update_pressed_keys,
    };
    use std::collections::HashSet;

    #[test]
    fn injected_pointer_comparison_allows_subpixel_rounding() {
        assert!(events_match(
            &NativeInputEvent::PointerMoved {
                x: 10.2,
                y: 19.8,
                delta_x: None,
                delta_y: None,
            },
            &NativeInputEvent::PointerMoved {
                x: 10.8,
                y: 20.1,
                delta_x: Some(1.0),
                delta_y: Some(1.0),
            }
        ));
    }

    #[test]
    fn emergency_shortcut_is_detected() {
        let mut keys = HashSet::new();
        update_pressed_keys(
            &NativeInputEvent::Key {
                code: "ControlLeft".into(),
                pressed: true,
            },
            &mut keys,
        );
        update_pressed_keys(
            &NativeInputEvent::Key {
                code: "Alt".into(),
                pressed: true,
            },
            &mut keys,
        );
        let escape = NativeInputEvent::Key {
            code: "Escape".into(),
            pressed: true,
        };
        assert!(is_emergency_release(&escape, &keys));
    }

    #[test]
    fn common_keys_round_trip_through_hid_usage() {
        for key in [
            "KeyA",
            "KeyZ",
            "Num0",
            "Num7",
            "F1",
            "F12",
            "Return",
            "LeftArrow",
            "ControlLeft",
            "MetaRight",
            "Kp8",
        ] {
            let usage = key_name_to_hid(key).expect(key);
            assert_eq!(hid_to_key_name(usage).as_deref(), Some(key));
        }
    }
}
