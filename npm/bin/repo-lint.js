#!/usr/bin/env node

const { spawn, spawnSync } = require("child_process");
const path = require("path");
const fs = require("fs");

const binName = process.platform === "win32" ? "repo-lint.exe" : "repo-lint";
const binPath = path.join(__dirname, binName);

// If binary missing, try to install it (lazy install)
if (!fs.existsSync(binPath)) {
  console.error("repo-lint binary not found. Installing...");
  const installScript = path.join(__dirname, "..", "install.js");

  const result = spawnSync(process.execPath, [installScript], {
    stdio: "inherit",
    cwd: path.join(__dirname, ".."),
  });

  if (result.status !== 0 || !fs.existsSync(binPath)) {
    console.error("\nFailed to install repo-lint binary.");
    console.error("You can install manually:");
    console.error("  - Download from: https://github.com/Rika-Labs/repo-lint/releases");
    console.error("  - Or install via cargo: cargo install repo-lint");
    process.exit(1);
  }
}

const child = spawn(binPath, process.argv.slice(2), {
  stdio: "inherit",
  shell: false,
});

child.on("close", (code) => {
  process.exit(code ?? 0);
});
