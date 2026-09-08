# macOS

macOS-specific event-tap capture and CoreGraphics injection code is kept in `src-tauri/vendor/rdev/src/macos`. macOS validation and local-signing tools are kept in `tools/macos`.

Build macOS release artifacts on Apple Silicon macOS:

```bash
pnpm install --frozen-lockfile
pnpm release:check
pnpm check
pnpm tauri build
```

For v0.1.4, publish only the filenames and SHA-256 values recorded in `releases/v0.1.4/macos/SHA256SUMS.txt`.
