#!/usr/bin/env node

const { execSync, spawn } = require("child_process");
const fs = require("fs");
const path = require("path");
const https = require("https");
const os = require("os");
const zlib = require("zlib");

const PACKAGE_VERSION = require("./package.json").version;
const REPO = "Rika-Labs/repo-lint";
const BIN_NAME = process.platform === "win32" ? "repo-lint.exe" : "repo-lint";
const BIN_DIR = path.join(__dirname, "bin");
const BIN_PATH = path.join(BIN_DIR, BIN_NAME);

function getPlatformInfo() {
  const platform = os.platform();
  const arch = os.arch();

  const platformMap = {
    darwin: {
      x64: "repo-lint-darwin-x64",
      arm64: "repo-lint-darwin-arm64",
    },
    linux: {
      x64: "repo-lint-linux-x64",
      arm64: "repo-lint-linux-arm64",
    },
    win32: {
      x64: "repo-lint-windows-x64",
    },
  };

  const archMap = platformMap[platform];
  if (!archMap) {
    throw new Error(`Unsupported platform: ${platform}`);
  }

  const binaryName = archMap[arch];
  if (!binaryName) {
    throw new Error(`Unsupported architecture: ${arch} on ${platform}`);
  }

  const ext = platform === "win32" ? ".zip" : ".tar.gz";
  return { binaryName, ext, platform };
}

function downloadFile(url) {
  return new Promise((resolve, reject) => {
    const handleResponse = (response) => {
      if (response.statusCode === 302 || response.statusCode === 301) {
        https.get(response.headers.location, handleResponse).on("error", reject);
        return;
      }

      if (response.statusCode !== 200) {
        reject(new Error(`Failed to download: ${response.statusCode}`));
        return;
      }

      const chunks = [];
      response.on("data", (chunk) => chunks.push(chunk));
      response.on("end", () => resolve(Buffer.concat(chunks)));
      response.on("error", reject);
    };

    https.get(url, handleResponse).on("error", reject);
  });
}

async function extractTarGz(buffer, destDir) {
  const tar = require("tar");
  const tmpFile = path.join(os.tmpdir(), `repo-lint-${Date.now()}.tar.gz`);
  
  fs.writeFileSync(tmpFile, buffer);
  
  await tar.extract({
    file: tmpFile,
    cwd: destDir,
  });
  
  fs.unlinkSync(tmpFile);
}

async function extractZip(buffer, destDir) {
  const AdmZip = require("adm-zip");
  const zip = new AdmZip(buffer);
  zip.extractAllTo(destDir, true);
}

async function install() {
  try {
    const { binaryName, ext, platform } = getPlatformInfo();
    const version = `v${PACKAGE_VERSION}`;
    const assetName = `${binaryName}${ext}`;
    const url = `https://github.com/${REPO}/releases/download/${version}/${assetName}`;

    console.log(`Downloading repo-lint ${version} for ${platform}...`);

    if (!fs.existsSync(BIN_DIR)) {
      fs.mkdirSync(BIN_DIR, { recursive: true });
    }

    const buffer = await downloadFile(url);

    console.log("Extracting...");

    if (ext === ".tar.gz") {
      await extractTarGz(buffer, BIN_DIR);
    } else {
      await extractZip(buffer, BIN_DIR);
    }

    if (platform !== "win32") {
      fs.chmodSync(BIN_PATH, 0o755);
    }

    console.log(`repo-lint installed successfully to ${BIN_PATH}`);
  } catch (error) {
    console.error("Failed to install repo-lint binary:", error.message);
    console.error("");
    console.error("You can install manually:");
    console.error("  - Download from: https://github.com/Rika-Labs/repo-lint/releases");
    console.error("  - Or install via cargo: cargo install repo-lint");
    process.exit(1);
  }
}

install();
