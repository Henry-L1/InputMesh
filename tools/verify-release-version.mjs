import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const read = (path) => readFileSync(resolve(root, path), "utf8");
const packageVersion = JSON.parse(read("package.json")).version;
const tauriVersion = JSON.parse(read("src-tauri/tauri.conf.json")).version;
const cargoVersion = read("src-tauri/Cargo.toml").match(
  /^version\s*=\s*"([^"]+)"/m,
)?.[1];
const manifestPath = `releases/v${packageVersion}/manifest.json`;
const manifest = JSON.parse(read(manifestPath));
const draftManifest = manifest.status === "draft";

const failures = [];
for (const [source, version] of [
  ["package.json", packageVersion],
  ["src-tauri/tauri.conf.json", tauriVersion],
  ["src-tauri/Cargo.toml", cargoVersion],
  [manifestPath, manifest.version],
]) {
  if (version !== packageVersion) {
    failures.push(`${source} has version ${version ?? "missing"}, expected ${packageVersion}`);
  }
}

if (manifest.tag !== `v${packageVersion}`) {
  failures.push(`${manifestPath} has tag ${manifest.tag}, expected v${packageVersion}`);
}

for (const platform of ["windows", "macos"]) {
  const assets = manifest.platforms?.[platform]?.assets;
  if (!Array.isArray(assets) || assets.length === 0) {
    failures.push(`${manifestPath} has no ${platform} assets`);
    continue;
  }
  for (const asset of assets) {
    if (!asset.name.includes(`v${packageVersion}`)) {
      failures.push(`${asset.name} does not contain v${packageVersion}`);
    }
    const hasHash = typeof asset.sha256 === "string" && /^[a-f0-9]{64}$/.test(asset.sha256);
    if (!hasHash && !(draftManifest && asset.sha256 === null)) {
      failures.push(`${asset.name} has an invalid SHA-256`);
    }
  }
}

if (failures.length > 0) {
  console.error(failures.join("\n"));
  process.exit(1);
}

console.log(
  draftManifest
    ? `release v${packageVersion}: draft Windows and macOS manifest is version-aligned; artifact hashes are pending`
    : `release v${packageVersion}: Windows and macOS manifests are aligned`,
);
