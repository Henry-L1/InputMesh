# InputMesh v0.1.4 release manifest

Tag, application metadata and every recorded asset in this directory use version `0.1.4`. Windows and macOS artifacts were rebuilt from the dependency-audited source and uploaded to the public GitHub prerelease. The packages are unsigned previews, so users should verify the checksums and expect platform security warnings.

| Platform | Architecture | Assets | Checksums |
| --- | --- | --- | --- |
| Windows | x64 | NSIS setup EXE, MSI | [windows/SHA256SUMS.txt](windows/SHA256SUMS.txt) |
| macOS | arm64 | DMG, application ZIP | [macos/SHA256SUMS.txt](macos/SHA256SUMS.txt) |

`pnpm release:check` verifies that `package.json`, Cargo, Tauri, the tag and all asset names remain aligned before CI builds the project. The release workflow also publishes per-platform `.sha256` files.

Download only from the [official v0.1.4 prerelease](https://github.com/Henry-L1/InputMesh/releases/tag/v0.1.4). The macOS artifacts support Apple Silicon (`arm64`); the Windows artifacts support `x64`.
