# InputMesh v0.1.5

## Second-stage fixes

- Keep remote pointer focus when the receiving computer's local keyboard is used.
- Require a real directional, overlapping screen edge before pointer crossing.
- Post macOS pointer movement as dragged events while a remote mouse button is held.
- Preserve pixel/line wheel units and normalize scrolling between macOS and Windows.

## Validation status

- macOS automated checks pass (55 Rust tests plus frontend tests); Windows
  automated checks pass (54 Rust tests plus frontend tests).
- Strict application clippy checks pass on both platforms.
- Physical two-host checks cover both keyboard-focus directions, directional
  edge crossing and a held-button drag from Windows into macOS.
- Matched macOS arm64 and Windows x64 installers were built and their SHA-256
  values match `manifest.json`.

See `docs/TEST_REPORT_0.1.5.md` for the verification record.

## Preview signing status

Windows packages are not Authenticode-signed. The macOS app is ad-hoc signed
and not notarized. This release is intended for trusted testing environments.
