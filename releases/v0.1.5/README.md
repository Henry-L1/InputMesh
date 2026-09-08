# InputMesh v0.1.5 release manifest

This directory records the matched Windows x64 and macOS arm64 artifacts built
for the second-stage bug fixes. Each platform has its own checksum file and the
four packages are tied together by `manifest.json` under the single v0.1.5 tag.

Run `pnpm release:check` before publishing. These binaries remain unsigned
preview packages: Windows has no Authenticode signature and macOS uses an
ad-hoc signature without notarization.
