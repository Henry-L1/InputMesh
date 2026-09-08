#[cfg(target_os = "windows")]
fn main() {
    use rdev::{grab, simulate, EventType};
    use std::{
        sync::{
            atomic::{AtomicBool, Ordering},
            mpsc, Arc,
        },
        thread,
        time::Duration,
    };
    use winapi::um::winuser::{
        mouse_event, GetSystemMetrics, MOUSEEVENTF_MOVE, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
        SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
    };

    let original = cursor_position().expect("GetCursorPos failed");
    let left = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let top = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
    assert!(
        width > 1 && height > 1,
        "Windows reported an empty virtual desktop"
    );

    let target_x = (original.0 + 17).clamp(left, left + width - 1);
    let target_y = (original.1 + 11).clamp(top, top + height - 1);
    let (event_tx, event_rx) = mpsc::sync_channel(1);
    let injected_from_callback = Arc::new(AtomicBool::new(false));

    let callback_guard = Arc::clone(&injected_from_callback);
    thread::spawn(move || {
        let _ = grab(move |event| {
            if matches!(event.event_type, EventType::MouseMove { .. })
                && !callback_guard.swap(true, Ordering::AcqRel)
            {
                // This is the production failure mode: returning from a remote
                // screen injects an absolute warp while the physical event's
                // low-level-hook callback is still active. The nested tagged
                // hook event must bypass the callback mutex and return.
                let result = simulate(&EventType::MouseMove {
                    x: f64::from(target_x),
                    y: f64::from(target_y),
                });
                let _ = event_tx.try_send(result);
                // The physical boundary event is suppressed in production;
                // otherwise it would apply after the nested absolute warp.
                return None;
            }
            Some(event)
        });
    });
    thread::sleep(Duration::from_millis(300));

    // Generate a non-InputMesh mouse event so the callback above runs. Its
    // nested InputMesh SendInput call used to deadlock permanently.
    unsafe { mouse_event(MOUSEEVENTF_MOVE, 1, 0, 0, 0) };

    event_rx
        .recv_timeout(Duration::from_secs(3))
        .expect("nested SendInput deadlocked inside the low-level hook callback")
        .expect("nested SendInput failed; run the probe in the interactive Windows session");
    let observed = cursor_position().expect("GetCursorPos failed after nested SendInput");

    simulate(&EventType::MouseMove {
        x: f64::from(original.0),
        y: f64::from(original.1),
    })
    .expect("failed to restore the original cursor position");

    assert!((observed.0 - target_x).abs() <= 1);
    assert!((observed.1 - target_y).abs() <= 1);
    println!(
        "{{\"ok\":true,\"reentrantSendInput\":true,\"syntheticCallbackBypassed\":true,\"virtualDesktop\":{{\"x\":{left},\"y\":{top},\"width\":{width},\"height\":{height}}},\"target\":{{\"x\":{target_x},\"y\":{target_y}}},\"observed\":{{\"x\":{},\"y\":{}}}}}",
        observed.0, observed.1
    );
}

#[cfg(target_os = "windows")]
fn cursor_position() -> Option<(i32, i32)> {
    use winapi::{shared::windef::POINT, um::winuser::GetCursorPos};

    let mut point = POINT { x: 0, y: 0 };
    (unsafe { GetCursorPos(&mut point) } != 0).then_some((point.x, point.y))
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("inputmesh_windows_probe only runs on Windows");
}
