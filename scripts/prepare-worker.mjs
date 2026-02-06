/**
 * Build the voxai-llm-worker binary and copy it to src-tauri/binaries/
 * for Tauri bundling.
 *
 * Usage:
 *   node scripts/prepare-worker.mjs           # Release build (CPU)
 *   node scripts/prepare-worker.mjs --cuda    # Release build (CUDA)
 *   node scripts/prepare-worker.mjs --dev     # Debug build (CPU)
 *   node scripts/prepare-worker.mjs --tauri   # Also run tauri build after worker
 */

import { execSync } from "child_process";
import { copyFileSync, mkdirSync, existsSync, readdirSync } from "fs";
import { join } from "path";

const args = process.argv.slice(2);
const cuda = args.includes("--cuda");
const dev = args.includes("--dev");
const tauri = args.includes("--tauri");

const profile = dev ? "debug" : "release";
const releaseFlag = dev ? "" : " --release";
const featuresFlag = cuda ? " --features cuda" : "";

const ext = process.platform === "win32" ? ".exe" : "";
const workerName = `voxai-llm-worker${ext}`;

// Ensure CUDA_PATH is set for CUDA builds
if (cuda && !process.env.CUDA_PATH) {
  // Auto-detect CUDA toolkit on Windows
  const cudaBase = "C:\\Program Files\\NVIDIA GPU Computing Toolkit\\CUDA";
  if (existsSync(cudaBase)) {
    const versions = readdirSync(cudaBase)
      .filter((d) => d.startsWith("v"))
      .sort()
      .reverse();
    if (versions.length > 0) {
      process.env.CUDA_PATH = join(cudaBase, versions[0]);
      console.log(`Auto-detected CUDA_PATH: ${process.env.CUDA_PATH}`);
    }
  }
  if (!process.env.CUDA_PATH) {
    console.error("ERROR: CUDA_PATH not set and CUDA toolkit not found.");
    process.exit(1);
  }
}

// Build the worker crate
console.log(`Building worker (${profile}${cuda ? " + CUDA" : ""})...`);
execSync(
  `cargo build --manifest-path src-tauri/Cargo.toml -p voxai-llm-worker${releaseFlag}${featuresFlag}`,
  { stdio: "inherit", env: process.env }
);

// Copy to src-tauri/binaries/ for Tauri bundle
const src = join("src-tauri", "target", profile, workerName);
const destDir = join("src-tauri", "binaries");
const dest = join(destDir, workerName);

if (!existsSync(src)) {
  console.error(`ERROR: Worker binary not found at ${src}`);
  process.exit(1);
}

mkdirSync(destDir, { recursive: true });
copyFileSync(src, dest);
console.log(`Copied ${src} -> ${dest}`);

// Optionally run tauri build (keeps CUDA_PATH set in env)
if (tauri) {
  console.log(`\nRunning tauri build${featuresFlag}...`);
  execSync(`npx tauri build${featuresFlag}`, {
    stdio: "inherit",
    env: process.env,
  });
}
