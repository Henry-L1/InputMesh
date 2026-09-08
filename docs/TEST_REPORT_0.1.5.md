# InputMesh v0.1.5 test report

Date: 2026-09-09

## Scope

This release addresses the second-stage cross-platform regressions on a
physical macOS arm64 host and a physical Windows x64 host connected over the
same trusted LAN.

## Automated verification

- macOS: frontend tests 3/3; Rust tests 55/55; strict application clippy passed.
- Windows: frontend tests 3/3; Rust tests 54/54; strict application clippy passed.
- Vendored macOS input simulator: held-button motion emits a native dragged
  event rather than an ordinary mouse-move event.
- Release metadata: `pnpm release:check` passed for v0.1.5.

The vendored Windows input dependency still prints one existing Rust 2024
compatibility warning about a mutable static. It does not fail the application
clippy gate and is not introduced by this release.

## Physical two-host checks

| Regression | Verification | Result |
| --- | --- | --- |
| Windows pointer on macOS jumps back after using the macOS keyboard | Crossed from Windows to macOS, sent a native macOS key event, then sent a small native Windows mouse delta | Pointer lease stayed on macOS; the Windows cursor remained parked |
| macOS pointer on Windows jumps back after using the Windows keyboard | Crossed from macOS to Windows, sent an unmarked native Windows key event, then continued macOS pointer input | Pointer lease stayed on Windows |
| A leftward macOS gesture incorrectly enters an upper Windows screen | With the saved stacked topology, tested left and up independently | Left stayed on macOS; up entered the intended Windows screen |
| Windows drag on macOS has no live movement | Held the Windows left button, moved the physical Windows pointer and inspected the macOS pointer/button state before release | macOS received held-button movement and the pointer moved before button-up |
| Scroll speed differs severely between platforms | Exercised line-to-pixel and pixel-to-line unit conversion, including fractional accumulation | Windows line input maps to 40 macOS pixels; four 10-pixel macOS deltas accumulate to one Windows line |

Temporary probe tasks and result files were removed after validation. The saved
user configuration was preserved on both hosts.

## Release artifacts

| Platform | File | SHA-256 |
| --- | --- | --- |
| Windows x64 | `InputMesh-v0.1.5-windows-x64-setup.exe` | `3b4bfecc700b39083f6dde11a25f46dfe839c0c7d6653f08a858251cccf2d620` |
| Windows x64 | `InputMesh-v0.1.5-windows-x64.msi` | `65748e5cd0913d5ea5bf40989ff6f26112756ce644ce76b78f7d9b41226fd15e` |
| macOS arm64 | `InputMesh-v0.1.5-macos-arm64.dmg` | `65fe7a3972bc9df59803c0946a9c0e9b0768a2b2de58faa52127ef0142bbc610` |
| macOS arm64 | `InputMesh-v0.1.5-macos-arm64.app.zip` | `dd65a499ae1a86473613270efde8825ed64881345cb530a338f4010100c263b1` |

Windows packages are not Authenticode-signed. The macOS app is ad-hoc signed
and not notarized, so v0.1.5 remains a preview release for trusted testing.
