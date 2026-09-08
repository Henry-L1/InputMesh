# Contributing

By submitting a contribution, you certify that you have the right to submit it
and agree to license it under the repository's PolyForm Noncommercial License
1.0.0. Do not copy code from GPL or other incompatible sources. Preserve all
third-party copyright and license notices.

Use a feature branch and open a pull request. Changes to `main` are expected to
pass CI and receive review before merge.

Run the complete local checks before submitting changes:

```bash
pnpm install --frozen-lockfile
pnpm test
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

Platform input changes require both Windows and macOS review. Never weaken pairing, frame-size limits, permission checks, injected-event filtering or the emergency release path solely to make a demo work.
