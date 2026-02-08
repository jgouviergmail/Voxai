/**
 * Build both CPU and CUDA distributions of Voxai in a single command.
 *
 * Usage:
 *   node scripts/build-all.mjs              # Build both CPU + CUDA installers
 *   node scripts/build-all.mjs --cpu-only   # Build CPU installer only
 *   node scripts/build-all.mjs --cuda-only  # Build CUDA installer only
 *
 * Output:
 *   src-tauri/target/release/dist/Voxai_<ver>_CPU_x64-setup.exe
 *   src-tauri/target/release/dist/Voxai_<ver>_CUDA_x64-setup.exe
 */

import { execSync } from "child_process";
import {
  copyFileSync,
  mkdirSync,
  existsSync,
  readdirSync,
  statSync,
  readFileSync,
} from "fs";
import { join } from "path";
import { ensureCargo, ensureCmake, ensureLibclang, ensureCudaPath } from "./env-setup.mjs";

// ---------------------------------------------------------------------------
// CLI flags
// ---------------------------------------------------------------------------
const args = process.argv.slice(2);
const cpuOnly = args.includes("--cpu-only");
const cudaOnly = args.includes("--cuda-only");

if (cpuOnly && cudaOnly) {
  console.error("ERROR: --cpu-only and --cuda-only are mutually exclusive.");
  process.exit(1);
}

const buildCpu = !cudaOnly;
const buildCuda = !cpuOnly;

// ---------------------------------------------------------------------------
// Read version from tauri.conf.json
// ---------------------------------------------------------------------------
const tauriConf = JSON.parse(
  readFileSync(join("src-tauri", "tauri.conf.json"), "utf-8"),
);
const version = tauriConf.version;
console.log(`\nVoxai v${version} — Build All\n${"=".repeat(40)}`);

// ---------------------------------------------------------------------------
// Environment auto-detection
// ---------------------------------------------------------------------------
ensureCargo();
ensureCmake();
ensureLibclang();
if (buildCuda) {
  ensureCudaPath();
}

// ---------------------------------------------------------------------------
// Output directory
// ---------------------------------------------------------------------------
const distDir = join("src-tauri", "target", "release", "dist");
mkdirSync(distDir, { recursive: true });

// NSIS installer produced by Tauri (conventional path)
const nsisDir = join("src-tauri", "target", "release", "bundle", "nsis");
const nsisName = `Voxai_${version}_x64-setup.exe`;

const results = [];

// ---------------------------------------------------------------------------
// Phase 1: CPU build
// ---------------------------------------------------------------------------
if (buildCpu) {
  console.log(`\n[1/2] Building CPU variant…`);
  runTauriBuild([]);
  const dest = join(distDir, `Voxai_${version}_CPU_x64-setup.exe`);
  copyInstaller(nsisDir, nsisName, dest);
  results.push(dest);
}

// ---------------------------------------------------------------------------
// Phase 2: CUDA build
// ---------------------------------------------------------------------------
if (buildCuda) {
  const step = buildCpu ? "2/2" : "1/1";
  console.log(`\n[${step}] Building CUDA variant…`);
  runTauriBuild(["--features", "cuda"]);
  const dest = join(distDir, `Voxai_${version}_CUDA_x64-setup.exe`);
  copyInstaller(nsisDir, nsisName, dest);
  results.push(dest);
}

// ---------------------------------------------------------------------------
// Summary
// ---------------------------------------------------------------------------
console.log(`\n${"=".repeat(40)}`);
console.log("Build complete!\n");
for (const f of results) {
  const size = (statSync(f).size / (1024 * 1024)).toFixed(1);
  console.log(`  ${f}  (${size} MB)`);
}
console.log();

// ===========================================================================
// Helpers
// ===========================================================================

function runTauriBuild(extraArgs) {
  const cmd = ["npx", "tauri", "build", ...extraArgs].join(" ");
  console.log(`> ${cmd}`);
  execSync(cmd, { stdio: "inherit", env: process.env });
}

function copyInstaller(srcDir, srcName, destPath) {
  const src = join(srcDir, srcName);
  if (!existsSync(src)) {
    // Tauri may use a slightly different naming — try to find it
    const candidates = existsSync(srcDir)
      ? readdirSync(srcDir).filter((f) => f.endsWith("-setup.exe"))
      : [];
    if (candidates.length === 1) {
      const fallback = join(srcDir, candidates[0]);
      console.log(`  Installer found as ${candidates[0]}`);
      copyFileSync(fallback, destPath);
      console.log(`  -> ${destPath}`);
      return;
    }
    console.error(`ERROR: Installer not found at ${src}`);
    if (candidates.length > 1) {
      console.error(`  Ambiguous candidates: ${candidates.join(", ")}`);
    }
    process.exit(1);
  }
  copyFileSync(src, destPath);
  console.log(`  -> ${destPath}`);
}
