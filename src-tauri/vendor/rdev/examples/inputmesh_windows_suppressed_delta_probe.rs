#[cfg(target_os = "windows")]
fn main() {
    use rdev::{grab, simulate, EventType};
    use std::{sync::mpsc, thread, time::Duration};
    use winapi::um::winuser::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

    let original = cursor_position().expect("GetCursorPos failed");
    let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    let park = (width / 2, height / 2);
    let proposed = (park.0 + 32, park.1);
    let (delta_tx, delta_rx) = mpsc::sync_channel(2);

    thread::spawn(move || {
        let _ = grab(move |event| {
            if matches!(event.event_type, EventType::MouseMove { .. }) {
                let _ = delta_tx.try_send(event.relative_delta);
                // Model remote-screen input: the native event is withheld, so
                // Windows leaves the real cursor at the parking point.
                return None;
            }
            Some(event)
        });
    });
    thread::sleep(Duration::from_millis(300));

    simulate(&EventType::MouseMove {
        x: f64::from(park.0),
        y: f64::from(park.1),
    })
    .expect("failed to establish the tagged parking position");
    thread::sleep(Duration::from_millis(100));

    send_unmarked_absolute(proposed.0, proposed.1, width, height);
    let first = delta_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("the hook did not receive the first unmarked movement")
        .expect("the first movement had no relative delta");
    send_unmarked_absolute(proposed.0, proposed.1, width, height);
    let second = delta_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("the hook did not receive the second unmarked movement")
        .expect("the second movement had no relative delta");

    simulate(&EventType::MouseMove {
        x: f64::from(original.0),
        y: f64::from(original.1),
    })
    .expect("failed to restore the original cursor position");

    assert!(
        first.0 > 0,
        "first suppressed movement was not positive: {:?}",
        first
    );
    assert!(
        second.0 > 0,
        "second suppressed movement lost its delta after the first was swallowed: {:?}",
        second
    );
    println!(
        "{{\"ok\":true,\"suppressedBaselineReset\":true,\"firstDelta\":{{\"x\":{},\"y\":{}}},\"secondDelta\":{{\"x\":{},\"y\":{}}}}}",
        first.0, first.1, second.0, second.1
    );
}

#[cfg(target_os = "windows")]
fn send_unmarked_absolute(x: i32, y: i32, width: i32, height: i32) {
    use std::mem::size_of;
    use winapi::{
        ctypes::c_int,
        shared::minwindef::UINT,
        um::winuser::{
            INPUT_u, SendInput, INPUT, INPUT_MOUSE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_MOVE,
            MOUSEINPUT,
        },
    };

    let normalized_x = i64::from(x) * 65_535 / i64::from(width - 1);
    let normalized_y = i64::from(y) * 65_535 / i64::from(height - 1);
    let mut union: INPUT_u = unsafe { std::mem::zeroed() };
    *unsafe { union.mi_mut() } = MOUSEINPUT {
        dx: normalized_x as i32,
        dy: normalized_y as i32,
        mouseData: 0,
        dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE,
        time: 0,
        dwExtraInfo: 0,
    };
    let mut input = INPUT {
        type_: INPUT_MOUSE,
        u: union,
    };
    assert_eq!(
        unsafe { SendInput(1 as UINT, &mut input, size_of::<INPUT>() as c_int) },
        1,
        "unmarked absolute SendInput failed"
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
    eprintln!("inputmesh_windows_suppressed_delta_probe only runs on Windows");
}
