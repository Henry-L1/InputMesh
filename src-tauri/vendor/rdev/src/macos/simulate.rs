use crate::rdev::{Button, EventType, SimulateError};
use core_graphics::event::{
    CGEvent, CGEventTapLocation, CGEventType, CGMouseButton, ScrollEventUnit,
};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;
use lazy_static::lazy_static;
use std::collections::HashSet;
use std::convert::TryInto;
use std::sync::Mutex;

use crate::macos::keycodes::code_from_key;
use crate::macos::SYNTHETIC_EVENT_MARKER;

lazy_static! {
    static ref PRESSED_MOUSE_BUTTONS: Mutex<HashSet<Button>> = Mutex::new(HashSet::new());
}

fn mouse_move_event_type(pressed: &HashSet<Button>) -> (CGEventType, CGMouseButton) {
    if pressed.contains(&Button::Left) {
        (CGEventType::LeftMouseDragged, CGMouseButton::Left)
    } else if pressed.contains(&Button::Right) {
        (CGEventType::RightMouseDragged, CGMouseButton::Right)
    } else if pressed.contains(&Button::Middle) {
        (CGEventType::OtherMouseDragged, CGMouseButton::Center)
    } else {
        (CGEventType::MouseMoved, CGMouseButton::Left)
    }
}

unsafe fn convert_native_with_source(
    event_type: &EventType,
    source: CGEventSource,
    pressed_buttons: &HashSet<Button>,
) -> Option<CGEvent> {
    match event_type {
        EventType::KeyPress(key) => {
            let code = code_from_key(*key)?;
            CGEvent::new_keyboard_event(source, code, true).ok()
        }
        EventType::KeyRelease(key) => {
            let code = code_from_key(*key)?;
            CGEvent::new_keyboard_event(source, code, false).ok()
        }
        EventType::ButtonPress(button) => {
            let point = get_current_mouse_location()?;
            let (event, native_button) = match button {
                Button::Left => (CGEventType::LeftMouseDown, CGMouseButton::Left),
                Button::Right => (CGEventType::RightMouseDown, CGMouseButton::Right),
                Button::Middle => (CGEventType::OtherMouseDown, CGMouseButton::Center),
                _ => return None,
            };
            CGEvent::new_mouse_event(source, event, point, native_button).ok()
        }
        EventType::ButtonRelease(button) => {
            let point = get_current_mouse_location()?;
            let (event, native_button) = match button {
                Button::Left => (CGEventType::LeftMouseUp, CGMouseButton::Left),
                Button::Right => (CGEventType::RightMouseUp, CGMouseButton::Right),
                Button::Middle => (CGEventType::OtherMouseUp, CGMouseButton::Center),
                _ => return None,
            };
            CGEvent::new_mouse_event(source, event, point, native_button).ok()
        }
        EventType::MouseMove { x, y } => {
            let point = CGPoint { x: (*x), y: (*y) };
            let (event, button) = mouse_move_event_type(pressed_buttons);
            CGEvent::new_mouse_event(source, event, point, button).ok()
        }
        EventType::Wheel { delta_x, delta_y } => {
            let wheel_count = 2;
            CGEvent::new_scroll_event(
                source,
                ScrollEventUnit::PIXEL,
                wheel_count,
                (*delta_y).try_into().ok()?,
                (*delta_x).try_into().ok()?,
                0,
            )
            .ok()
        }
    }
}

unsafe fn get_current_mouse_location() -> Option<CGPoint> {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState).ok()?;
    let event = CGEvent::new(source).ok()?;
    Some(event.location())
}

#[link(name = "Cocoa", kind = "framework")]
extern "C" {}

pub fn simulate(event_type: &EventType) -> Result<(), SimulateError> {
    unsafe {
        let mut pressed = PRESSED_MOUSE_BUTTONS
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|_| SimulateError)?;
        if let Some(cg_event) = convert_native_with_source(event_type, source, &pressed) {
            cg_event.set_integer_value_field(
                core_graphics::event::EventField::EVENT_SOURCE_USER_DATA,
                SYNTHETIC_EVENT_MARKER,
            );
            cg_event.post(CGEventTapLocation::HID);
            match event_type {
                EventType::ButtonPress(button) => {
                    pressed.insert(*button);
                }
                EventType::ButtonRelease(button) => {
                    pressed.remove(button);
                }
                _ => {}
            }
            Ok(())
        } else {
            Err(SimulateError)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_motion_uses_drag_event_while_a_button_is_held() {
        let none = HashSet::new();
        assert_eq!(mouse_move_event_type(&none).0 as u32, CGEventType::MouseMoved as u32);

        let left = HashSet::from([Button::Left]);
        assert_eq!(
            mouse_move_event_type(&left).0 as u32,
            CGEventType::LeftMouseDragged as u32
        );

        let right = HashSet::from([Button::Right]);
        assert_eq!(
            mouse_move_event_type(&right).0 as u32,
            CGEventType::RightMouseDragged as u32
        );
    }
}
