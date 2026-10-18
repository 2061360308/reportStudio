# 发版与自动更新

本文说明报告工坊的版本规则、GitHub Release 发布流程、`latest.json` 的合并方式，以及
`scripts/` 下各脚本的用法。

---

## 1. 设计概览

- **版本号驱动发布**：应用看根目录 `VERSION`，模板看各自的 `manifest.json` 的 `version`。
- **应用与模板分开分发**：首次启动或启动时检查更新，从 `latest.json` 同步模板。
- **每轮只发布变动资源**：应用变了才构建安装包；哪个模板版本变了才打包哪个模板。
- **双 CPU 架构**：应用同时构建 Windows x64（`x86_64-pc-windows-msvc`）与 Windows
  ARM64（`aarch64-pc-windows-msvc`）两个安装包，共用同一个版本号与 Release。
- **单一 `latest.json`**：同时承载应用更新信息和模板列表。未变动的资源沿用上一轮 Release 的地址。
- **一个 Release 对应一轮发布**：无论本轮只发应用、只发模板，还是新增模板，所有本轮产物和
  `latest.json` 都放在同一个 Release 里。

客户端按顺序读取的更新清单地址：

```
https://oilu.cn/reportStudio/downloads/latest.json
https://github.com/2061360308/reportStudio/releases/latest/download/latest.json
```

优先使用 Pages 清单；Pages 不可用、返回非 2xx 或清单无法解析时，回退到 GitHub Release。

---

## 2. 版本与日志文件

| 文件 | 作用 |
| --- | --- |
| `VERSION` | 应用版本，唯一入口，例如 `0.1.0` |
| `src-tauri/tauri.conf.json` | 安装包版本，必须等于 `VERSION` |
| `package.json` / `package-lock.json` | 项目版本，必须等于 `VERSION` |
| `src-tauri/Cargo.toml` | Rust 包版本，必须等于 `VERSION` |
| `CHANGELOG.md` | 应用更新日志，工作流按 `## <VERSION>` 提取 |
| `src-tauri/templates/<id>/manifest.json` | 模板版本 `version`、最低应用版本 `minAppVersion` |
| `src-tauri/templates/<id>/CHANGELOG.md` | 模板更新日志，工作流按 `## <version>` 提取 |

规则：

- 版本号必须是合法 SemVer，不允许回退。
- 应用代码有改动但不提升 `VERSION`，**不会发布应用**。
- 模板内容有改动但不提升 `manifest.version`，**不会发布该模板**。
- 更新日志缺少对应版本段落时，发布说明使用「更新说明暂无」。
- 建议始终通过脚本修改版本，避免多处版本号不一致。

---

## 3. 首次配置

### 3.1 环境

- Node.js + npm
- Rust stable
- Tauri CLI（项目已通过 `@tauri-apps/cli` 安装）
- 可写的 GitHub 仓库

### 3.2 生成更新签名密钥

```powershell
npm run tauri signer generate -- -w $HOME\.tauri\report-studio.key
```

会生成：

- `report-studio.key`：私钥，**不要提交到仓库，务必离线备份**
- `report-studio.key.pub`：公钥，可公开

私钥丢失后，已经安装的旧版本将无法再接收新更新。

### 3.3 配置更新地址与公钥

```powershell
npm run updater:configure -- `
  --endpoints "https://oilu.cn/reportStudio/downloads/latest.json,https://github.com/2061360308/reportStudio/releases/latest/download/latest.json" `
  --pubkey-file "$HOME\.tauri\report-studio.key.pub"
```

不带参数运行可查看当前配置：

```powershell
npm run updater:configure
```

### 3.4 GitHub 仓库设置

在仓库 **Settings → Secrets and variables → Actions** 中添加：

| 类型 | 名称 | 内容 |
| --- | --- | --- |
| Secret | `TAURI_SIGNING_PRIVATE_KEY` | `report-studio.key` 文件内容或路径 |
| Secret | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 生成密钥时设置的密码（没有则留空） |
| Variable | `TAURI_UPDATER_PUBKEY` | `report-studio.key.pub` 文件内容 |
| Variable | `PAGES_CUSTOM_DOMAIN` | 可选，Pages 自定义域名，例如 `oilu.cn/reportStudio`；默认 `oilu.cn/reportStudio` |

注意：

- 更新源要求 **仓库或 Release 仓库公开可读**。私有仓库的 Release 附件需要鉴权，客户端无法匿名下载。
- 如果源码必须私有，请另建一个公开的 Release 仓库，并修改工作流中的上传目标和 endpoint。
- 旧 Release 的资产不要删除，未变资源的 `latest.json` 会继续指向它们。

---

## 4. 日常发版

### 4.1 发布应用

```powershell
# 1. 提升版本，并自动插入空的 CHANGELOG 段落
npm run version:set -- 0.1.1

# 2. 编辑 CHANGELOG.md 的 ## 0.1.1 段落
# 3. 提交并推送
git add VERSION package.json package-lock.json src-tauri/tauri.conf.json src-tauri/Cargo.toml CHANGELOG.md
git commit -m "chore(release): 0.1.1"
git push
```

CI 判断：

- `VERSION > 上一轮 latest.json 的 version` → 构建应用
- 模板版本未变 → 不重新打包模板，模板地址沿用上一轮

### 4.2 发布模板

```powershell
# 1. 修改模板内容
# 2. 提升模板版本，并自动插入空的 CHANGELOG 段落
npm run version:set -- --template settle-audit-report 1.0.1

# 3. 编辑 src-tauri/templates/settle-audit-report/CHANGELOG.md
# 4. 提交并推送
git add src-tauri/templates/settle-audit-report
git commit -m "feat(template): settle-audit-report 1.0.1"
git push
```

CI 判断：

- `VERSION` 未变 → 跳过应用构建
- 只有该模板版本变大 → 只打包该模板
- 其他模板沿用上一轮地址
- `latest.json` 顶层应用字段保持不变

### 4.3 新增模板

```
src-tauri/templates/<id>/
├─ manifest.json
├─ CHANGELOG.md
├─ pages/
└─ assets/
```

`manifest.json` 至少包含：

```json
{
  "schemaVersion": 1,
  "id": "new-template",
  "version": "1.0.0",
  "name": "新模板",
  "minAppVersion": "0.1.0"
}
```

上一轮 `latest.json` 中没有该 `id`，CI 会直接打包并追加。

### 4.4 移除模板

删除模板目录并提交。下一轮发布会：

- 从新的 `latest.json` 中移除该模板
- 不删除用户本地已安装的副本

---

## 5. CI 发布流程

工作流：`.github/workflows/release.yml`，触发条件为推送 `main` 或手动运行。

```
plan
 ├─ 下载上一轮 latest.json
 ├─ 校验 VERSION 与三处版本号一致
 ├─ 比较应用版本
 ├─ 比较每个模板版本
 ├─ 识别新增/移除模板
 └─ 输出 release-plan.json + release-notes.md

build-app（仅应用版本变大时，x64 / ARM64 矩阵并行）
 ├─ 安装对应的 Rust target
 └─ 配置 Pages + Release 更新源 → tauri build --target <target> → 整理 ASCII 文件名 → 上传产物

pack-templates（仅模板版本变大时）
 └─ report-dev pack --only <变动的模板>

release
 ├─ 下载计划、应用产物、模板产物
 ├─ 合并 latest.json（未变资源沿用上一轮 URL）
 ├─ 创建 Draft Release
 ├─ 上传安装包、.sig、模板 zip
 ├─ 最后上传 latest.json
 └─ 发布为 latest
```

其他约束：

- `concurrency: release` 保证同一时间只有一轮发布。
- Draft 上传完成后才发布为 latest，避免客户端读到缺少资产的 `latest.json`。
- 发布 tag 使用 `release-<run_number>-<run_attempt>`，避免重跑冲突。
- 没有版本变化时不会创建空 Release。
- 当前工作流会在构建成功后自动发布为 latest；如需人工确认，删除「发布为 latest」这一步即可。

---

## 6. latest.json

示例：

```json
{
  "version": "0.1.1",
  "notes": "应用 0.1.1 更新说明",
  "pub_date": "2026-10-17T10:00:00Z",
  "platforms": {
    "windows-x86_64-nsis": {
      "signature": "<安装包 .sig 内容>",
      "url": "https://github.com/<owner>/<repo>/releases/download/<tag>/report-studio_0.1.1_x64-setup.exe"
    },
    "windows-x86_64": {
      "signature": "<同一个 .sig 内容>",
      "url": "https://github.com/<owner>/<repo>/releases/download/<tag>/report-studio_0.1.1_x64-setup.exe"
    },
    "windows-aarch64-nsis": {
      "signature": "<ARM64 安装包 .sig 内容>",
      "url": "https://github.com/<owner>/<repo>/releases/download/<tag>/report-studio_0.1.1_arm64-setup.exe"
    },
    "windows-aarch64": {
      "signature": "<同一个 ARM64 .sig 内容>",
      "url": "https://github.com/<owner>/<repo>/releases/download/<tag>/report-studio_0.1.1_arm64-setup.exe"
    }
  },
  "schemaVersion": 1,
  "generatedAt": "2026-10-17T10:00:00Z",
  "release": "release-123-1",
  "templates": [
    {
      "id": "settle-audit-report",
      "name": "竣工结算审核报告",
      "version": "1.0.1",
      "notes": "1.0.1：修正第 8 页表头",
      "url": "https://github.com/<owner>/<repo>/releases/download/release-123-1/settle-audit-report-1.0.1.zip",
      "sha256": "<zip sha256>",
      "size": 123456,
      "minAppVersion": "0.1.0",
      "publishedAt": "2026-10-17T10:00:00Z"
    },
    {
      "id": "start-completion-report",
      "name": "开竣工报告、竣工验收证明书",
      "version": "1.0.0",
      "notes": "1.0.0：初始版本",
      "url": "https://github.com/<owner>/<repo>/releases/download/release-100-1/start-completion-report-1.0.0.zip",
      "sha256": "<zip sha256>",
      "size": 234567,
      "minAppVersion": "0.1.0",
      "publishedAt": "2026-10-10T09:00:00Z"
    }
  ]
}
```

关键规则：

- 顶层 `version` 永远是**应用版本**，不是发布编号。
- 只发模板时，顶层 `version`、`notes`、`pub_date`、`platforms` 原样沿用上一轮。
- 未变模板的条目原样沿用上一轮，`url` 指向旧 Release。
- `templates` 是自定义字段，Tauri updater 会忽略；应用更新和模板同步共用这一个文件。
- `platforms` 同时提供 `windows-x86_64` 和 `windows-aarch64`，并附带 `-nsis` 兼容键。
  Tauri updater 会按当前系统与架构选择对应安装包；两个架构的 `.sig` 必须分别生成。

---

## 6.1 文档站与下载按钮

每次 Release 发布完成后，`.github/workflows/pages.yml` 会自动：

1. 拉取最新 `latest.json`
2. 下载 Windows x64 / ARM64 两个应用安装包和全部最新模板
3. 校验模板 sha256
4. 生成 `docs/downloads/index.json`
5. 生成改写为 Pages 地址的 `docs/downloads/latest.json`
6. 部署到 GitHub Pages

`latest.json` 中的 `platforms[*].url` 和 `templates[*].url` 会改写为 Pages 上的文件地址，
`signature`、`sha256`、`size` 保持不变。应用自动更新和模板同步都会优先读取 Pages 清单，
失败时回退到 GitHub Release 清单。

注意：如果 Pages 返回 HTTP 200 但清单仍是旧版本，更新器会按“无更新”处理，不会继续
尝试 GitHub；正常情况下 `pages` 工作流会在 Release 成功后立即重新部署。

文档站使用本地 vendoring 的 Docsify + Vue 主题（不使用 CDN）。封面上的下载按钮会读取
`downloads/index.json`，自动识别当前系统与架构；下拉菜单固定提供 Windows x64 / Windows
ARM64 和「更多版本」（指向 GitHub Releases），当前系统架构对应项带有「当前系统」徽标。

完整说明见 [文档站部署](pages.md)。

---

## 7. scripts 使用说明

### 7.1 `npm run version:set`

脚本：`scripts/set-version.mjs`

应用版本：

```powershell
npm run version:set -- 0.1.1
```

- 写入 `VERSION`
- 同步 `package.json`、`package-lock.json`、`tauri.conf.json`、`Cargo.toml`
- 在根目录 `CHANGELOG.md` 插入空的 `## 0.1.1`
- 不构建、不发布

模板版本：

```powershell
npm run version:set -- --template settle-audit-report 1.0.1
```

- 修改该模板 `manifest.json` 的 `version`
- 在模板 `CHANGELOG.md` 插入空的 `## 1.0.1`
- 不同步应用版本

### 7.2 `npm run version:check` / `npm run release:plan`

脚本：`scripts/check-versions.mjs`

```powershell
npm run version:check
npm run version:check -- --prev prev/latest.json
```

常用参数：

| 参数 | 默认值 | 说明 |
| --- | --- | --- |
| `--prev` | 无 | 上一轮 `latest.json`；没有则视为首次发布 |
| `--out` | `release-plan.json` | 发布计划输出路径 |
| `--notes-out` | `release-notes.md` | Release 说明输出路径 |

输出 `release-plan.json`：

```json
{
  "appVersion": "0.1.1",
  "previousVersion": "0.1.0",
  "appChanged": true,
  "appNotes": "应用更新说明",
  "hasChanges": true,
  "templatesToPack": [
    {
      "id": "settle-audit-report",
      "name": "竣工结算审核报告",
      "version": "1.0.1",
      "minAppVersion": "0.1.0",
      "notes": "模板更新说明"
    }
  ],
  "templatesRemoved": [],
  "allTemplateIds": ["settle-audit-report", "start-completion-report"]
}
```

同时会向 `GITHUB_OUTPUT` 写入：

- `app_changed`
- `app_version`
- `template_ids`
- `has_changes`

校验内容：

- `VERSION` 是合法 SemVer
- `VERSION` 与 `tauri.conf.json`、`package.json`、`Cargo.toml` 一致
- 应用/模板版本不能回退
- 模板 `id`、`name`、`version` 合法

### 7.3 `npm run release:merge`

脚本：`scripts/merge-latest.mjs`，通常只在 CI 使用。

```bash
node scripts/merge-latest.mjs \
  --prev prev/latest.json \
  --plan dist/release-plan.json \
  --dist dist \
  --out dist/latest.json \
  --tag "$RELEASE_TAG" \
  --repo "$GITHUB_REPOSITORY"
```

| 参数 | 默认值 | 说明 |
| --- | --- | --- |
| `--prev` | 无 | 上一轮 `latest.json`；增量发布必须提供 |
| `--plan` | `release-plan.json` | 发布计划 |
| `--dist` | `dist` | 本轮产物目录，递归查找 `.exe` / `.exe.sig` / `packed.json` |
| `--out` | `<dist>/latest.json` | 输出路径 |
| `--tag` | `RELEASE_TAG` | 本轮 Release tag |
| `--repo` | `GITHUB_REPOSITORY` | GitHub 仓库，格式 `owner/repo` |

行为：

- 应用有更新：读取 x64 与 ARM64 安装包及各自的 `.sig`，生成两个架构的 `platforms`
- 应用未更新：沿用上一轮的 `version` / `notes` / `pub_date` / `platforms`
- 应用有更新但缺少任一架构的 `.exe` / `.exe.sig` 时直接失败
- 模板有更新：使用本轮 zip 的 URL、sha256、size
- 模板未更新：原样沿用上一轮条目
- 模板被删除：不再写入新 `latest.json`

### 7.4 `npm run updater:configure`

脚本：`scripts/configure-updater.mjs`

```powershell
# 查看当前配置
npm run updater:configure

# 写入配置（按顺序尝试，第一个可用清单生效）
npm run updater:configure -- `
  --endpoints "https://oilu.cn/reportStudio/downloads/latest.json,https://github.com/<owner>/<repo>/releases/latest/download/latest.json" `
  --pubkey-file "$HOME\.tauri\report-studio.key.pub"
```

参数：

| 参数 | 说明 |
| --- | --- |
| `--endpoints` | 逗号分隔的更新清单地址，按顺序尝试，必须为 HTTPS |
| `--endpoint` | 单个更新清单地址，兼容旧用法 |
| `--pubkey-file` | 公钥文件路径 |
| `--pubkey` | 直接传公钥内容 |

作用：

- 写入 `src-tauri/tauri.conf.json` 的 `plugins.updater`
- 不会生成密钥，也不会修改版本号

### 7.5 `report-dev pack`

用于把模板打成 zip，并生成 `packed.json`。CI 的模板发布任务会自动调用。

```powershell
cargo run --manifest-path src-tauri/Cargo.toml -p report-dev -- `
  pack `
  --templates src-tauri/templates `
  --out dist/templates `
  --only settle-audit-report,start-completion-report
```

参数：

| 参数 | 说明 |
| --- | --- |
| `--templates, -t` | 模板根目录，默认 `templates` |
| `--out, -o` | 输出目录，默认 `dist/templates` |
| `--only` | 只打包指定 id，逗号分隔；省略则打包全部 |

输出：

```
dist/templates/
  settle-audit-report-1.0.1.zip
  start-completion-report-1.0.0.zip
  packed.json
```

`packed.json` 包含 id、name、version、file、sha256、size、minAppVersion。

### 7.6 `scripts/version-utils.mjs`

内部共享模块，不直接执行。提供：

- SemVer 解析与比较
- `VERSION`、`package.json`、`tauri.conf.json`、`Cargo.toml` 读写
- 模板扫描与 manifest 读取
- CHANGELOG 段落提取与空段落插入
- 通用命令行参数解析

---

## 8. 本地验证

发布前建议执行：

```powershell
npm run check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml -p report-core
npm run build
npm run tauri build -- --no-bundle
rustup target add aarch64-pc-windows-msvc
npm run tauri build -- --target aarch64-pc-windows-msvc --no-bundle
npm run version:check
```

本地模拟合并 `latest.json` 时，`dist` 中必须同时存在 x64 与 ARM64 的 `.exe` 和
`.exe.sig`，否则 `merge-latest` 会直接失败。

如需本地模拟发布计划：

```powershell
npm run release:plan -- --prev dist/prev-latest.json
```

如需本地模拟合并：

```powershell
npm run release:merge -- `
  --prev dist/prev-latest.json `
  --plan release-plan.json `
  --dist dist `
  --out dist/latest.json `
  --tag release-local `
  --repo <owner>/<repo>
```

---

## 9. 回滚与约束

- **不能降级**：`VERSION` 或模板版本小于上一轮，CI 直接失败。
- **不能删除被引用的旧 Release**：未变资源的 URL 指向旧 Release，删除后更新链断裂。
- **应用回滚**：发布一个更高版本号，重新包含旧逻辑；不要试图降低版本号。
- **模板回滚**：同样发布更高版本号；已安装用户不会自动降级。
- **模板兼容性**：通过 `minAppVersion` 限制，旧应用会跳过需要新版的模板。
- **私钥安全**：只放在本机安全位置和 GitHub Secret，不要写入仓库、日志或 `latest.json`。

---

## 10. 常见问题

**推了代码但没有发布**

版本号没变。应用需要提升 `VERSION`，模板需要提升 `manifest.version`。这是版本号驱动的预期行为。

**CI 报版本号不一致**

```powershell
npm run version:set -- <当前版本>
```

让脚本重新同步四处版本号。

**客户端 404 `latest.json`**

- 仓库或 Release 仓库不是公开可读
- 还没有发布过正式 Release
- 目标 Release 仍是 Draft
- endpoint 写错

**更新签名校验失败**

- `TAURI_UPDATER_PUBKEY` 与构建使用的私钥不匹配
- 客户端安装包被篡改
- `latest.json` 的 `signature` 不是 `.sig` 文件内容

**模板没有更新**

- 模板 `manifest.version` 没变
- `latest.json` 中 `templates[].url` 或 `sha256` 错误
- `minAppVersion` 高于当前应用版本
- 客户端网络无法访问 GitHub

**Release 上传失败**

检查工作流权限是否为：

```yaml
permissions:
  contents: write
```

---

## 11. 相关文件

```
VERSION
CHANGELOG.md
.github/workflows/release.yml
scripts/set-version.mjs
scripts/check-versions.mjs
scripts/merge-latest.mjs
scripts/configure-updater.mjs
scripts/version-utils.mjs
src-tauri/src/template_sync.rs
src-tauri/templates/<id>/manifest.json
src-tauri/templates/<id>/CHANGELOG.md
src-tauri/crates/report-dev/src/main.rs
src/lib/updater.ts
src/lib/templateSync.ts
```
