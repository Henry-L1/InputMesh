# Windows

Windows-specific input capture, suppression and injection code is kept in `src-tauri/vendor/rdev/src/windows`. Interactive validation launchers are kept in `tools/windows`.

Build Windows release artifacts on Windows:

```powershell
pnpm install --frozen-lockfile
pnpm release:check
pnpm check
pnpm tauri build
```

For v0.1.4, publish only the filenames and SHA-256 values recorded in `releases/v0.1.4/windows/SHA256SUMS.txt`.
