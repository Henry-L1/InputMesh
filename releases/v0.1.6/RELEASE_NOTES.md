# InputMesh v0.1.6

## Keyboard-source independence

- Keyboard input follows the logical pointer's current screen, regardless of
  which computer the physical keyboard is connected to.
- A keyboard on the other computer can type into the current remote screen
  through the independent keyboard route, without moving the pointer or
  requiring “本机鼠标优先”.
- The local keyboard still types directly when the pointer is already on the
  local screen, so it does not steal the pointer lease.

## Validation status

- Frontend tests: 3/3.
- Rust tests: 56/56.
- macOS arm64 local bundle: built as 0.1.6; deep signature and DMG verification
  passed.
- Windows x64 NSIS and MSI bundles: built as 0.1.6 on `192.168.101.19`.
- Two-host validation: both installed 0.1.6 binaries connected over the LAN,
  pointer transitions were observed, and the console used the same canonical
  screen order on both sides. Synthetic keyboard probes were not counted as
  physical-key evidence because the active Windows/macOS global hooks filter
  synthetic events in this session; the independent keyboard protocol path is
  covered by the Rust tests and shipped in both binaries.
- SHA-256 manifests are recorded under `windows/SHA256SUMS.txt` and
  `macos/SHA256SUMS.txt`.
