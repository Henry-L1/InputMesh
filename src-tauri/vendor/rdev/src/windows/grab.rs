use crate::rdev::{Event, EventType, GrabError};
use crate::windows::common::{
    convert, get_point, is_inputmesh_event, set_key_hook, set_mouse_hook, HookError, HOOK, KEYBOARD,
};
use lazy_static::lazy_static;
use std::ptr::null_mut;
use std::sync::Mutex;
use std::time::SystemTime;
use winapi::shared::windef::POINT;
use winapi::um::winuser::{
    CallNextHookEx, DispatchMessageA, GetCursorPos, GetMessageA, TranslateMessage, HC_ACTION, MSG,
};

static LAST_MOUSE_POINT: Mutex<Option<(i32, i32)>> = Mutex::new(None);
lazy_static! {
    static ref GLOBAL_CALLBACK: Mutex<Option<Box<dyn FnMut(Event) -> Option<Event> + Send>>> =
        Mutex::new(None);
}

unsafe fn reset_mouse_baseline_to_cursor() {
    let mut point = POINT { x: 0, y: 0 };
    if GetCursorPos(&mut point) != 0 {
        *LAST_MOUSE_POINT
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some((point.x, point.y));
    }
}

unsafe extern "system" fn raw_callback(code: i32, param: usize, lpdata: isize) -> isize {
    if code == HC_ACTION {
        let is_inputmesh_synthetic = is_inputmesh_event(param, lpdata);
        let opt = convert(param, lpdata);
        if let Some(event_type) = opt {
            let is_mouse_move = matches!(&event_type, EventType::MouseMove { .. });
            let relative_delta = match event_type {
                EventType::MouseMove { .. } => {
                    let point = get_point(lpdata);
                    let mut previous = LAST_MOUSE_POINT
                        .lock()
                        .unwrap_or_else(|error| error.into_inner());
                    let delta = previous.map(|(x, y)| {
                        (
                            i64::from(point.0) - i64::from(x),
                            i64::from(point.1) - i64::from(y),
                        )
                    });
                    *previous = Some(point);
                    delta
                }
                _ => None,
            };
            // SendInput calls low-level hooks synchronously. The application
            // callback can itself inject a cursor warp while returning from a
            // remote screen, so trying to lock GLOBAL_CALLBACK again here
            // would deadlock the hook thread. InputMesh-tagged events are only
            // feedback from our own injector; update the mouse baseline above
            // and let them pass without re-entering the application callback.
            if is_inputmesh_synthetic {
                return CallNextHookEx(HOOK, code, param, lpdata);
            }
            let name = match &event_type {
                EventType::KeyPress(_key) => match (*KEYBOARD).lock() {
                    Ok(mut keyboard) => keyboard.get_name(lpdata),
                    Err(_) => None,
                },
                _ => None,
            };
            let event = Event {
                event_type,
                time: SystemTime::now(),
                name,
                relative_delta,
                is_synthetic: false,
            };
            let mut callback_slot = GLOBAL_CALLBACK
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if let Some(callback) = callback_slot.as_mut() {
                if callback(event).is_none() {
                    if is_mouse_move {
                        // A swallowed WH_MOUSE_LL event never updates the real
                        // cursor. Keep the next delta relative to that unchanged
                        // cursor instead of the discarded event's proposed point.
                        // This lets remote motion reverse immediately after the
                        // logical cursor reaches an outer screen edge.
                        reset_mouse_baseline_to_cursor();
                    }
                    // https://stackoverflow.com/questions/42756284/blocking-windows-mouse-click-using-setwindowshookex
                    // https://android.developreference.com/article/14560004/Blocking+windows+mouse+click+using+SetWindowsHookEx()
                    // https://cboard.cprogramming.com/windows-programming/99678-setwindowshookex-wm_keyboard_ll.html
                    // let _result = CallNextHookEx(HOOK, code, param, lpdata);
                    return 1;
                }
            }
        }
    }
    CallNextHookEx(HOOK, code, param, lpdata)
}
impl From<HookError> for GrabError {
    fn from(error: HookError) -> Self {
        match error {
            HookError::Mouse(code) => GrabError::MouseHookError(code),
            HookError::Key(code) => GrabError::KeyHookError(code),
        }
    }
}

pub fn grab<T>(callback: T) -> Result<(), GrabError>
where
    T: FnMut(Event) -> Option<Event> + Send + 'static,
{
    unsafe {
        *GLOBAL_CALLBACK
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(Box::new(callback));
        set_key_hook(raw_callback)?;
        set_mouse_hook(raw_callback)?;

        let mut message: MSG = std::mem::zeroed();
        while GetMessageA(&mut message, null_mut(), 0, 0) > 0 {
            TranslateMessage(&message);
            DispatchMessageA(&message);
        }
    }
    Ok(())
}
