/**
 * Run `tauri build` or `tauri dev` with proper environment.
 *
 * The voxai-llm-worker binary is built automatically by build.rs with
 * matching feature flags. CUDA DLLs are bundled by build.rs when the
 * cuda feature is enabled.
 *
 * Usage:
 *   node scripts/prepare-worker.mjs                # CPU release build
 *   node scripts/prepare-worker.mjs --cuda         # CUDA release build
 *   node scripts/prepare-worker.mjs --cuda --tauri # (legacy) same as --cuda
 *   node scripts/prepare-worker.mjs --dev          # Debug build (tauri dev)
 *
 * Prefer `npm run build:all` for producing both CPU + CUDA installers.
 */

import { execSync } from "child_process";
import { ensureCargo, ensureCmake, ensureLibclang, ensureCudaPath } from "./env-setup.mjs";

const args = process.argv.slice(2);
const cuda = args.includes("--cuda");
const dev = args.includes("--dev");

// Environment auto-detection
ensureCargo();
ensureCmake();
ensureLibclang();
if (cuda) {
  ensureCudaPath();
}

const featuresFlag = cuda ? " --features cuda" : "";

if (dev) {
  console.log(`Running tauri dev${featuresFlag}...`);
  execSync(`npx tauri dev${featuresFlag}`, {
    stdio: "inherit",
    env: process.env,
  });
} else {
  console.log(`Running tauri build${featuresFlag}...`);
  execSync(`npx tauri build${featuresFlag}`, {
    stdio: "inherit",
    env: process.env,
  });
}
