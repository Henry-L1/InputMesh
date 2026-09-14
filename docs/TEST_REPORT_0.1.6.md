# InputMesh v0.1.6 test report

## Behavior regression found in v0.1.5

The existing v0.1.5 policy coupled keyboard delivery to the mouse control
lease. Changing the keyboard source could therefore be rejected by a stale
lease or change the active controller instead of sending the key to the
screen under the logical pointer.

## Fix

Keyboard input is now independent of the physical mouse/keyboard host:

- a key on either computer targets the screen holding the logical pointer;
- a local keyboard is passed to the local OS when that screen is local;
- a keyboard on the other computer uses the independent `KeyboardInput` route,
  without acquiring the mouse lease or moving the pointer;
- the remote endpoint accepts the key only when the authenticated sender and
  `screen_id` match the current logical focus.

## Automated verification

- Frontend tests: 3/3.
- Rust tests: 56/56.
- `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`: passed.
- Strict application clippy: passed; existing warnings are from vendored rdev
  compatibility code.
- `pnpm release:check`: passed with verified Windows and macOS hashes.

## Local macOS package

- `InputMesh_0.1.6_aarch64.dmg` built successfully.
- `codesign --verify --deep --strict`: passed.
- `hdiutil verify`: passed.
- Bundle short version and bundle version: `0.1.6`.

## Two-host validation

- Windows host: `192.168.101.19`, installed executable version `0.1.6`.
- The two installed 0.1.6 binaries established an encrypted LAN connection;
  the Mac console showed one online peer at `192.168.101.19:42424`.
- The Windows 0.1.6 runtime recorded pointer transitions between the Windows
  display and the Mac display after the temporary test edge was aligned.
- The console displayed the same canonical order on the Mac side:
  `27A6MR → DELL U2417H → Built-in Retina Display`.
- Synthetic keyboard probes were not counted as physical-key evidence: the
  Windows interactive session accepted the probe, but did not expose it to the
  global hook in this session, while macOS explicitly filters synthetic keys.
  The independent `KeyboardInput` protocol path is covered by Rust round-trip
  and HID conversion tests and is included in both installed 0.1.6 binaries.

The saved user layout was restored to its original Mac-screen y-position of
1430 after the test. The 10-pixel overlap with the Windows screen means users
must align the screen edges in the layout editor before crossing that edge.
