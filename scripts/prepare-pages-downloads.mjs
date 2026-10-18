import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { ROOT, parseArgs } from "./version-utils.mjs";

const DEFAULT_PAGES_BASE_URL = "https://oilu.cn/reportStudio/";

const args = parseArgs(process.argv.slice(2));
const docsDir = path.resolve(args.docs ?? path.join(ROOT, "docs"));
const latestPath = args.latest ? path.resolve(args.latest) : null;
const repo = args.repo ?? process.env.GITHUB_REPOSITORY ?? "2061360308/reportStudio";
const downloadsDir = path.join(docsDir, "downloads");
const indexJsonPath = path.join(downloadsDir, "index.json");
const latestJsonPath = path.join(downloadsDir, "latest.json");
const releaseUrl = `https://github.com/${repo}/releases`;

function normalizeBaseUrl(value) {
  const raw = String(value || DEFAULT_PAGES_BASE_URL).trim();
  const withScheme = /^https?:\/\//i.test(raw) ? raw : `https://${raw}`;
  const url = new URL(withScheme);
  const local = url.hostname === "localhost" || url.hostname === "127.0.0.1";
  if (url.protocol !== "https:" && !local) {
    throw new Error("Pages 下载地址必须使用 HTTPS");
  }
  return url.href.endsWith("/") ? url.href : `${url.href}/`;
}

function pagesAssetUrl(file) {
  return new URL(`downloads/${file.replace(/^\/+/, "")}`, pagesBaseUrl).href;
}

const pagesBaseUrl = normalizeBaseUrl(
  args["base-url"] ?? process.env.PAGES_BASE_URL ?? DEFAULT_PAGES_BASE_URL,
);

function safeName(value) {
  return (
    String(value ?? "")
      .trim()
      .replace(/[^0-9A-Za-z._-]+/g, "_")
      .replace(/^_+|_+$/g, "") || "unknown"
  );
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function download(url, destination) {
  let lastError;
  for (let attempt = 1; attempt <= 3; attempt += 1) {
    try {
      const headers = {
        "User-Agent": "report-studio-pages",
        Accept: "application/octet-stream",
      };
      if (process.env.GITHUB_TOKEN && url.startsWith("https://github.com/")) {
        headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
      }
      const response = await fetch(url, { headers, redirect: "follow" });
      if (!response.ok) {
        throw new Error(`HTTP ${response.status} ${response.statusText}`);
      }
      const buffer = Buffer.from(await response.arrayBuffer());
      if (!buffer.length) throw new Error("下载内容为空");
      fs.writeFileSync(destination, buffer);
      return buffer;
    } catch (error) {
      lastError = error;
      if (attempt < 3) await sleep(attempt * 1000);
    }
  }
  throw new Error(`下载失败：${url}（${lastError?.message ?? "未知错误"}）`);
}

function platformLabel(key) {
  const parts = String(key).split("-");
  const os = parts[0];
  const arch = parts[1];
  const kind = parts.slice(2).join("-");

  const osLabel =
    { windows: "Windows", darwin: "macOS", linux: "Linux" }[os] ?? os;
  const archLabel =
    { x86_64: "x64", aarch64: "ARM64", arm64: "ARM64", i686: "32-bit" }[arch] ??
    arch;
  const kindLabel =
    { nsis: "", msi: "MSI", appimage: "AppImage", dmg: "DMG" }[kind] ?? kind;

  return [osLabel, archLabel, kindLabel].filter(Boolean).join(" ");
}

function platformExtension(url) {
  try {
    const ext = path.extname(new URL(url).pathname);
    return ext || ".bin";
  } catch {
    return ".bin";
  }
}

async function main() {
  fs.rmSync(downloadsDir, { recursive: true, force: true });
  fs.mkdirSync(downloadsDir, { recursive: true });

  if (!latestPath || !fs.existsSync(latestPath)) {
    console.log("未找到 latest.json，跳过下载资源");
    return;
  }

  const latest = JSON.parse(
    fs.readFileSync(latestPath, "utf8").replace(/^\uFEFF/, ""),
  );
  const version = String(latest.version ?? "0.0.0");
  const platforms = latest.platforms ?? {};
  const platformEntries = [];
  const downloadedFiles = new Map();
  const seenUrls = new Set();

  for (const [key, platform] of Object.entries(platforms)) {
    if (!platform?.url || seenUrls.has(platform.url)) continue;
    seenUrls.add(platform.url);

    const ext = platformExtension(platform.url);
    const file = `report-studio_${safeName(version)}_${safeName(key)}${ext}`;
    await download(platform.url, path.join(downloadsDir, file));
    downloadedFiles.set(platform.url, file);
    platformEntries.push({
      key,
      label: platformLabel(key),
      os: String(key).split("-")[0] ?? "",
      arch: String(key).split("-")[1] ?? "",
      href: `downloads/${file}`,
    });
    console.log(`安装包：${file}`);
  }

  if (!platformEntries.length) {
    throw new Error("latest.json 中找不到可用的安装包地址");
  }

  const archRank = (arch) =>
    arch === "x86_64" ? 0 : arch === "aarch64" ? 1 : 2;
  platformEntries.sort(
    (a, b) => archRank(a.arch) - archRank(b.arch) || a.key.localeCompare(b.key),
  );

  const templates = Array.isArray(latest.templates) ? latest.templates : [];
  const templateEntries = [];

  for (const template of templates) {
    if (!template?.url) {
      throw new Error(`模板 ${template?.id ?? "unknown"} 缺少下载地址`);
    }
    const id = safeName(template.id);
    const templateVersion = String(template.version ?? "1.0.0");
    const file = `${id}-${safeName(templateVersion)}.zip`;
    const buffer = await download(template.url, path.join(downloadsDir, file));
    downloadedFiles.set(template.url, file);

    if (template.sha256) {
      const actual = crypto.createHash("sha256").update(buffer).digest("hex");
      if (actual.toLowerCase() !== String(template.sha256).toLowerCase()) {
        throw new Error(`模板 ${id} 的 sha256 校验失败`);
      }
    }
    if (
      typeof template.size === "number" &&
      template.size > 0 &&
      buffer.length !== template.size
    ) {
      throw new Error(
        `模板 ${id} 大小不匹配：期望 ${template.size}，实际 ${buffer.length}`,
      );
    }

    templateEntries.push({
      id: template.id,
      name: template.name || template.id,
      version: templateVersion,
      notes: template.notes || "更新说明暂无",
      href: `downloads/${file}`,
    });
    console.log(`模板：${file}`);
  }

  const index = {
    version,
    notes: latest.notes ?? "",
    releases: releaseUrl,
    platforms: platformEntries,
    templates: templateEntries,
  };

  const mirroredLatest = JSON.parse(JSON.stringify(latest));
  for (const [key, platform] of Object.entries(mirroredLatest.platforms ?? {})) {
    if (!platform?.url) continue;
    const file = downloadedFiles.get(platform.url);
    if (!file) {
      throw new Error(`安装包 ${key} 未下载到 Pages，无法改写 latest.json`);
    }
    platform.url = pagesAssetUrl(file);
  }
  if (Array.isArray(mirroredLatest.templates)) {
    for (const template of mirroredLatest.templates) {
      if (!template?.url) continue;
      const file = downloadedFiles.get(template.url);
      if (!file) {
        throw new Error(`模板 ${template?.id ?? "unknown"} 未下载到 Pages`);
      }
      template.url = pagesAssetUrl(file);
    }
  }

  fs.writeFileSync(indexJsonPath, `${JSON.stringify(index, null, 2)}\n`, "utf8");
  fs.writeFileSync(latestJsonPath, `${JSON.stringify(mirroredLatest, null, 2)}\n`, "utf8");
  console.log(`下载索引：${indexJsonPath}`);
  console.log(`Pages 清单：${latestJsonPath}`);
  console.log(`Pages 地址：${pagesBaseUrl}`);
}

main().catch((error) => {
  console.error(`[prepare-pages-downloads] ${error.message ?? error}`);
  process.exit(1);
});
