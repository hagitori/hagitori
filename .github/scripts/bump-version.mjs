#!/usr/bin/env node

import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { execSync } from "node:child_process";

const version = process.argv[2];

if (!version) {
  console.error("Usage: pnpm bump <version>");
  console.error("Example: pnpm bump 1.2.0");
  process.exit(1);
}

// Validate version format
if (!/^\d+\.\d+\.\d+(-[\w.]+)?$/.test(version)) {
  console.error(`Invalid version format: "${version}"`);
  console.error("Expected: major.minor.patch");
  process.exit(1);
}

const root = resolve(import.meta.dirname, "../..");

const files = [
  {
    path: resolve(root, "src-tauri/tauri.conf.json"),
    label: "tauri.conf.json",
    update(content) {
      const json = JSON.parse(content);
      json.version = version;
      return JSON.stringify(json, null, 2) + "\n";
    },
  },
  {
    path: resolve(root, "package.json"),
    label: "package.json",
    update(content) {
      const json = JSON.parse(content);
      json.version = version;
      return JSON.stringify(json, null, 2) + "\n";
    },
  },
  {
    path: resolve(root, "src-tauri/Cargo.toml"),
    label: "Cargo.toml",
    update(content) {
      // replace only the workspace.package.version line
      return content.replace(
        /^(version\s*=\s*)"[^"]*"/m,
        `$1"${version}"`
      );
    },
  },
];

console.log(`\nBumping version to ${version}\n`);

for (const file of files) {
  const content = readFileSync(file.path, "utf-8");
  const updated = file.update(content);
  writeFileSync(file.path, updated);
  console.log(`${file.label}`);
}

console.log(`\nDone! Updating Cargo.lock...\n`);

try {
  execSync("cargo generate-lockfile", {
    cwd: resolve(root, "src-tauri"),
    stdio: "inherit",
  });
  console.log(`Cargo.lock\n`);
} catch {
  console.warn("Could not update Cargo.lock\n");
}
