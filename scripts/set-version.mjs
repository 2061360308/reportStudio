import {
  getCargoPackageVersion,
  parseArgs,
  parseSemver,
  readAppVersion,
  readJson,
  setAppVersion,
  setTemplateVersion,
} from "./version-utils.mjs";
import path from "node:path";
import { ROOT } from "./version-utils.mjs";

const args = parseArgs(process.argv.slice(2));
const version = args._[0];

if (!version) {
  console.error("用法：");
  console.error("  node scripts/set-version.mjs <version>                  # 更新应用版本");
  console.error("  node scripts/set-version.mjs --template <id> <version>   # 更新模板版本");
  process.exit(1);
}

try {
  parseSemver(version);
} catch (error) {
  console.error(String(error.message ?? error));
  process.exit(1);
}

try {
  if (args.template) {
    const result = setTemplateVersion(args.template, version);
    console.log(`模板 ${result.id} 版本已更新为 ${result.version}`);
    if (result.changelogCreated) {
      console.log(`已插入空的 CHANGELOG 段落：## ${result.version}`);
    } else {
      console.log(`CHANGELOG 已存在 ${result.version} 段落，未修改`);
    }
  } else {
    const previous = readAppVersion();
    const result = setAppVersion(version);
    console.log(`应用版本已更新：${previous} -> ${result.version}`);
    console.log("已同步：VERSION / package.json / package-lock.json / tauri.conf.json / Cargo.toml");
    if (result.changelogCreated) {
      console.log(`已插入空的 CHANGELOG 段落：## ${result.version}`);
    } else {
      console.log(`CHANGELOG 已存在 ${result.version} 段落，未修改`);
    }

    const tauriVersion = readJson(path.join(ROOT, "src-tauri", "tauri.conf.json")).version;
    const pkgVersion = readJson(path.join(ROOT, "package.json")).version;
    const cargoVersion = getCargoPackageVersion();
    const versions = { tauriVersion, pkgVersion, cargoVersion };
    for (const [name, value] of Object.entries(versions)) {
      if (value !== result.version) {
        throw new Error(`${name} 同步失败：期望 ${result.version}，实际 ${value}`);
      }
    }
  }
} catch (error) {
  console.error(String(error.message ?? error));
  process.exit(1);
}
