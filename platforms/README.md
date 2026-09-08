# Platform source layout

InputMesh deliberately uses one repository and one shared application core, while keeping operating-system integrations in separate directories. The protocol, connection state, screen topology, control lease and UI must behave identically on both systems; duplicating them into independent Windows and macOS projects would create protocol drift and mismatched releases. Native input capture/injection, validation tools and packaged artifacts remain platform-specific.

| Scope | Directory |
| --- | --- |
| Shared Rust protocol/runtime | `src-tauri/src` |
| Windows input backend | `src-tauri/vendor/rdev/src/windows` |
| Windows validation tools | `tools/windows` |
| macOS input backend | `src-tauri/vendor/rdev/src/macos` |
| macOS validation/signing tools | `tools/macos` |

Release records are also separated under `releases/<version>/windows` and
`releases/<version>/macos`. Windows packages must be built on Windows and macOS
packages on macOS; neither platform's binaries belong in the other's directory.

See the platform-specific notes in [windows](windows/README.md) and [macos](macos/README.md).
