# 报告工坊 (Report Studio)

桌面端报告模板编辑与输出工具。基于 **Tauri v2 + Svelte 5 + Vite + Tailwind v4**。

管理员编写/导入报告模板，普通用户填写变量后实时预览、导出 PDF 或打印。渲染逻辑
与桌面外壳解耦，可独立复用与调试。

在线文档与下载：<https://2061360308.github.io/reportStudio/>

## 目录结构

```
report-studio/
├─ src/                        # Svelte 前端（编辑器 / 预览阅读器 / 命令面板）
├─ src-tauri/
│  ├─ src/                     # 桌面应用（Tauri 命令、打印、导出）
│  │  ├─ commands.rs           # 前后端命令接口
│  │  ├─ pdf.rs                # WebView2 打印 / 导出 PDF
│  │  └─ lib.rs                # 应用入口、协议、模板目录解析
│  ├─ crates/
│  │  ├─ report-core/          # 纯渲染库（模型 / 渲染 / 模板管理），不依赖 Tauri
│  │  └─ report-dev/           # 模板开发服务器（仅开发期使用，不打包）
│  └─ templates/               # 模板源文件（不随安装包发布，由发布工作流单独分发）
└─ package.json
```

- **report-core**：`model`（变量/清单模型）、`render`（minijinja 渲染、自定义过滤器）、
  `templates`（模板安装/导出/目录管理）。桌面包与开发服务器共用同一套渲染逻辑。
- **report-dev**：把模板渲染成可在浏览器中浏览的 HTML，改动模板文件时自动热重载，
  并提供一个悬浮变量面板用于实时调试字段。

## 开发

```bash
npm install
npm run tauri dev      # 启动桌面应用（开发模式）
```

开发模式直接读取 `src-tauri/templates/`。发布版安装包不再内置模板，首次启动会从
`latest.json` 同步到应用数据目录（Windows 下为 `%APPDATA%/com.muban.reportstudio/templates`）。

### 模板开发服务器

无需启动桌面应用即可调试模板（变量注入 + 热重载）：

```bash
npm run dev:template -- src-tauri/templates/settle-audit-report
```

- 默认监听 `http://127.0.0.1:1735`（端口被占用时自动挑选空闲端口）。
- 左下角悬浮「模板变量」按钮可开合左侧变量面板（状态记忆在浏览器本地）。
- 面板以 `manifest.sample` 为初始值，修改后点「应用并刷新」即时重渲染。
- 面板顶部可配置「资源目录」；`image` / `images` 变量可直接选择图片上传。
- 勾选「显示字段标记」，或在 URL 加 `?annot=1`，高亮所有字段以核对数据绑定。
- 预览页用灰色底 + 阴影/描边勾勒每一页纸张轮廓，便于对照版面。
- 直接编辑 `pages/*.html`、`assets/*.css` 等文件，浏览器会在约 1 秒内自动刷新。

参数：

| 参数 | 说明 |
| --- | --- |
| `--port, -p` | 监听端口，默认 `1735` |
| `--values, -v` | 可选的 JSON 变量文件，覆盖 `sample` 值 |
| `--resources, -r` | 可选的工程资源目录；也可在侧边栏配置 |
| `-h, --help` | 显示帮助 |

## 模板结构（manifest）

```jsonc
{
  "schemaVersion": 1,
  "id": "my-template",
  "version": "1.0.0",
  "name": "我的模板",
  "assetsPath": "assets",   // 可选，默认 "assets"
  "pagesPath": "pages",     // 可选，默认 "pages"
  "variables": [ /* … */ ],
  "pages": [ /* 每页几何与 file */ ],
  "toc": [ /* … */ ],
  "hooks": { "validate": "assets/validate.js" } // 可选，变量校验脚本
}
```

- 页面清单 `pages` 为必填（不再有 `entry` 单页回退）。
- 模板静态资源（`base.css`、图片等）统一放 `assetsPath` 目录下，页面片段用 `../assets/…` 引用。
- 变量校验脚本在 Worker 中隔离执行，输入时提示，导出 PDF/打印前拦截错误。详见 [模板编写指南](docs/templates.md)。
- 内容超出单页的页：在该页加 `"mode": "flow"`（见下）。

## 图片资源变量（用户上传）

变量类型 `image`（单张）与 `images`（多张）：值保存为**资源文件名**，资源文件放在工程的 `resources/` 目录，随工程 zip 自包含。

```jsonc
{ "key": "license", "alias": "营业执照", "type": "image",
  "accept": ["png", "jpg", "jpeg"] }                 // 允许的扩展名
{ "key": "scans", "alias": "合同扫描件", "type": "images",
  "accept": ["png", "jpg"], "max": 20 }              // max 限制张数
```

- 用户在编辑器选择文件后，文件按内容哈希（`sha256` 前 8 位 + 扩展名）写入 `resources/`，自动去重。
- 渲染前后端把值改写为**绝对挂载 URL**，模板直接写即可：
  ```html
  <img src="{{ license }}">
  {% for s in scans %}<img class="scanimg" src="{{ s }}">{% endfor %}
  ```

## 模板作者：单页内容溢出自动续页

页面默认 `fixed`（固定一页）。若某页内容会随数据变长（明细表/清单），在 manifest 的该页加 `"mode": "flow"`，内容超出页框时由排版引擎自动续到下一页：

```jsonc
{ "file": "pages/p09.html", "mode": "flow", "size": "A4", "orientation": "portrait", "mt": "7mm", "mr": "7mm", "mb": "7mm", "ml": "7mm" }
```

- **数据仍由 minijinja 渲染**：片段里照样写 `{% for r in items %}<tr>…</tr>{% endfor %}`，分页只是排版，不写渲染 JS。
- 表格用 `<thead>` 承载列头，跨页时会自动重复；表格行不会被拦腰断开。
- 续页沿用该页的统一页边距（`mt/mr/mb/ml`）。
- **预览**：流式页以连续滚动呈现，并叠加每 A4 高的分页参考线（仅屏幕，不影响导出）。
- 依赖 `height:100%`/flex 撑满页面的页（盖章表、封面等）请保持 `fixed`，否则会失去撑满效果。

## 构建

```bash
npm run build                 # 构建前端
npm run tauri build           # 构建安装包（NSIS）
```

## 发版

应用版本以根目录 `VERSION` 为准，模板版本以各模板 `manifest.json` 的 `version` 为准。

```bash
# 应用发版：同步版本号，并在 CHANGELOG.md 插入空的当前版本段落
npm run version:set -- 0.1.1

# 模板发版：更新模板版本，并在模板 CHANGELOG.md 插入空段落
npm run version:set -- --template settle-audit-report 1.0.2
```

提交并推送到 `main` 后，`.github/workflows/release.yml` 会比较上一轮 `latest.json`：

- `VERSION` 变大 → 构建应用
- 某个模板版本变大 → 只打包该模板
- 新模板 → 直接打包
- 未变化资源 → 沿用上一轮 Release 的地址
- 版本号回退或与 `tauri.conf.json` / `package.json` / `Cargo.toml` 不一致 → CI 失败

首次配置更新源：

```bash
npm run tauri signer generate -- -w $HOME/.tauri/report-studio.key
npm run updater:configure -- \
  --endpoint https://github.com/<owner>/<repo>/releases/latest/download/latest.json \
  --pubkey-file $HOME/.tauri/report-studio.key.pub
```

CI 需要配置 `TAURI_SIGNING_PRIVATE_KEY` 与 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
两个 Secret。更新源要求仓库或 Release 仓库公开可读。

完整说明见 [docs/dev/release.md](docs/dev/release.md)。

## 许可证

MIT
