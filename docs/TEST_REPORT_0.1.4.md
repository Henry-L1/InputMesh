# InputMesh 0.1.4 two-Mac test report

> Historical candidate report: the hashes below document the 2026-09-05 two-Mac test build and are not the canonical GitHub Release assets. The matched Windows/macOS v0.1.4 release files and current checksums are recorded in [`releases/v0.1.4`](../releases/v0.1.4/README.md).

Date: 2026-09-05

## Test environment

- Controller Mac: `192.0.2.114` (documentation address)
- Peer Mac: `192.0.2.102` (documentation address)
- Installed bundle: `/Applications/InputMesh.app`
- Version on both Macs: `0.1.4`
- Binary SHA-256 on both Macs:
  `6fe139e84b1a8d44b56ba0fdf88608164c248c9f7d6c9134cd781005223b7b5b`
- Designated requirement on both Macs:
  `identifier "com.inputmesh.desktop"`
- DMG SHA-256:
  `8eef077c9961711c8d511dc3f60fe8e2012e19e31ff9baa571ad50e63724e091`

## Automated checks

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Pass |
| `cargo test --all-targets` | Pass, 44 tests |
| `cargo clippy --all-targets -- -D warnings` | Pass |
| `pnpm test` | Pass, 3 tests |
| `pnpm build` | Pass |
| Deep bundle signature verification | Pass on both Macs |
| Installed binary equality | Pass |

## Live two-Mac checks

| Check | Result | Evidence |
| --- | --- | --- |
| Persisted sharing after relaunch | Pass | Both peers logged `resumed persisted input sharing` and installed their input taps. |
| Authenticated peer connection | Pass | One established TCP connection between the two test hosts (addresses anonymized). |
| Layout synchronization | Pass | Both peers persisted the same device positions: local `(0, 0)`, peer `(18, 864)`. |
| Initial pointer crossing | Pass | Peer cursor moved from `(444, 488)` to the expected entry point `(748, 106)`. |
| Relative pointer movement | Pass | Three `+40` deltas produced `(788, 106)`, `(828, 106)`, `(868, 106)` with no acceleration or jump. |
| Return to local control | Pass | Runtime logged the peer-to-local topology transition and restored local control. |
| Mouse button forwarding | Pass | Local button remained suppressed while the peer reported pressed, then released cleanly. |
| Receiver feedback suppression | Pass | Ten stress rounds produced zero topology transitions on the receiving peer. |
| Peer restart and reconnect | Pass | Disconnect was detected; the peer relaunched, restored sharing and re-established the connection. |
| Crash and stderr check | Pass | No new crash report; both captured stderr logs were empty. |
| Permission persistence across rebuild | Pass | Accessibility/Input Monitoring grants survived the `0.1.3` to `0.1.4` update. |

## Defect fixed in 0.1.4

Injected CoreGraphics pointer events could miss coordinate-based expected-event
matching and be captured again as physical input. That created a reverse input
feedback loop and made the cursor appear to jump or run away.

InputMesh now marks every macOS-injected event with a private
`EVENT_SOURCE_USER_DATA` value. The event tap identifies that marker and passes
the event through without forwarding it to the peer. The stress test confirmed
that receiver-side topology transitions remained at zero.

## Coverage boundary

The physical key path was not re-measured through a macOS key-state API because
that API does not reliably expose state from posted synthetic events. Keyboard
protocol/HID unit coverage passes, and keyboard forwarding had already worked in
the user's live test. The 100-round and 10-minute endurance gates in
`docs/TEST_PLAN.md` remain release-candidate soak tests; this report covers the
P0 two-Mac functional and reconnect acceptance for `0.1.4`.
