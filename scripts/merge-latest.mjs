import fs from "node:fs";
import path from "node:path";
import {
  ROOT,
  parseArgs,
  parseSemver,
  readJson,
  readText,
  writeJson,
} from "./version-utils.mjs";

const args = parseArgs(process.argv.slice(2));
const previousPath = args.prev ? path.resolve(args.prev) : null;
const planPath = path.resolve(args.plan ?? path.join(ROOT, "release-plan.json"));
const distDir = path.resolve(args.dist ?? path.join(ROOT, "dist"));
const outPath = path.resolve(args.out ?? path.join(distDir, "latest.json"));
const tag = args.tag ?? process.env.RELEASE_TAG;
const repo = args.repo ?? process.env.GITHUB_REPOSITORY;

function fail(message) {
  console.error(`[merge-latest] ${message}`);
  process.exit(1);
}

function walk(dir) {
  if (!fs.existsSync(dir)) return [];
  const result = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) result.push(...walk(full));
    else if (entry.isFile()) result.push(full);
  }
  return result;
}

if (!fs.existsSync(planPath)) fail(`找不到发布计划：${planPath}`);
if (!tag) fail("缺少 release tag（--tag 或 RELEASE_TAG）");
if (!repo) fail("缺少仓库名（--repo 或 GITHUB_REPOSITORY）");

const plan = readJson(planPath);
const previous = previousPath && fs.existsSync(previousPath) ? readJson(previousPath) : null;
const templatesToPack = new Map(plan.templatesToPack.map((item) => [item.id, item]));

function assetUrl(fileName) {
  return `https://github.com/${repo}/releases/download/${tag}/${encodeURIComponent(fileName)}`;
}

function artifactArch(file) {
  const name = path.basename(file).toLowerCase();
  if (name.includes("arm64") || name.includes("aarch64")) return "aarch64";
  if (name.includes("x64") || name.includes("x86_64") || name.includes("amd64")) {
    return "x86_64";
  }
  return null;
}

function findAppArtifacts() {
  const files = walk(distDir);
  const sigs = files.filter((file) => file.endsWith(".exe.sig"));
  const byArch = new Map();

  for (const sig of sigs) {
    const exe = sig.slice(0, -4);
    if (!fs.existsSync(exe)) continue;
    const arch = artifactArch(exe);
    if (!arch) continue;
    const current = byArch.get(arch);
    const score = (item) => (path.basename(item.exe).includes("setup") ? 1 : 0);
    if (!current || score({ exe }) > score(current)) {
      byArch.set(arch, { exe, sig });
    }
  }

  return byArch;
}

let appSection;
if (plan.appChanged) {
  const artifacts = findAppArtifacts();
  if (!artifacts.size) fail("应用有更新，但 dist 中找不到 .exe + .exe.sig");

  const platforms = {};
  for (const arch of ["x86_64", "aarch64"]) {
    const artifact = artifacts.get(arch);
    if (!artifact) {
      fail(`应用有更新，但 dist 中找不到 ${arch} 安装包`);
    }
    const exeName = path.basename(artifact.exe);
    const sigName = path.basename(artifact.sig);
    const signature = readText(artifact.sig).trim();
    if (!signature) fail(`${sigName} 内容为空`);

    const platform = { signature, url: assetUrl(exeName) };
    platforms[`windows-${arch}-nsis`] = { ...platform };
    platforms[`windows-${arch}`] = { ...platform };
    console.log(`应用资源（${arch}）：${exeName}`);
    console.log(`应用签名（${arch}）：${sigName}`);
  }

  appSection = {
    version: parseSemver(plan.appVersion).raw,
    notes: plan.appNotes || "更新说明暂无",
    pub_date: new Date().toISOString(),
    platforms,
  };
} else {
  if (!previous?.platforms) fail("应用未更新，但上一轮 latest.json 缺少 platforms");
  appSection = {
    version: previous.version,
    notes: previous.notes ?? "更新说明暂无",
    pub_date: previous.pub_date ?? new Date().toISOString(),
    platforms: previous.platforms,
  };
  console.log(`应用未更新，沿用 ${previous.version} 的安装包地址`);
}

const packedFile = walk(distDir).find((file) => path.basename(file) === "packed.json");
const packed = packedFile ? readJson(packedFile) : [];
const packedById = new Map(packed.map((item) => [item.id, item]));
const previousTemplates = new Map(
  (Array.isArray(previous?.templates) ? previous.templates : []).map((item) => [item.id, item]),
);
const now = new Date().toISOString();
const templateIds = Array.isArray(plan.allTemplateIds) ? plan.allTemplateIds : [];

const templates = [];
for (const id of templateIds) {
  const changed = templatesToPack.get(id);
  const packedItem = packedById.get(id);

  if (changed) {
    if (!packedItem) fail(`模板 ${id} 标记为更新，但 packed.json 中没有产物`);
    const expectedName = `${id}-${changed.version}.zip`;
    if (packedItem.file !== expectedName) {
      fail(`模板 ${id} 产物文件名异常：${packedItem.file}，期望 ${expectedName}`);
    }
    templates.push({
      id,
      name: changed.name,
      version: changed.version,
      notes: changed.notes || "更新说明暂无",
      url: assetUrl(packedItem.file),
      sha256: packedItem.sha256,
      size: packedItem.size,
      minAppVersion: changed.minAppVersion ?? "0.0.0",
      publishedAt: now,
    });
    console.log(`模板更新：${id} v${changed.version} -> ${packedItem.file}`);
    continue;
  }

  const prev = previousTemplates.get(id);
  if (!prev) fail(`模板 ${id} 既没有更新产物，也没有上一轮记录`);
  templates.push(prev);
  console.log(`模板沿用：${id} v${prev.version} -> ${prev.url}`);
}

for (const id of plan.templatesRemoved ?? []) {
  console.log(`模板移除：${id}（已从 latest.json 移除，不删除用户本地副本）`);
}

const latest = {
  ...appSection,
  schemaVersion: 1,
  generatedAt: now,
  release: tag,
  templates,
};

if (!latest.platforms || typeof latest.platforms !== "object" || Array.isArray(latest.platforms)) {
  fail("platforms 必须为对象");
}
if (!latest.platforms["windows-x86_64"] && !latest.platforms["windows-x86_64-nsis"]) {
  fail("platforms 缺少 windows-x86_64 或 windows-x86_64-nsis");
}
if (!latest.platforms["windows-aarch64"] && !latest.platforms["windows-aarch64-nsis"]) {
  if (plan.appChanged) {
    fail("platforms 缺少 windows-aarch64 或 windows-aarch64-nsis");
  }
  console.warn("上一轮 latest.json 缺少 ARM64 安装包，将在下一次应用发版时补齐");
}

writeJson(outPath, latest);
console.log(`latest.json 已生成：${outPath}`);
