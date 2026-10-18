import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
export const TEMPLATES_DIR = path.join(ROOT, "src-tauri", "templates");
export const VERSION_FILE = path.join(ROOT, "VERSION");

export function readText(file) {
  return fs.readFileSync(file, "utf8");
}

export function writeText(file, text) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, text.endsWith("\n") ? text : `${text}\n`, "utf8");
}

export function readJson(file) {
  return JSON.parse(readText(file));
}

export function writeJson(file, value) {
  writeText(file, `${JSON.stringify(value, null, 2)}\n`);
}

export function parseSemver(input) {
  const raw = String(input ?? "").trim();
  const value = raw.startsWith("v") ? raw.slice(1) : raw;
  const match =
    /^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?(?:\+[0-9A-Za-z.-]+)?$/.exec(value);
  if (!match) {
    throw new Error(`不是合法的 SemVer：${JSON.stringify(raw)}`);
  }
  return {
    raw: value,
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3]),
    prerelease: match[4] ? match[4].split(".") : [],
  };
}

function compareIdentifier(a, b) {
  const an = /^\d+$/.test(a);
  const bn = /^\d+$/.test(b);
  if (an && bn) return Math.sign(Number(a) - Number(b));
  if (an) return -1;
  if (bn) return 1;
  return a < b ? -1 : a > b ? 1 : 0;
}

export function compareSemver(a, b) {
  const left = typeof a === "string" ? parseSemver(a) : a;
  const right = typeof b === "string" ? parseSemver(b) : b;

  for (const key of ["major", "minor", "patch"]) {
    if (left[key] !== right[key]) return Math.sign(left[key] - right[key]);
  }

  if (!left.prerelease.length && !right.prerelease.length) return 0;
  if (!left.prerelease.length) return 1;
  if (!right.prerelease.length) return -1;

  const len = Math.max(left.prerelease.length, right.prerelease.length);
  for (let i = 0; i < len; i += 1) {
    const l = left.prerelease[i];
    const r = right.prerelease[i];
    if (l === undefined) return -1;
    if (r === undefined) return 1;
    const cmp = compareIdentifier(l, r);
    if (cmp !== 0) return cmp;
  }
  return 0;
}

export function readAppVersion() {
  return parseSemver(readText(VERSION_FILE)).raw;
}

export function getCargoPackageVersion() {
  const text = readText(path.join(ROOT, "src-tauri", "Cargo.toml"));
  const packageBlock = /\[package\]([\s\S]*?)(?:\n\[|$)/.exec(text);
  if (!packageBlock) throw new Error("src-tauri/Cargo.toml 中找不到 [package]");
  const version = /^\s*version\s*=\s*"([^"]+)"/m.exec(packageBlock[1]);
  if (!version) throw new Error("src-tauri/Cargo.toml 的 [package] 中找不到 version");
  return version[1];
}

export function setCargoPackageVersion(version) {
  const file = path.join(ROOT, "src-tauri", "Cargo.toml");
  const text = readText(file);
  const packageBlock = /(\[package\][\s\S]*?)(\n\[|$)/.exec(text);
  if (!packageBlock) throw new Error("src-tauri/Cargo.toml 中找不到 [package]");
  const updatedBlock = packageBlock[1].replace(
    /^(\s*version\s*=\s*")[^"]*(")/m,
    `$1${version}$2`,
  );
  writeText(file, text.replace(packageBlock[1], updatedBlock));
}

export function listTemplateDirs() {
  if (!fs.existsSync(TEMPLATES_DIR)) return [];
  return fs
    .readdirSync(TEMPLATES_DIR, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => path.join(TEMPLATES_DIR, entry.name))
    .filter((dir) => fs.existsSync(path.join(dir, "manifest.json")))
    .sort();
}

export function readTemplateManifest(dir) {
  const file = path.join(dir, "manifest.json");
  const manifest = readJson(file);
  return { dir, file, manifest };
}

export function readAllTemplates() {
  return listTemplateDirs().map(readTemplateManifest);
}

export function changelogSection(markdown, version, fallback = "更新说明暂无") {
  const escaped = version.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const heading = new RegExp(`^##\\s*\\[?v?${escaped}\\]?\\s*$`, "m");
  const match = heading.exec(markdown);
  if (!match) return fallback;
  const rest = markdown.slice(match.index + match[0].length);
  const next = /^##\s+/m.exec(rest);
  const body = (next ? rest.slice(0, next.index) : rest).trim();
  return body || fallback;
}

export function readTemplateNotes(dir, version) {
  const file = path.join(dir, "CHANGELOG.md");
  if (!fs.existsSync(file)) return "更新说明暂无";
  return changelogSection(readText(file), version);
}

export function ensureChangelogSection(file, version, title = "更新日志") {
  let markdown = fs.existsSync(file) ? readText(file) : `# ${title}\n`;
  const escaped = version.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const heading = new RegExp(`^##\\s*\\[?v?${escaped}\\]?\\s*$`, "m");
  if (heading.test(markdown)) return false;

  const titleMatch = /^#\s+.*$/m.exec(markdown);
  const section = `## ${version}\n`;
  if (titleMatch) {
    const at = titleMatch.index + titleMatch[0].length;
    markdown = `${markdown.slice(0, at)}\n\n${section}${markdown.slice(at).replace(/^\s*\n/, "\n")}`;
  } else {
    markdown = `# ${title}\n\n${section}${markdown}`;
  }
  writeText(file, markdown);
  return true;
}

export function setAppVersion(version) {
  const parsed = parseSemver(version);
  const next = parsed.raw;

  writeText(VERSION_FILE, next);

  const pkgFile = path.join(ROOT, "package.json");
  const pkg = readJson(pkgFile);
  pkg.version = next;
  writeJson(pkgFile, pkg);

  const lockFile = path.join(ROOT, "package-lock.json");
  if (fs.existsSync(lockFile)) {
    const lock = readJson(lockFile);
    lock.version = next;
    if (lock.packages && lock.packages[""]) {
      lock.packages[""].version = next;
    }
    writeJson(lockFile, lock);
  }

  const tauriFile = path.join(ROOT, "src-tauri", "tauri.conf.json");
  const tauri = readJson(tauriFile);
  tauri.version = next;
  writeJson(tauriFile, tauri);

  setCargoPackageVersion(next);

  const changelog = path.join(ROOT, "CHANGELOG.md");
  const created = ensureChangelogSection(changelog, next, "更新日志");

  return { version: next, changelogCreated: created };
}

export function setTemplateVersion(id, version) {
  const parsed = parseSemver(version);
  const dir = path.join(TEMPLATES_DIR, id);
  const manifestFile = path.join(dir, "manifest.json");
  if (!fs.existsSync(manifestFile)) {
    throw new Error(`模板不存在：${id}（${manifestFile}）`);
  }
  const manifest = readJson(manifestFile);
  if (manifest.version !== parsed.raw) {
    manifest.version = parsed.raw;
    writeJson(manifestFile, manifest);
  }

  const changelog = path.join(dir, "CHANGELOG.md");
  const created = ensureChangelogSection(changelog, parsed.raw, `${manifest.name} · 更新日志`);
  return { id, version: parsed.raw, changelogCreated: created };
}

export function parseArgs(argv) {
  const args = { _: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const item = argv[i];
    if (item === "--template") {
      args.template = argv[++i];
    } else if (item === "--prev") {
      args.prev = argv[++i];
    } else if (item === "--out") {
      args.out = argv[++i];
    } else if (item === "--dist") {
      args.dist = argv[++i];
    } else if (item === "--tag") {
      args.tag = argv[++i];
    } else if (item === "--repo") {
      args.repo = argv[++i];
    } else if (item === "--plan") {
      args.plan = argv[++i];
    } else if (item === "--notes-out") {
      args.notesOut = argv[++i];
    } else if (item.startsWith("--")) {
      const key = item.slice(2);
      const next = argv[i + 1];
      if (next !== undefined && !next.startsWith("--")) {
        args[key] = next;
        i += 1;
      } else {
        args[key] = true;
      }
    } else {
      args._.push(item);
    }
  }
  return args;
}
