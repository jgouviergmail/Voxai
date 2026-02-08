/**
 * Shared environment auto-detection for Voxai build scripts.
 *
 * Auto-detects and injects into process.env:
 *   - cargo (adds ~/.cargo/bin to PATH)
 *   - cmake (adds VS/CMake to PATH if needed)
 *   - LIBCLANG_PATH (scans common LLVM/libclang locations)
 *   - CUDA_PATH (scans NVIDIA GPU Computing Toolkit)
 */

import { execSync } from "child_process";
import { existsSync, readdirSync } from "fs";
import { join } from "path";

/**
 * Ensure cargo is in PATH. Adds ~/.cargo/bin if needed.
 */
export function ensureCargo() {
  try {
    execSync("cargo --version", { stdio: "ignore" });
    return;
  } catch {
    // Not in PATH — try to find it
  }

  const home = process.env.USERPROFILE || process.env.HOME;
  if (!home) {
    console.error("ERROR: Cannot determine home directory for cargo.");
    process.exit(1);
  }

  const cargoBin = join(home, ".cargo", "bin");
  const cargoExe = join(
    cargoBin,
    process.platform === "win32" ? "cargo.exe" : "cargo",
  );

  if (existsSync(cargoExe)) {
    process.env.PATH = `${cargoBin};${process.env.PATH}`;
    console.log(`Added cargo to PATH: ${cargoBin}`);
    return;
  }

  console.error("ERROR: cargo not found. Install Rust: https://rustup.rs/");
  process.exit(1);
}

/**
 * Ensure cmake is in PATH. Scans Visual Studio installations if needed.
 */
export function ensureCmake() {
  try {
    execSync("cmake --version", { stdio: "ignore" });
    return;
  } catch {
    // Not in PATH — try to find it
  }

  const candidates = scanDirs(
    ["C:\\Program Files\\Microsoft Visual Studio", "C:\\Program Files (x86)\\Microsoft Visual Studio"],
    (base) => {
      const paths = [];
      for (const year of safeDirDesc(base)) {
        for (const edition of ["BuildTools", "Enterprise", "Professional", "Community"]) {
          paths.push(join(base, year, edition, "Common7", "IDE", "CommonExtensions", "Microsoft", "CMake", "CMake", "bin"));
        }
      }
      return paths;
    },
  );

  // Also check standalone CMake install
  candidates.push("C:\\Program Files\\CMake\\bin");

  for (const dir of candidates) {
    if (existsSync(join(dir, "cmake.exe"))) {
      process.env.PATH = `${dir};${process.env.PATH}`;
      console.log(`Added cmake to PATH: ${dir}`);
      return;
    }
  }

  console.error(
    "ERROR: cmake not found. Install CMake (https://cmake.org/) or Visual Studio Build Tools.",
  );
  process.exit(1);
}

/**
 * Ensure LIBCLANG_PATH is set. Scans common locations if not.
 */
export function ensureLibclang() {
  if (process.env.LIBCLANG_PATH) return;

  const candidates = [
    // LLVM system install
    "C:\\Program Files\\LLVM\\bin",
    "C:\\Program Files (x86)\\LLVM\\bin",
    // Visual Studio bundled LLVM
    ...scanDirs(
      ["C:\\Program Files\\Microsoft Visual Studio", "C:\\Program Files (x86)\\Microsoft Visual Studio"],
      (base) => {
        const paths = [];
        for (const year of safeDirDesc(base)) {
          for (const edition of ["BuildTools", "Enterprise", "Professional", "Community"]) {
            paths.push(join(base, year, edition, "VC", "Tools", "Llvm", "x64", "bin"));
          }
        }
        return paths;
      },
    ),
    // Unity bundled LLVM (fallback)
    ...scanDirs(
      ["C:\\Program Files\\Unity\\Hub\\Editor"],
      (base) =>
        safeDirDesc(base).map((ver) =>
          join(base, ver, "Editor", "Data", "PlaybackEngines", "WebGLSupport", "BuildTools", "Emscripten", "llvm"),
        ),
    ),
  ];

  for (const dir of candidates) {
    if (existsSync(join(dir, "libclang.dll")) || existsSync(join(dir, "clang.dll"))) {
      process.env.LIBCLANG_PATH = dir;
      console.log(`Auto-detected LIBCLANG_PATH: ${dir}`);
      return;
    }
  }

  console.error(
    "ERROR: LIBCLANG_PATH not set and libclang.dll not found.\n" +
      "Install LLVM (https://releases.llvm.org/) or set LIBCLANG_PATH manually.",
  );
  process.exit(1);
}

/**
 * Ensure CUDA_PATH is set. Scans NVIDIA toolkit directory if not.
 */
export function ensureCudaPath() {
  if (process.env.CUDA_PATH) return;

  const cudaBase = "C:\\Program Files\\NVIDIA GPU Computing Toolkit\\CUDA";
  if (existsSync(cudaBase)) {
    const versions = readdirSync(cudaBase)
      .filter((d) => d.startsWith("v"))
      .sort()
      .reverse();
    if (versions.length > 0) {
      process.env.CUDA_PATH = join(cudaBase, versions[0]);
      console.log(`Auto-detected CUDA_PATH: ${process.env.CUDA_PATH}`);
      return;
    }
  }

  console.error(
    "ERROR: CUDA_PATH not set and CUDA Toolkit not found.\n" +
      "Install CUDA Toolkit: https://developer.nvidia.com/cuda-downloads",
  );
  process.exit(1);
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/** Read directory entries sorted descending. Returns [] on error. */
function safeDirDesc(dir) {
  try {
    return existsSync(dir) ? readdirSync(dir).sort().reverse() : [];
  } catch {
    return [];
  }
}

/** For each existing base dir, call fn to produce candidate paths. */
function scanDirs(bases, fn) {
  return bases.filter(existsSync).flatMap(fn);
}
