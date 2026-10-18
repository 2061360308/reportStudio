# 文档站与下载按钮

文档站使用 **Docsify + Vue 主题**，运行时完全使用 `docs/vendor/docsify/` 下的本地文件，
不依赖 CDN。普通页面全部复用 Docsify 默认渲染，只有封面上的下载按钮是自定义控件。

每次发布新 Release 后，Pages 工作流会拉取 `latest.json`，下载最新的 **Windows x64** 和
**Windows ARM64** 两个安装包以及模板，生成 `downloads/index.json`，再和 Docsify 文档
一起部署。

---

## 1. 目录结构

```
docs/
├─ index.html                 # Docsify 入口
├─ _coverpage.md              # 封面 + 下载按钮
├─ README.md                  # Docsify 首页内容
├─ _sidebar.md                # Docsify 侧边栏
├─ _footer.md                 # Docsify 页脚
├─ .nojekyll                  # 保证 _sidebar.md 等文件可访问
├─ assets/
│  ├─ cover.css               # 封面布局、按钮与下拉菜单样式
│  ├─ cover-download.js       # 系统检测与下拉菜单
│  └─ icon.ico                # 站点图标 / 封面 Logo（来自 src-tauri/icons/icon.ico）
├─ vendor/
│  └─ docsify/                # 本地 vendoring，不依赖 CDN（Docsify 4.13.1）
│     ├─ docsify.min.js
│     ├─ vue.css              # 已移除 Google Fonts 的 @import
│     ├─ search.min.js
│     └─ LICENSE
├─ dev/
│  ├─ release.md
│  └─ pages.md
└─ downloads/                 # CI 生成（不提交）
   ├─ index.json
   ├─ latest.json
   ├─ report-studio_<version>_<platform>.exe
   └─ <template-id>-<version>.zip
```

`docs/downloads/` 已加入 `.gitignore`。

---

## 2. 本地预览

```powershell
npx serve docs
```

或：

```powershell
npx http-server docs -p 4173
```

打开对应地址即可。不要直接双击 `docs/index.html`，Docsify 需要通过 HTTP 读取 Markdown。

---

## 3. 封面下载按钮

`_coverpage.md` 包含：

- 同一行的操作按钮：`开始阅读`、`GitHub`
- 下载按钮（主按钮直接下载，右侧箭头展开下拉菜单）

默认行为：

1. `cover-download.js` 请求 `downloads/index.json`
2. 通过 `navigator.userAgentData`（不可用时回退到 User-Agent）识别操作系统与 CPU 架构
3. 主按钮显示当前系统对应的安装包，例如「下载 Windows x64 v0.1.1」或
   「下载 Windows ARM64 v0.1.1」
4. 下拉菜单显示：
   - Windows x64
   - Windows ARM64
   - 「更多版本」→ GitHub Releases
5. 与当前系统架构匹配的 x64 / ARM64 选项会显示「当前系统」胶囊徽标
6. 没有 `index.json` 或请求失败时，主按钮回退到 GitHub Releases

Windows ARM64 浏览器无法通过 User-Agent 可靠区分时，`navigator.userAgentData` 的
`architecture` 字段会给出正确结果；仍无法判断则默认选择 x64，用户可在下拉菜单中手动
切换到 ARM64。

`downloads/index.json` 缺少某一架构时，下拉菜单只展示实际存在的架构和「更多版本」。

`index.html` 会加载 `assets/cover.css` 和 `assets/cover-download.js`，但样式全部限定在
`section.cover` 内，不会影响侧边栏、正文和导航。

### 本地依赖

`docs/vendor/docsify/` 的文件来自 `docsify@4.13.1` 的 npm 包：

- `lib/docsify.min.js` → `vendor/docsify/docsify.min.js`
- `lib/themes/vue.css` → `vendor/docsify/vue.css`（删除 Google Fonts 的 `@import`）
- `lib/plugins/search.min.js` → `vendor/docsify/search.min.js`
- `LICENSE` → `vendor/docsify/LICENSE`

升级 Docsify 时替换上述文件即可，不需要修改页面代码。

---

## 4. Pages 工作流

工作流：`.github/workflows/pages.yml`

触发条件：

- 推送 `main`，且改动涉及 `docs/**`、下载准备脚本或工作流本身
- `release` 工作流完成
- 手动 `workflow_dispatch`

### 为什么用 `workflow_run`

Release 是由 `release.yml` 使用 `GITHUB_TOKEN` 创建的。GitHub 不会因为
`GITHUB_TOKEN` 创建的 Release 再次触发 `release: published` 工作流，因此这里使用：

```yaml
on:
  workflow_run:
    workflows: [release]
    types: [completed]
```

并只在该 Release 工作流成功时继续部署。

### 主要步骤

1. 签出默认分支
2. 下载最新 Release 的 `latest.json`
3. 运行 `scripts/prepare-pages-downloads.mjs`，下载 x64 / ARM64 安装包与全部最新模板
4. 上传 `docs/` 为 Pages artifact
5. 部署到 GitHub Pages

下载步骤使用 `GITHUB_TOKEN` 访问 Release 资产，避免匿名请求限流。如果仓库还没有任何
Release，工作流只部署 Docsify 文档，封面按钮回退到 Releases。

---

## 5. 下载资源生成

脚本：`scripts/prepare-pages-downloads.mjs`

```bash
node scripts/prepare-pages-downloads.mjs \
  --latest .release/latest.json \
  --docs docs \
  --repo <owner>/<repo>
```

参数：

| 参数 | 说明 |
| --- | --- |
| `--latest` | `latest.json` 路径；不存在时跳过下载资源 |
| `--docs` | 文档目录，默认 `docs` |
| `--repo` | GitHub 仓库，用于生成 Releases 链接；默认读 `GITHUB_REPOSITORY` |

行为：

- 清空并重建 `docs/downloads/`
- 遍历 `latest.json` 的 `platforms`，按 URL 去重后下载安装包
- 同时保留 `windows-x86_64-nsis` 与 `windows-aarch64-nsis` 两个 CPU 版本
- 下载每个模板为 `<id>-<version>.zip`
- 校验模板 `sha256` 和 `size`
- 复制一份 `latest.json` 到 `docs/downloads/latest.json`
- 生成 `docs/downloads/index.json`

`index.json` 结构：

```json
{
  "version": "0.1.1",
  "notes": "更新说明",
  "releases": "https://github.com/<owner>/<repo>/releases",
  "platforms": [
    {
      "key": "windows-x86_64-nsis",
      "label": "Windows x64",
      "os": "windows",
      "arch": "x86_64",
      "href": "downloads/report-studio_0.1.1_windows-x86_64-nsis.exe"
    },
    {
      "key": "windows-aarch64-nsis",
      "label": "Windows ARM64",
      "os": "windows",
      "arch": "aarch64",
      "href": "downloads/report-studio_0.1.1_windows-aarch64-nsis.exe"
    }
  ],
  "templates": [
    {
      "id": "settle-audit-report",
      "name": "竣工结算审核报告",
      "version": "1.0.0",
      "notes": "更新说明暂无",
      "href": "downloads/settle-audit-report-1.0.0.zip"
    }
  ]
}
```

`cover-download.js` 只读取 `platforms` 和 `releases`；模板资源会随 Pages 一起发布，
供后续扩展使用。

---

## 6. 封面样式

`assets/cover.css` 只做封面相关样式：

- 封面内容水平居中，最大宽度 760px
- 封面顶部使用项目图标 `src-tauri/icons/icon.ico`
- 操作按钮与下载按钮在同一行排列
- 下载按钮沿用 Vue 主题的绿色描边
- 下拉菜单使用白色卡片和轻阴影
- 不覆盖 Docsify 的侧边栏、正文、导航和 GitHub 徽标

其余页面完全使用 stock Docsify + Vue 主题。

---

## 7. 仓库设置

1. 仓库 **Settings → Pages**
2. **Build and deployment → Source** 选择 **GitHub Actions**
3. 确保仓库公开（私有仓库的 Pages 需要 GitHub Enterprise）
4. 首次部署后，Pages 地址通常是：

```
https://<owner>.github.io/<repo>/
```

---

## 8. 常见问题

**侧边栏或 GitHub 徽标不显示**

- 确认 `docs/index.html` 中配置了 `repo`
- 确认加载的是 `docs/vendor/docsify/vue.css`
- 确认没有额外的全局样式覆盖 Docsify

**封面下载按钮一直显示「下载最新版本」**

- 还没有发布 Release
- `docs/downloads/index.json` 不存在
- `pages` 工作流失败
- 浏览器拦截了 `fetch`，可查看控制台

**下拉菜单缺少 x64 或 ARM64**

当前 `latest.json` / `downloads/index.json` 只包含一个 CPU 架构。确认 `release` 工作流的
`build-app` 矩阵同时成功构建了 x64 与 ARM64。

**Release 后封面按钮没有更新**

- 检查 `release` 工作流是否成功
- 检查 `pages` 工作流的 `workflow_run` 是否被触发
- 检查 Pages Source 是否为 GitHub Actions
- 检查工作流权限是否包含 `pages: write` 和 `id-token: write`

**模板下载 404**

`latest.json` 里模板的 `url` 指向旧 Release 或被删除的资产。不要删除仍被引用的旧 Release。
