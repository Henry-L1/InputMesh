# Third-party notices

InputMesh includes and depends on third-party software. The PolyForm
Noncommercial License applies only to InputMesh project-owned code. Third-party
components remain available under their own licenses.

## Vendored source

- `rdev` 0.5.3, Copyright Nicolas Patry and contributors, MIT License. The
  license text is preserved at `src-tauri/vendor/rdev/LICENSE` and local changes
  are documented in the source tree.

## Dependency license families

The locked JavaScript and Rust dependency graphs include components under MIT,
Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0, CC0-1.0,
BlueOak-1.0.0, MPL-2.0 and compatible dual-license expressions. MPL-2.0
components currently include `cssparser`, `cssparser-macros`, `dtoa-short`,
`epoll`, `option-ext`, `selectors`, `lightningcss` and
`lightningcss-darwin-arm64`.

Exact versions are fixed in `pnpm-lock.yaml` and `src-tauri/Cargo.lock`. Before
each binary release, regenerate the dependency inventory, review license
changes, and bundle all license and attribution texts required by the selected
platform artifacts.

Useful review commands:

```bash
pnpm licenses list --json
cargo metadata --manifest-path src-tauri/Cargo.toml --format-version 1
```
