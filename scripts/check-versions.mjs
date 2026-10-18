import fs from "node:fs";
import path from "node:path";
import {
  ROOT,
  changelogSection,
  compareSemver,
  getCargoPackageVersion,
  parseArgs,
  parseSemver,
  readAllTemplates,
  readAppVersion,
  readJson,
  readTemplateNotes,
  writeJson,
  writeText,
} from "./version-utils.mjs";

const args = parseArgs(process.argv.slice(2));
const previousPath = args.prev ? path.resolve(args.prev) : null;
const outPath = path.resolve(args.out ?? path.join(ROOT, "release-plan.json"));
const notesPath = path.resolve(args.notesOut ?? path.join(ROOT, "release-notes.md"));

function fail(message) {
  console.error(`[check-versions] ${message}`);
  process.exit(1);
}

let appVersion;
try {
  appVersion = readAppVersion();
} catch (error) {
  fail(String(error.message ?? error));
}

const tauriVersion = readJson(path.join(ROOT, "src-tauri", "tauri.conf.json")).version;
const packageVersion = readJson(path.join(ROOT, "package.json")).version;
const cargoVersion = getCargoPackageVersion();

const mismatches = [
  ["src-tauri/tauri.conf.json", tauriVersion],
  ["package.json", packageVersion],
  ["src-tauri/Cargo.toml", cargoVersion],
].filter(([, value]) => String(value).replace(/^v/, "") !== appVersion);

if (mismatches.length) {
  fail(
    `版本号不一致：VERSION=${appVersion}；${mismatches
      .map(([file, value]) => `${file}=${value}`)
      .join("；")}。请运行 node scripts/set-version.mjs ${appVersion}`,
  );
}

let previous = null;
if (previousPath && fs.existsSync(previousPath)) {
  try {
    previous = readJson(previousPath);
  } catch (error) {
    fail(`无法解析上一轮 latest.json：${error.message ?? error}`);
  }
}

const previousVersion = previous?.version ? parseSemver(previous.version).raw : null;
let appChanged = previousVersion === null;
if (previousVersion !== null) {
  const cmp = compareSemver(appVersion, previousVersion);
  if (cmp < 0) {
    fail(`应用版本回退：上一轮 ${previousVersion}，当前 ${appVersion}`);
  }
  appChanged = cmp > 0;
}

const previousTemplates = new Map(
  (Array.isArray(previous?.templates) ? previous.templates : []).map((item) => [item.id, item]),
);

const allTemplates = readAllTemplates();
const templatesToPack = [];
const templatesRemoved = [];

for (const { dir, manifest } of allTemplates) {
  if (!manifest.id) fail(`模板缺少 id：${dir}`);
  if (!manifest.name) fail(`模板 ${manifest.id} 缺少 name`);

  let currentVersion;
  try {
    currentVersion = parseSemver(manifest.version).raw;
  } catch (error) {
    fail(`模板 ${manifest.id} 的版本号无效：${error.message ?? error}`);
  }

  const prev = previousTemplates.get(manifest.id);
  if (!prev) {
    templatesToPack.push({
      id: manifest.id,
      name: manifest.name,
      version: currentVersion,
      minAppVersion: manifest.minAppVersion ?? "0.0.0",
      notes: readTemplateNotes(dir, currentVersion),
    });
    continue;
  }

  let prevVersion;
  try {
    prevVersion = parseSemver(prev.version).raw;
  } catch (error) {
    fail(`上一轮模板 ${manifest.id} 的版本号无效：${error.message ?? error}`);
  }

  const cmp = compareSemver(currentVersion, prevVersion);
  if (cmp < 0) {
    fail(`模板版本回退：${manifest.id} 上一轮 ${prevVersion}，当前 ${currentVersion}`);
  }
  if (cmp > 0) {
    templatesToPack.push({
      id: manifest.id,
      name: manifest.name,
      version: currentVersion,
      minAppVersion: manifest.minAppVersion ?? "0.0.0",
      notes: readTemplateNotes(dir, currentVersion),
    });
  }
}

const currentIds = new Set(allTemplates.map(({ manifest }) => manifest.id));
for (const id of previousTemplates.keys()) {
  if (!currentIds.has(id)) templatesRemoved.push(id);
}

const hasChanges = appChanged || templatesToPack.length > 0 || templatesRemoved.length > 0;

let appNotes = "更新说明暂无";
const appChangelog = path.join(ROOT, "CHANGELOG.md");
if (fs.existsSync(appChangelog)) {
  appNotes = changelogSection(fs.readFileSync(appChangelog, "utf8"), appVersion);
}

const plan = {
  appVersion,
  previousVersion,
  appChanged,
  appNotes,
  hasChanges,
  templatesToPack,
  templatesRemoved,
  allTemplateIds: [...currentIds].sort(),
};

writeJson(outPath, plan);

const notes = [`# 报告工坊 v${appVersion}`, ""];
notes.push("## 应用");
notes.push(appNotes, "");
if (templatesToPack.length) {
  notes.push("## 模板");
  for (const item of templatesToPack) {
    notes.push(`### ${item.name} v${item.version}`);
    notes.push(item.notes || "更新说明暂无", "");
  }
}
if (templatesRemoved.length) {
  notes.push("## 已移除模板");
  notes.push(...templatesRemoved.map((id) => `- ${id}`));
}
writeText(notesPath, `${notes.join("\n").trim()}\n`);

if (process.env.GITHUB_OUTPUT) {
  const lines = [
    `app_changed=${appChanged}`,
    `app_version=${appVersion}`,
    `template_ids=${templatesToPack.map((item) => item.id).join(",")}`,
    `has_changes=${hasChanges}`,
  ];
  fs.appendFileSync(process.env.GITHUB_OUTPUT, `${lines.join("\n")}\n`, "utf8");
}

console.log(`应用版本：${appVersion}${previousVersion ? `（上一轮 ${previousVersion}）` : "（首次发布）"}`);
console.log(`应用发布：${appChanged ? "是" : "否"}`);
console.log(
  `模板发布：${
    templatesToPack.length
      ? templatesToPack.map((item) => `${item.id}@${item.version}`).join(", ")
      : "无"
  }`,
);
console.log(`模板移除：${templatesRemoved.length ? templatesRemoved.join(", ") : "无"}`);
console.log(`发布计划：${outPath}`);
console.log(`发布说明：${notesPath}`);
