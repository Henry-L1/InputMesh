# InputMesh v0.1.4 release manifest

Tag, application metadata and every recorded asset in this directory use version `0.1.4`. Windows and macOS artifacts were built from the same source candidate. The unsigned binaries were withdrawn from the public GitHub Release after the pre-public dependency audit; their checksums remain here only as a historical test record.

| Platform | Architecture | Assets | Checksums |
| --- | --- | --- | --- |
| Windows | x64 | NSIS setup EXE, MSI | [windows/SHA256SUMS.txt](windows/SHA256SUMS.txt) |
| macOS | arm64 | DMG, application ZIP | [macos/SHA256SUMS.txt](macos/SHA256SUMS.txt) |

`pnpm release:check` verifies that `package.json`, Cargo, Tauri, the tag and all asset names remain aligned before CI builds the project.

The public v0.1.4 Release is source-only. Build from the current source tree; do not obtain the withdrawn binaries from third-party mirrors.
