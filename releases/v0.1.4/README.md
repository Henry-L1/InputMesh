# InputMesh v0.1.4 release manifest

Tag, application metadata and every asset in this directory use version `0.1.4`. Windows and macOS artifacts were built from the same source candidate and are published together in one GitHub Release.

| Platform | Architecture | Assets | Checksums |
| --- | --- | --- | --- |
| Windows | x64 | NSIS setup EXE, MSI | [windows/SHA256SUMS.txt](windows/SHA256SUMS.txt) |
| macOS | arm64 | DMG, application ZIP | [macos/SHA256SUMS.txt](macos/SHA256SUMS.txt) |

`pnpm release:check` verifies that `package.json`, Cargo, Tauri, the tag and all asset names remain aligned before CI builds the project.

These v0.1.4 files are unsigned public preview artifacts for noncommercial testing: the macOS build is ad-hoc signed and not notarized; the Windows installers are not Authenticode-signed. Verify the published SHA-256 values before installation.
