# 报告工坊

> 桌面端工程报告模板编辑与输出工具

报告工坊基于 **Tauri v2 + Svelte 5 + Vite + Tailwind v4**。管理员可以编辑、导入报告
模板，普通用户填写变量后实时预览、导出 PDF 或打印。渲染逻辑与桌面外壳解耦，可独立
复用与调试。

## 下载与安装

点击封面上的下载按钮即可获取最新版本。按钮会自动识别当前系统：

- Windows x64
- Windows ARM64

下拉菜单中可以切换到另一个 CPU 架构，或前往 GitHub Releases 查看全部版本。

安装包不再内置模板，首次启动会从 `latest.json` 同步模板到应用数据目录（Windows 下
为 `%APPDATA%/com.muban.reportstudio/templates`）。

## 主要能力

- **模板化报告**：变量清单、页面几何、目录结构全部由 `manifest.json` 描述
- **实时预览**：编辑变量后即时重渲染，支持打印与 PDF 导出
- **图片资源变量**：单图 / 多图变量按内容哈希存入工程 `resources/`，随工程自包含
- **自动续页**：页面标记 `flow` 后，内容超出页框时由排版引擎自动续页
- **变量校验钩子**：模板可用 Worker 隔离执行校验脚本，输入即时提示，导出前拦截错误
- **模板开发服务器**：无需启动桌面应用即可调试模板，支持热重载与字段标记
- **自动更新**：应用与模板分开分发，按版本增量发布

## 本地开发

```bash
npm install
npm run tauri dev
```

开发模式直接读取 `src-tauri/templates/`。

模板开发服务器：

```bash
npm run dev:template -- src-tauri/templates/settle-audit-report
```

默认监听 `http://127.0.0.1:1735`，左下角「模板变量」按钮可打开变量面板；可在面板中配置资源目录，并直接上传 `image` / `images` 变量图片。

## 文档导航

- [模板编写指南](templates.md)：`manifest.json`、页面函数、资源路径、导出与易错点
- [发版与自动更新](dev/release.md)：版本规则、GitHub Release、`latest.json` 与脚本说明
- [文档站与下载按钮](dev/pages.md)：Docsify 文档站结构、下载按钮与 Pages 工作流
- [GitHub Releases](https://github.com/2061360308/reportStudio/releases)：全部历史版本

## 目录结构

```text
report-studio/
├─ src/                        # Svelte 前端
├─ src-tauri/
│  ├─ src/                     # 桌面应用（Tauri 命令、打印、导出）
│  ├─ crates/
│  │  ├─ report-core/          # 纯渲染库（模型 / 渲染 / 模板管理）
│  │  └─ report-dev/           # 模板开发服务器
│  └─ templates/               # 模板源文件（随发布工作流单独分发）
├─ docs/                       # Docsify 文档站与下载页
├─ scripts/                    # 版本、发布与文档下载脚本
└─ package.json
```

## 相关文件

| 文件 | 作用 |
| --- | --- |
| `VERSION` | 应用版本唯一入口 |
| `.github/workflows/release.yml` | 构建双架构安装包并发布 Release |
| `.github/workflows/pages.yml` | 生成下载资源并部署文档站 |
| `scripts/prepare-pages-downloads.mjs` | 下载 Release 安装包与模板，生成下载索引 |
| `scripts/merge-latest.mjs` | 合并双架构安装包与模板，生成 `latest.json` |
