# Platform source layout

InputMesh uses one shared protocol/runtime and separate operating-system input backends. Platform code must stay in the directories below; release binaries are never committed to these source directories.

| Scope | Directory |
| --- | --- |
| Shared Rust protocol/runtime | `src-tauri/src` |
| Windows input backend | `src-tauri/vendor/rdev/src/windows` |
| Windows validation tools | `tools/windows` |
| macOS input backend | `src-tauri/vendor/rdev/src/macos` |
| macOS validation/signing tools | `tools/macos` |

See the platform-specific notes in [windows](windows/README.md) and [macos](macos/README.md).
