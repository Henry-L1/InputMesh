# InputMesh

[简体中文](README.md) | English

InputMesh is a peer-to-peer keyboard and mouse sharing application for Windows and macOS. Run it on computers connected to the same trusted local network, arrange their displays on one canvas, and move the pointer across a configured edge to switch computers. Keyboard focus follows the pointer.

The project is currently a preview release. InputMesh's original code is licensed under the [PolyForm Noncommercial License 1.0.0](LICENSE), for personal learning, research, testing, and other non-commercial use.

## Quick start

### 1. Prepare the development environment

- Node.js 24 (the repository's `.nvmrc` specifies the major version).
- pnpm 11.19.0 (the `packageManager` field in `package.json` pins the version).
- Rust stable, version 1.88 or newer.
- macOS 12 or later: install Xcode Command Line Tools.
- Windows 10/11: install Visual Studio Build Tools with the MSVC C++ workload and make sure the WebView2 Runtime is available.

If pnpm is not installed, use Corepack or npm to install the pinned version:

```bash
corepack enable
corepack prepare pnpm@11.19.0 --activate
```

Alternatively:

```bash
npm install --global pnpm@11.19.0
```

### 2. Clone the repository

```bash
git clone https://github.com/Henry-L1/InputMesh.git
cd InputMesh
```

### 3. Install dependencies

Run this from the repository root:

```bash
pnpm install --frozen-lockfile
```

This installs the frontend, Tauri CLI, and test dependencies from `pnpm-lock.yaml` without changing the lockfile.

### 4. Start the desktop development app

```bash
pnpm tauri dev
```

This starts the React/Vite frontend and the Tauri/Rust desktop backend. The first run also compiles the Rust dependencies and may take longer. The desktop build is required to test native capabilities such as global keyboard and mouse capture, platform permissions, LAN discovery, encrypted sessions, and cross-device input forwarding.

To preview only the frontend control interface, run:

```bash
pnpm dev
```

The browser preview uses built-in demo data. It does not capture or inject real keyboard or mouse events, so it cannot replace `pnpm tauri dev` for two-computer testing.

### 5. Grant platform permissions

On the first macOS launch, open **System Settings → Privacy & Security** and grant InputMesh:

- Accessibility
- Input Monitoring

macOS 15 and later may also ask for Local Network access. Restart InputMesh after changing permissions.

When Windows Firewall prompts for the first time, allow access only on a trusted private network.

### 6. Optional VS Code extensions

These extensions are recommended for Rust, Tauri, TOML, and native debugging support:

- [Rust Analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
- [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
- [Even Better TOML](https://marketplace.visualstudio.com/items?itemName=tamasfe.even-better-toml)
- [CodeLLDB](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb)

If the VS Code `code` command is available, install them from a terminal:

```bash
code --install-extension rust-lang.rust-analyzer
code --install-extension tauri-apps.tauri-vscode
code --install-extension tamasfe.even-better-toml
code --install-extension vadimcn.vscode-lldb
```

## Test two computers

1. Connect the Windows and macOS computers to the same trusted LAN and start InputMesh on both.
2. Compare the six-digit pairing codes. Confirm that they match, then click **Allow** on both computers.
3. Drag the displays on the canvas until their edges match the physical arrangement. Disable any display that should not participate.
4. Click **Start sharing**, then move the pointer across a configured edge.

The default TCP listener starts at port `42424` and selects another available port when needed. Discovery is not trust: only a device confirmed by the user can receive or inject input. Do not enable keyboard and mouse sharing on an untrusted public Wi-Fi network.

## Verify, test, and package

Run frontend unit tests:

```bash
pnpm test
```

Run the TypeScript/Vite production build:

```bash
pnpm build
```

Run Rust tests:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Run the common local checks together:

```bash
pnpm check
```

`pnpm check` runs the frontend tests, frontend build, and Rust tests in sequence. Build an installer for the current platform with:

```bash
pnpm tauri build
```

Build Windows and macOS installers on their respective operating systems or CI runners. The current preview packages are not Apple Developer ID notarized or Windows Authenticode signed; stable distribution still requires platform signing, notarization, and broader real-device regression testing.

## Goals and architecture

The first release targets:

- One Tauri 2 / Rust codebase for Windows 10/11 and macOS 12+.
- mDNS discovery of InputMesh instances on the local network.
- End-to-end encrypted sessions using a Noise XX handshake, with first pairing confirmed by the same six-digit code on both devices.
- Automatic display enumeration with draggable placement and per-display enable/disable.
- Forwarding of mouse, wheel, and keyboard events after crossing a display edge. Keyboard events use physical key codes and default to a Windows-style layout; input-method layouts are not synchronized.
- Local physical input taking control according to a most-recent-user-wins policy.
- `Ctrl + Alt + Esc` as the local emergency release shortcut.

```text
React control interface
      │ Tauri commands / events
Rust application runtime
      ├── display topology and persisted configuration
      ├── mDNS LAN discovery
      ├── Noise XX encrypted TCP sessions
      └── platform input layer
            ├── macOS: CoreGraphics event tap (Accessibility + Input Monitoring)
            └── Windows: low-level hooks / SendInput
```

The protocol, control ownership, and screen-coordinate design are documented in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). The full engineering plan is in [docs/DEVELOPMENT_PLAN.md](docs/DEVELOPMENT_PLAN.md), and platform directory guidance is in [platforms](platforms/README.md).

## Download a preview build

If you only want to use a packaged application, download it from [GitHub Releases](https://github.com/Henry-L1/InputMesh/releases):

- macOS Apple Silicon (arm64): DMG or APP ZIP.
- Windows x64: NSIS installer EXE or MSI.
- SHA-256 checksum manifests are provided for each platform.

Download only from this repository's Release page and verify the SHA-256 values. Unsigned installers may show an unknown-developer or security warning.

## Platform code and releases

- Cross-platform protocol, networking, and control-ownership logic lives in `src-tauri/src`.
- The Windows input backend is in `src-tauri/vendor/rdev/src/windows`, with Windows tools in `tools/windows`.
- The macOS input backend is in `src-tauri/vendor/rdev/src/macos`, with macOS tools in `tools/macos`.
- Windows and macOS release packages must use the same version and be built on their respective platforms.

See [platforms/windows](platforms/windows/README.md) and [platforms/macos](platforms/macos/README.md) for platform-specific notes.

## Current limitations

- An operating system fundamentally has one system pointer. Multiple keyboards and mice are represented by a most-recent-real-input ownership policy, not multiple independent cursors.
- Clipboard, files, audio, and video are not forwarded.
- A normal user process does not control the Windows UAC secure desktop or the macOS login window.
- Every release candidate still needs validation on real Windows and Mac hardware. Unit tests on one machine cannot replace testing system permissions, sleep and wake, firewalls, and Wi-Fi latency.

## Related projects

- [Deskflow](https://github.com/deskflow/deskflow): mature and active; suitable for products that can use GPL-2.0 code directly.
- [Input Leap](https://github.com/input-leap/input-leap): archived; useful as a historical behavior reference.
- [Barrier](https://github.com/debauchee/barrier): unmaintained; not a good base for a new project.
- [Synergy](https://github.com/symless/synergy): active, and its source is also GPL-2.0 licensed.

InputMesh is a clean-room implementation to avoid importing GPL code from those projects. Its current protocol does not promise compatibility with them.

## License

InputMesh's original code is licensed under the [PolyForm Noncommercial License 1.0.0](LICENSE):

- Personal learning, research, testing, entertainment, and hobby projects are allowed when they are non-commercial.
- Viewing, modifying, and redistributing the source for those non-commercial purposes is allowed.
- Commercial products or services, paid distribution, commercial internal use, customer projects, and other commercial purposes are not allowed.
- Third-party components retain their own licenses; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

The project is source-available rather than open-source software under the OSI definition. The [LICENSE](LICENSE) text controls the exact boundaries.

For security issues, read [SECURITY.md](SECURITY.md). Before contributing, read [CONTRIBUTING.md](CONTRIBUTING.md).
