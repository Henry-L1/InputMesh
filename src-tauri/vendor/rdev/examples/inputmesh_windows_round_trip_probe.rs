#[cfg(target_os = "windows")]
fn main() {
    use rdev::{simulate, EventType};
    use std::{thread, time::Duration};
    use winapi::um::winuser::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

    let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    assert!(
        width > 2 && height > 2,
        "Windows reported an empty primary display"
    );

    // The deployed test layout places macOS immediately below the Windows
    // primary display. Start on its final pixel without presenting the warp as
    // physical input to InputMesh.
    let start_x = width / 2;
    let start_y = height - 1;
    simulate(&EventType::MouseMove {
        x: f64::from(start_x),
        y: f64::from(start_y),
    })
    .expect("failed to place the Windows cursor at the shared edge");
    thread::sleep(Duration::from_millis(250));

    // Windows -> macOS, hit the remote outer edge, reverse away from that edge,
    // macOS -> Windows, then repeat a complete crossing. The first return
    // executes the formerly deadlocking nested cursor warp. Moving away from
    // the outer edge verifies that swallowed Windows events keep a stable
    // native baseline instead of making the remote cursor feel magnetized.
    send_unmarked_relative(0, 8);
    thread::sleep(Duration::from_millis(500));
    send_unmarked_relative(0, 5_000);
    thread::sleep(Duration::from_millis(500));
    send_unmarked_relative(0, -64);
    thread::sleep(Duration::from_millis(500));
    send_unmarked_relative(0, -5_000);
    thread::sleep(Duration::from_millis(500));
    send_unmarked_relative(0, 5_000);
    thread::sleep(Duration::from_millis(500));
    send_unmarked_relative(0, -5_000);
    thread::sleep(Duration::from_millis(500));

    println!(
        "{{\"ok\":true,\"sequence\":[\"windows-to-macos\",\"macos-outer-edge\",\"reverse-away-from-edge\",\"macos-to-windows\",\"windows-to-macos\",\"macos-to-windows\"],\"start\":{{\"x\":{start_x},\"y\":{start_y}}}}}"
    );
}

#[cfg(target_os = "windows")]
fn send_unmarked_relative(x: i32, y: i32) {
    use std::mem::size_of;
    use winapi::{
        ctypes::c_int,
        shared::minwindef::UINT,
        um::winuser::{INPUT_u, SendInput, INPUT, INPUT_MOUSE, MOUSEEVENTF_MOVE, MOUSEINPUT},
    };

    let mut union: INPUT_u = unsafe { std::mem::zeroed() };
    *unsafe { union.mi_mut() } = MOUSEINPUT {
        dx: x,
        dy: y,
        mouseData: 0,
        dwFlags: MOUSEEVENTF_MOVE,
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
        "unmarked SendInput failed"
    );
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("inputmesh_windows_round_trip_probe only runs on Windows");
}
