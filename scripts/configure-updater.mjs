import fs from "node:fs";
import path from "node:path";
import { ROOT, parseArgs, readJson, writeJson } from "./version-utils.mjs";

const args = parseArgs(process.argv.slice(2));
const configPath = path.join(ROOT, "src-tauri", "tauri.conf.json");

let pubkey = args.pubkey;
if (args["pubkey-file"]) {
  pubkey = fs
    .readFileSync(path.resolve(args["pubkey-file"]), "utf8")
    .replace(/^\uFEFF/, "")
    .trim();
}
if (pubkey) pubkey = pubkey.replace(/^\uFEFF/, "").trim();

const endpoints = [
  ...new Set(
    (args.endpoints
      ? String(args.endpoints).split(",")
      : args.endpoint
        ? [String(args.endpoint)]
        : []
    )
      .map((value) => value.trim())
      .filter(Boolean),
  ),
];

if (!endpoints.length && !pubkey) {
  const config = readJson(configPath);
  const updater = config.plugins?.updater ?? {};
  console.log(`endpoints: ${JSON.stringify(updater.endpoints ?? [])}`);
  console.log(`pubkey:    ${updater.pubkey ?? ""}`);
  console.log("\n用法：");
  console.log(
    "  node scripts/configure-updater.mjs --endpoint <url> --pubkey-file <path>",
  );
  console.log(
    "  node scripts/configure-updater.mjs --endpoints <url1,url2> --pubkey <内容>",
  );
  process.exit(0);
}

if (!endpoints.length) {
  console.error("缺少 --endpoint 或 --endpoints");
  process.exit(1);
}
if (!pubkey) {
  console.error("缺少 --pubkey 或 --pubkey-file");
  process.exit(1);
}
for (const endpoint of endpoints) {
  if (!/^https:\/\//i.test(endpoint)) {
    console.error(`endpoint 必须使用 HTTPS：${endpoint}`);
    process.exit(1);
  }
}
if (pubkey.includes("REPLACE_WITH_YOUR_UPDATER_PUBLIC_KEY")) {
  console.error("pubkey 仍是占位符，请填入真实的公钥");
  process.exit(1);
}

const config = readJson(configPath);
config.plugins = config.plugins ?? {};
config.plugins.updater = config.plugins.updater ?? {};
config.plugins.updater.endpoints = endpoints;
config.plugins.updater.pubkey = pubkey;
config.plugins.updater.windows = config.plugins.updater.windows ?? { installMode: "passive" };
writeJson(configPath, config);

console.log("已更新 src-tauri/tauri.conf.json");
console.log(`endpoints: ${JSON.stringify(endpoints)}`);
console.log("pubkey:    已写入");
