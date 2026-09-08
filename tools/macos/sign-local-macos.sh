#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
  echo "usage: $0 /absolute/path/to/InputMesh.app" >&2
  exit 2
fi

app_path=$1
case "$app_path" in
  /*/InputMesh.app) ;;
  *) echo "refusing unexpected app path: $app_path" >&2; exit 2 ;;
esac

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
entitlements="$script_dir/../../src-tauri/Entitlements.plist"

# A normal ad-hoc signature uses the changing binary CDHash as its designated
# requirement, which invalidates macOS TCC grants after every development
# build. This local-development requirement stays stable across builds. Public
# releases must replace it with Developer ID signing and Apple notarization.
codesign --force --deep \
  --entitlements "$entitlements" \
  --requirements '=designated => identifier "com.inputmesh.desktop"' \
  --sign - \
  "$app_path"

codesign --verify --deep --strict --verbose=2 "$app_path"
codesign -d -r- "$app_path" 2>&1
