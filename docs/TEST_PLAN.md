# InputMesh test plan

This plan is the release gate for each macOS and Windows build. Tests are ordered so a
failure can be assigned to one layer instead of being diagnosed by feel.

## 1. Static and unit checks

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all-targets`
- `pnpm test` and `pnpm build`
- Protocol round trips, malformed frames, stale input sequence rejection,
  topology edge selection, different resolutions, layout snapping and layout
  normalization must all have automated coverage.

## 2. Package checks

- Build the app and DMG from a clean source tree.
- Verify bundle version, designated identifier, deep code signature, binary
  hash and DMG hash.
- Development builds must be processed by `tools/macos/sign-local-macos.sh`; assert
  that the designated requirement remains `identifier
  "com.inputmesh.desktop"` after rebuilding with a changed binary.
- Install the identical app binary on both Macs without storing build artifacts
  in iCloud.
- Preserve the previous app bundle as a rollback point.

## 3. Startup and permission checks

- With `sharingEnabled=true`, quit and relaunch InputMesh. The global input tap
  must be installed automatically and the snapshot must report sharing active.
- Verify Accessibility and Input Monitoring authorization on both Macs.
- Verify one listener and one established authenticated connection per peer.
- Relaunch each side independently and confirm pairing survives.

## 4. Layout integration checks

- Move one screen on Mac A and assert both persisted layouts become identical.
- Repeat from Mac B.
- Test left, right, above and below arrangements.
- Assert adjacent edges touch, overlap exists on the perpendicular axis, no
  screens overlap in area, and the minimum canvas origin is `(0, 0)`.
- Disable and re-enable a remote screen; verify the change synchronizes and no
  device can lose its final enabled local screen.

## 5. Real pointer path checks

Build `tools/macos/macos_pointer_probe.c` with ApplicationServices. For each direction:

1. Read both cursor positions.
2. Post a deterministic sequence of mouse events on the controller, including
   explicit CoreGraphics relative deltas for longer than `switchDelayMs`.
3. Confirm the controller parks its native cursor and the peer cursor receives
   focus at the expected normalized edge coordinate.
4. Continue posting positive and negative deltas and assert the peer cursor
   moves monotonically without acceleration, jumps or reversal.
5. Move back across the opposite edge and assert control returns locally.

Repeat with a mouse button held, then test click, wheel, key press/release and
the `Ctrl + Alt + Esc` emergency release path. Release every injected key and
button in cleanup even if a test fails.

On Windows, run `tools/windows/run-windows-input-probe.ps1` in the signed-in user's
interactive session. It must report the complete virtual desktop (including a
negative origin when present), move and restore the cursor within one pixel,
and observe InputMesh's synthetic-event marker. For a keyboard smoke test, run
`tools/windows/run-windows-key-observer.ps1` in that same session, move focus to a
Windows screen from the peer, and send a harmless function-key press/release.
The observer must report both states through `GetAsyncKeyState`.

## 6. Reliability checks

- Run 100 round trips and a 10-minute continuous pointer stream.
- Restart one peer during remote control and assert the surviving peer releases
  all held keys/buttons and restores its local cursor.
- Disconnect/reconnect Wi-Fi and verify automatic recovery without duplicate
  connections.
- Confirm no new crash report, error/fault log, stuck key, stuck button or
  orphan InputMesh process remains.

## Release criteria

All automated checks pass, installed artifacts match the hashes recorded for
the candidate, layout files agree after either endpoint edits them, and real
macOS/macOS plus macOS/Windows pointer and keyboard probes cross in both
directions without user intervention.
