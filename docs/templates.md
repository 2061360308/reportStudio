# 模板编写指南

本文说明报告模板的目录结构、`manifest.json` 字段、页面渲染上下文、可用函数与过滤器，
以及模板编写和导出 PDF 时需要注意的常见问题。

模板渲染由 `report-core` 负责，桌面端与模板开发服务器共用同一套渲染逻辑。

- 模型定义：`src-tauri/crates/report-core/src/model.rs`
- 渲染与过滤器：`src-tauri/crates/report-core/src/render.rs`
- 桌面端命令：`src-tauri/src/commands.rs`
- 示例模板：`src-tauri/templates/`

---

## 1. 目录结构

一个模板通常包含：

```text
my-template/
├─ manifest.json
├─ assets/
│  ├─ base.css
│  ├─ logo.png
│  └─ fit.js
└─ pages/
   ├─ p01.html
   ├─ p02.html
   └─ p03.html
```

- `manifest.json` 必须在模板根目录，或在 zip 解压后唯一子目录的根目录。
- 页面文件路径由 `manifest.pages[].file` 指定，默认位于 `pages/`。
- 静态资源放在 `assetsPath` 指定的目录，默认 `assets/`。
- `assets/base.css` 会被自动引入；其他 CSS、JS、图片需要页面自行引用。

---

## 2. manifest.json 根字段

```jsonc
{
  "schemaVersion": 1,
  "id": "my-template",
  "version": "1.0.0",
  "contentHash": "",
  "name": "我的模板",
  "minAppVersion": "0.1.0",
  "assetsPath": "assets",
  "pagesPath": "pages",
  "variables": [],
  "pages": [],
  "toc": []
}
```

| 字段 | 必填 | 说明 |
| --- | --- | --- |
| `schemaVersion` | 是 | 当前固定写 `1` |
| `id` | 是 | 模板唯一标识，同时用于安装目录名，发布后不要修改 |
| `version` | 是 | 模板版本，内容更新后必须提升 |
| `contentHash` | 否 | 源模板留空，安装或同步时自动计算 |
| `name` | 是 | 模板显示名称 |
| `minAppVersion` | 否 | 远端同步时要求的最低应用版本 |
| `assetsPath` | 否 | 资源目录，默认 `assets` |
| `pagesPath` | 否 | 页面目录，默认 `pages`，同时作为预览页的 `<base href>` |
| `variables` | 否 | 变量定义列表 |
| `pages` | 是 | 页面清单，不能为空 |
| `toc` | 否 | 应用内目录 |
| `hooks` | 否 | 生命周期钩子；`validate` 用于声明变量校验脚本 |

注意：

- `pagesPath` **不会自动加到** `pages[].file` 前面。`file` 必须是相对模板根目录的真实路径。
- `assetsPath` 目前只负责自动引入 `../{assetsPath}/base.css`，不会自动处理其他资源路径。
- `id` 发布后修改会被视为新模板，旧工程可能无法找到模板。
- 模板内容更新后必须提升 `version`，仅修改文件不会触发模板更新。
- `contentHash` 不要手动维护，安装时会自动重算。
- `hooks.validate` 声明后，应用会在输入和导出前执行校验；未声明时跳过。

---

## 3. 变量定义

### 3.1 通用字段

```jsonc
{
  "key": "project_name",
  "alias": "工程名称",
  "label": "工程名称",
  "type": "string",
  "required": true,
  "default": "",
  "sample": "示例工程",
  "help": "填写完整工程名称",
  "placeholder": "请输入工程名称",
  "unit": "",
  "group": "",
  "order": 1
}
```

| 字段 | 说明 |
| --- | --- |
| `key` | 数据键名，必须唯一，建议使用 ASCII 下划线命名 |
| `alias` | 别名，用于预览字段列表；ASCII 时也可作为页面顶层变量名 |
| `label` | 编辑区标签，优先于 `alias` 显示 |
| `type` | 变量类型 |
| `required` | 必填标记；导出/打印前校验 |
| `default` | 新建项目时的默认值 |
| `sample` | 预览占位值，仅在字段为空时使用 |
| `unit` | 编辑区显示的单位，不会自动写入页面 |
| `placeholder` | 输入框占位文字 |
| `help` | 字段下方帮助文字 |
| `options` | `select` / `multiselect` 选项：`{ "value": "...", "label": "..." }` |
| `accept` | `image` / `images` 允许的扩展名 |
| `max` | `images` 最多张数 |
| `expression` | `computed` 计算表达式 |
| `columns` | `table` 列定义 |
| `format` | 模型接受，但当前未实现 |
| `group` | 模型接受，但当前编辑器未使用 |
| `order` | 模型接受，但当前编辑器未使用 |
| `readonly` | 模型接受，但当前编辑器未使用 |

编辑区按 `variables` 数组顺序渲染。需要调整字段顺序时，直接调整数组顺序。

### 3.2 变量类型

| 类型 | 编辑控件 | 值形态 |
| --- | --- | --- |
| `string` | 单行文本 | 字符串 |
| `text` | 多行文本 | 字符串 |
| `number` | 数字输入 | 数字或 `null` |
| `integer` | 整数输入 | 数字或 `null` |
| `money` | 数字输入 | 数字或 `null` |
| `percent` | 数字输入，步长 `0.01` | 数字或 `null` |
| `date` | 日期选择 | `YYYY-MM-DD` 字符串 |
| `datetime` | 日期时间选择 | 本地日期时间字符串 |
| `bool` | 复选框 | 布尔值 |
| `select` | 下拉框 | 字符串 |
| `multiselect` | 多选下拉框 | 字符串数组 |
| `table` | 表格编辑器 | 对象数组 |
| `image` | 单图上传 | 资源路径字符串 |
| `images` | 多图上传 | 资源路径数组 |
| `computed` | 只读显示 | 由表达式计算 |

注意：

- `table` / `images` 的 `default` 当前不会生效，新建项目时固定为 `[]`。
- `image`（单张）的 `default` 可以生效，适合放 `assets/logo.png`。
- `computed` 的 `default` 没有意义，会被表达式结果覆盖。
- 未知类型会退化成普通文本框，且不会走图片等特殊逻辑。

### 3.3 computed 计算变量

```jsonc
{
  "key": "actual_work_days",
  "alias": "实际工期",
  "type": "computed",
  "expression": "days_between(plan_start_date, plan_end_date)"
}
```

- 表达式使用 MiniJinja 表达式语法，可以调用过滤器与函数。
- 计算会迭代最多 8 轮，支持变量之间的依赖。
- 结果会尝试解析为整数、浮点数、布尔值，否则保存为字符串。
- 表达式中的顶层标识符优先使用 ASCII `alias`；`alias` 为中文时使用 ASCII `key`。
- 所有变量始终可以通过 `v.key` 或 `v['key']` 访问。
- 表达式出错会被静默忽略，不会弹出错误；计算值异常时需要自行检查依赖和类型。
- 避免循环依赖，超过迭代次数后可能无法得到稳定结果。

示例：

```jsonc
{ "expression": "reported_amount - approved_amount" }
{ "expression": "((reported_amount * 0.0035 + deducted_amount * 0.06) | round)" }
```

### 3.4 table 表格变量

```jsonc
{
  "key": "items",
  "alias": "明细",
  "type": "table",
  "columns": [
    { "key": "name", "alias": "名称", "type": "string" },
    { "key": "qty", "alias": "数量", "type": "number" },
    { "key": "price", "alias": "单价", "type": "money" },
    {
      "key": "amount",
      "alias": "金额",
      "type": "computed",
      "expression": "qty * price"
    }
  ]
}
```

- 数据是对象数组，例如 `[{ "name": "项目", "qty": 2, "price": 10 }]`。
- 计算列在行上下文中求值，可以使用列 `key`、ASCII `alias`、`r`（当前行）和 `v`（全局变量）。
- 页面中行对象会注入 ASCII 列别名，因此可以写 `row.amount`。
- 表格列为 `computed` 时在编辑器中只读。
- 表格列 `key` 也必须唯一。

### 3.5 image / images

```jsonc
{ "key": "license", "alias": "营业执照", "type": "image",
  "accept": ["png", "jpg", "jpeg"] }
{ "key": "scans", "alias": "合同扫描件", "type": "images",
  "accept": ["png", "jpg"], "max": 20 }
```

- 用户上传的图片按内容哈希命名，存放在工程 `resources/` 目录。
- 渲染时值会被改写为绝对挂载 URL，页面中直接使用即可。
- 只支持 `png`、`jpg`、`jpeg`、`bmp`、`webp`、`gif`。SVG、PDF 不支持。
- `accept` 只能缩小可选范围，不能突破后端白名单。
- 固定模板图片也可以直接写 `../assets/xxx.png`，不一定需要定义 `image` 变量。
- 图片占位建议使用 `default`，不要依赖 `sample`。

### 3.6 其他类型注意事项

- `bool` 直接输出会得到 `true` / `false`，中文请用条件判断。
- `select` / `multiselect` 的值是字符串；需要数字时使用 `|int` 转换。
- `date_cn` / `date_dot` 只解析日期格式，不解析带时间的 `datetime`。
- `required` 只在导出/打印前校验，且只拦截 `undefined`、`null`、空字符串；空数组不会拦截。
- 多行文本不会自动保留换行，需要 CSS `white-space: pre-wrap` 或自行处理。

---

## 4. 页面清单 pages

```jsonc
{
  "file": "pages/p01.html",
  "mode": "fixed",
  "order": 1,
  "size": "A4",
  "orientation": "portrait",
  "margin": "0",
  "mt": "10mm",
  "mr": "10mm",
  "mb": "10mm",
  "ml": "10mm"
}
```

| 字段 | 说明 |
| --- | --- |
| `file` | 相对模板根目录的页面片段路径，必填 |
| `mode` | `fixed`（默认）或 `flow`（内容超出自动续页） |
| `order` | 排序号，仅影响应用内预览 |
| `size` | 纸张尺寸，当前只保证 `A4` / `A3` |
| `orientation` | `portrait` / `landscape`，建议小写 |
| `margin` | CSS `@page` 外边距 |
| `mt` / `mr` / `mb` / `ml` | `.paper` 内容四边留白，使用 CSS 长度 |

### 4.1 页面几何变量

渲染器会根据 manifest 生成：

```css
--a4w: 210mm;
--a4h: 297mm;
--mt: 10mm;
--mr: 10mm;
--mb: 10mm;
--ml: 10mm;
```

`assets/base.css` 通常这样使用：

```css
* {
  box-sizing: border-box;
}

html,
body {
  margin: 0;
  padding: 0;
  background: #fff;
}

.paper {
  width: var(--a4w);
  height: var(--a4h);
  padding: var(--mt) var(--mr) var(--mb) var(--ml);
  background: #fff;
}

.page {
  width: 100%;
  height: 100%;
  overflow: hidden;
}
```

建议保留 `box-sizing: border-box`，否则宽度加 padding 可能超出纸张。

### 4.2 页面顺序

- `pages` 数组顺序就是导出 PDF 的顺序。
- `order` 只影响应用内预览排序。
- 如果数组顺序和 `order` 不一致，预览顺序和 PDF 顺序会不同，`toc.page` 也可能对不上。
- 最稳妥的做法：数组本身按最终顺序排列，同时 `order` 保持一致，或全部不写。
- `pages[].file` 必须唯一，重复会导致预览列表键值冲突。

### 4.3 fixed 与 flow

`fixed`：

- 固定一页。
- `.page` 通常 `overflow: hidden`，内容超出会被裁掉。
- 适合封面、签章表、依赖 `height: 100%` 或 flex 撑满页面的版式。

`flow`：

- 内容超出页框时自动续页。
- 适合明细表、清单、图片排版等长度不固定的内容。
- 不要使用固定高度、绝对定位、`height: 100%` 或内层 `overflow: hidden`，否则会破坏分页。
- 表格用 `<thead>` 承载表头，跨页时浏览器会重复表头。
- 表格行会尽量避免跨页断开。
- 续页留白需要在导出 PDF 中重点检查；需要每页固定留白时，参考 `image-layout` 的 `.sheet` 方案。

---

## 5. 页面模板

### 5.1 基本结构

```html
<div class="paper">
  <div class="page">
    <p class="title">报告标题</p>
    <table class="doc-tbl">
      <tr>
        <td>工程名称</td>
        <td>{{ project_name|field('project_name') }}</td>
      </tr>
    </table>
  </div>
</div>
```

必须注意：

- 不要写完整 `<html>` 文档。检测到 `<html` 时渲染器会原样返回，不再注入几何、`base.css` 和字段标记脚本。
- `flow` 模式依赖精确的 `class="paper"` 字符串来追加 `data-flow`，不要改成单引号或额外 class。
- 建议 `.paper` 是页面片段的最后一个顶层元素。示例 `base.css` 中的打印分页规则依赖 `.paper:last-child`，后面再放脚本可能产生空白页。
- 一个页面文件通常只放一个 `.paper` 根。
- 多页图片排版可以像 `image-layout` 一样，在 `.page` 内使用多个 `.sheet`。

### 5.2 变量上下文

顶层变量：

- ASCII `alias` 或 ASCII `key` 会作为顶层标识符暴露。
- 完整数据始终可以通过 `v` 访问。

```jinja
{{ project_name }}
{{ v.project_name }}
{{ v['project_name'] }}
```

表格：

```jinja
{% for row in items %}
  {{ row.name }}
  {{ row.amount }}
{% endfor %}
```

行内计算表达式可以使用 `r`（当前行）和 `v`（全局变量）。

图片：

```jinja
<img src="{{ business_license }}" alt="营业执照">

{% for src in scans %}
  <img src="{{ src }}" alt="">
{% endfor %}
```

图片地址不要使用 `|field`，因为 `field` 会输出 `<span>`，不能放在 `src` 属性里。

### 5.3 自动转义

页面文件是 `.html`，MiniJinja 默认开启 HTML 自动转义：

```jinja
{{ text }}            {# HTML 会被转义 #}
{{ html_value|safe }} {# 原样输出 HTML #}
```

`field` 返回安全 HTML，并会自行转义字段值。

### 5.4 包含与继承

`include` 路径相对于模板根目录，不是当前页面目录：

```jinja
{% include "pages/header.html" %}
```

可以使用：

- `{% include %}`
- `{% extends %}`
- `{% block %}`
- `{% macro %}` / `{% call %}`
- `{% import %}`

公共页眉、页脚建议放在独立 partial 中，通过 `include` 引入。

---

## 6. 自定义函数与过滤器

| 名称 | 用法 | 说明 |
| --- | --- | --- |
| `days_between` | `{{ days_between(start_date, end_date) }}` | 返回包含首尾的天数；同一天返回 1 |
| `money` | `{{ amount\|money }}` | 保留两位小数，无千分位、无货币符号 |
| `date_cn` | `{{ date\|date_cn }}` | 输出 `2026年10月19日` |
| `date_dot` | `{{ date\|date_dot }}` | 输出 `2026.10.19` |
| `rmb_upper` | `{{ amount\|rmb_upper }}` 或 `{{ rmb_upper(amount) }}` | 金额中文大写，四舍五入到分 |
| `field` | `{{ value\|field('key') }}` | 输出带 `data-field` 的字段标记，供预览定位和编辑 |

推荐写法：

```jinja
{{ project_name|field('project_name') }}
{{ plan_start_date|date_cn|field('plan_start_date') }}
{{ approved_amount|money|field('approved_amount') }}
{{ approved_amount|rmb_upper|field('approved_amount') }}
```

注意事项：

- `field` 必须放在过滤器链最后。
- `field` 的 key 使用变量 `key`，不要使用中文 `alias`。
- `field` 只能用在文本位置，不能用于属性值。
- `field` 生成的 `<span>` 可能受 `td > span`、`p > *` 等选择器影响。
- `money` 不添加千分位。
- `rmb_upper(0)` 当前输出 `零整`，不是 `零元整`；零金额需要模板特殊处理。
- `days_between` 包含首尾，反向日期会得到负数。
- `date_cn` / `date_dot` 解析失败时返回原字符串。
- 日期解析支持：
  - `YYYY-MM-DD`
  - `YYYY.MM.DD`
  - `YYYY/MM/DD`
  - `YYYY年MM月DD日`

---

## 7. MiniJinja 内置能力

当前使用 MiniJinja 2.x，默认启用了内置过滤器、宏和多模板能力。

### 7.1 常用过滤器

`batch`、`slice`、`groupby`、`unique`、`sort`、`reverse`、`list`、`chain`、`zip`、
`first`、`last`、`length` / `count`、`sum`、`min`、`max`、`join`、`split`、`lines`、
`replace`、`trim`、`upper`、`lower`、`title`、`capitalize`、`default` / `d`、`round`、
`abs`、`int`、`float`、`string`、`bool`、`attr`、`items`、`dictsort`、`indent`、`map`、
`select`、`reject`、`selectattr`、`rejectattr`、`pprint`、`format`、`safe`、`escape` / `e`。

示例：

```jinja
{% for group in images|batch(4) %}
  {% for src in group %}
    <img src="{{ src }}" alt="">
  {% endfor %}
{% endfor %}
```

### 7.2 常用测试

`defined`、`undefined`、`none`、`number`、`integer`、`float`、`string`、`boolean`、
`sequence`、`iterable`、`mapping`、`odd`、`even`、`divisibleby`、`startingwith`、
`endingwith`、`lower`、`upper`、`sameas`、`eq` / `ne`、比较运算、`in`、`true` / `false`、
`filter`、`test`。

```jinja
{% if owner is defined and owner %}
  ...
{% endif %}
```

### 7.3 控制结构

- `{% if %}` / `{% elif %}` / `{% else %}`
- `{% for %}` / `{% else %}`
- `{% set %}`
- `{% macro %}` / `{% call %}`
- `{% include %}` / `{% extends %}` / `{% block %}` / `{% import %}`
- `loop.index`、`loop.index0`、`loop.first`、`loop.last`、`loop.length`
- `loop.previtem`、`loop.nextitem`

### 7.4 当前不可用

当前构建未启用以下能力：

- `tojson`
- `urlencode`
- `{% break %}` / `{% continue %}`

也没有额外的日期加减函数，日期计算目前只有 `days_between`。

---

## 8. 资源与页面脚本

### 8.1 base.css

渲染器会自动为每个页面引入：

```html
<link rel="stylesheet" href="../{assetsPath}/base.css">
```

因此：

- `assets/base.css` 是模板的公共样式入口。
- 页面专属样式可以直接写在片段内的 `<style>` 中，或额外 `<link>` 其他 CSS。
- `base.css` 里的 `url(...)` 相对 CSS 文件自身，不是相对页面：

```css
.logo {
  background: url(./logo.png);
}
```

### 8.2 页面资源

页面基准地址是 `pagesPath/`，因此：

```html
<img src="../assets/logo.png" alt="">
<script src="../assets/fit.js"></script>
```

不要写成 `assets/logo.png`，它会解析到 `pages/assets/...`。

`pagesPath` / `assetsPath` 建议保持单层目录，不要写前导 `/`、`./` 或尾部 `/`。

### 8.3 页面脚本

页面片段中的 `<script>` 会保留并执行。现有 `fit.js` 提供了一个自适应字号示例：

```html
<td class="fit" data-max="20">
  {{ project_name|field('project_name') }}
</td>
<script src="../assets/fit.js"></script>
```

`fit.js` 会在内容超过 `data-max` 时按比例缩小字号，适合容易溢出的固定页单元格。

注意：

- 避免异步修改版面，打印窗口只在页面加载完成后触发，异步布局可能来不及生效。
- 不要把 `<script>` 放在 `.paper` 之后，可能影响示例 `base.css` 的分页规则。
- 使用本地资源，避免依赖网络图片或外部字体。

### 8.4 字体

- 中文 PDF 依赖目标电脑已安装的字体。
- 使用非常见字体时，建议把字体文件放入 `assets/` 并通过 `@font-face` 引入。
- 开发机和目标机器字体不一致时，导出 PDF 的分页和换行可能不同。

---

## 9. 校验钩子

模板可以通过 `hooks.validate` 声明一个校验脚本。应用会在 Web Worker 中执行它，
用户输入时显示错误/警告，导出 PDF 或打印前再次执行完整校验。

### 9.1 manifest 声明

```jsonc
{
  "hooks": {
    "validate": "assets/validate.js"
  }
}
```

- 路径相对模板根目录。
- 钩子文件必须是 UTF-8 编码，大小不超过 1 MB。
- 钩子文件必须使用经典脚本，不依赖 `import` / `importScripts`。
- Worker 中没有 DOM、没有 Tauri 权限，不能访问 `document`。
- 支持 `window` 写法，但底层是 Worker 的 `self`。
- 钩子文件会随模板打包、参与内容哈希；修改后仍需提升模板 `version`。

### 9.2 钩子函数

```js
window.__RS_TEMPLATE_VALIDATE__ = function (ctx) {
  return [];
};
```

也可以使用：

```js
self.templateValidate = function (ctx) {
  return [];
};
```

### 9.3 上下文

```ts
interface HookContext {
  event: "change" | "preview" | "beforeExport";
  key?: string;                       // 本次变化的字段
  value?: unknown;                    // 本次变化后的值
  values: Record<string, unknown>;    // 已解析的完整变量值，含 computed
  rawValues: Record<string, unknown>; // 原始 project.values
  variables: VariableDef[];           // 变量定义
}
```

- `values` 已包含 computed 计算结果。
- `rawValues` 保留原始输入值。
- 每次触发都会传入完整变量列表，`key` / `value` 只是本次变化的辅助信息。

### 9.4 返回值

推荐返回完整状态列表：

```js
return [
  {
    status: "error",
    keys: ["plan_start_date", "plan_end_date"],
    key: "plan_end_date",
    msg: "计划竣工日期不能早于计划开工日期",
  },
  {
    status: "warning",
    keys: ["budget"],
    msg: "预算金额低于报审金额，请确认",
  },
  {
    status: "ok",
    keys: ["project_name"],
  },
];
```

| 字段 | 说明 |
| --- | --- |
| `status` | `ok` 通过、`warning` 警告、`error` 错误 |
| `keys` | 受影响字段；多字段联动时全部刷新 |
| `key` | 主显示字段；不填时使用 `keys[0]` |
| `msg` | 提示文字 |

也可以返回：

- `true`：全部通过
- `false`：全局错误，默认消息“校验未通过”
- 字符串：全局错误，字符串作为消息
- `{ ok: false, msg, key }`：单条结果
- `{ issues: [...] }`：结果列表

返回列表视为**全量快照**：前端会整体替换当前校验状态。未出现的字段视为中性，
旧的错误会自动清除。`ok` 条目用于显式标记通过状态，不是清除错误的必要条件。

### 9.5 事件

| 事件 | 触发时机 | 说明 |
| --- | --- | --- |
| `change` | 变量输入变化后，防抖约 200ms | 返回全量快照，刷新所有字段状态 |
| `preview` | 打开项目/切换模板时 | 可用于初始提示 |
| `beforeExport` | 导出 PDF、打印前 | 有 `error` 时阻止导出 |

导出前会重新执行 `beforeExport`，并等待校验完成。`warning` 不阻止导出。
校验脚本加载失败、抛异常或超时，都会按错误处理并阻止导出。

### 9.6 示例

```js
window.__RS_TEMPLATE_VALIDATE__ = function (ctx) {
  const values = ctx.values;
  const issues = [];

  issues.push({
    status: values.project_name ? "ok" : "error",
    keys: ["project_name"],
    msg: values.project_name ? "" : "请填写工程名称",
  });

  if (values.plan_start_date && values.plan_end_date) {
    const valid = values.plan_end_date >= values.plan_start_date;
    issues.push({
      status: valid ? "ok" : "error",
      keys: ["plan_start_date", "plan_end_date"],
      key: "plan_end_date",
      msg: valid ? "" : "计划竣工日期不能早于计划开工日期",
    });
  }

  if (
    values.budget &&
    values.reported_amount &&
    Number(values.reported_amount) > Number(values.budget)
  ) {
    issues.push({
      status: "warning",
      keys: ["reported_amount"],
      msg: "报审金额超过预算金额",
    });
  }

  return issues;
};
```

### 9.7 开发服务器

`npm run dev:template -- <模板目录>` 也会加载同一个校验钩子：

- 页面加载后执行一次 `preview`。
- 修改输入框时执行 `change`。
- 点击“校验导出”按钮执行一次 `beforeExport`。
- 在左侧变量面板显示错误/警告，并刷新相关字段。
- 输入时的 `computed` 值不会即时重算，需要以“应用并刷新”后的结果为准。
- 开发服务器只展示校验结果，不阻止“应用并刷新”，方便调试不完整数据。

### 9.8 限制

- Worker 中没有 DOM，不能检查渲染后的布局。
- 钩子文件应保持单文件，不使用 `import` / `importScripts`。
- 不建议在钩子中请求外部网络；校验应尽量是纯函数。
- 不建议做同步死循环；宿主会在超时后终止 Worker。
- 钩子在当前前端 UI 中执行；绕过界面直接调用 PDF 命令不会触发校验。
- 钩子属于模板代码，仍然应该只从可信来源安装模板。

---

## 10. 常见注意事项

### 10.1 路径与目录

- `pages[].file` 是相对模板根目录的完整路径，`pagesPath` 不会自动补前缀。
- `assetsPath` 只负责自动引入 `base.css`。
- 页面资源用 `../assets/...`。
- CSS 内资源用相对 CSS 文件的路径。
- `include` 路径相对模板根目录。
- manifest 内统一使用正斜杠 `/`。
- `file`、`assetsPath`、`pagesPath` 尽量使用 ASCII 文件名和目录名。

### 10.2 版面与打印

- `fixed` 页内容超出会被裁切，不会自动缩小。
- 长文本优先考虑 `flow`，或使用 `fit.js` 类方案。
- `flow` 页避免固定高度、绝对定位、flex 撑满和内层 `overflow: hidden`。
- 表格建议 `table-layout: fixed`，用 `<colgroup>` 控制列宽。
- 长内容加 `word-break: break-all` 或 `overflow-wrap: anywhere`。
- 打印布局使用 `mm`、`pt`、`%`，少用 `px`。
- 表格线使用 `border`，不要依赖 `box-shadow` 或 `outline`。
- 导出 PDF 时 WebView2 打印边距被设为 `0`，实际内容位置主要靠 `mt/mr/mb/ml`。不要同时依赖 `margin` 和 padding 叠加。
- 导出是逐页单独打印后合并：
  - 没有自动页码和总页数
  - 没有浏览器页眉页脚
  - `toc` 不会写入 PDF 书签
  - 跨页页眉页脚需要自行设计
- 应用内预览按 A4 尺寸计算页面框，A3 模板请以导出 PDF 为准。
- `orientation` 建议严格写小写 `portrait` / `landscape`。
- `size` 当前只保证 `A4` / `A3`，其他值会按 A4 处理。
- 示例 `base.css` 中如果包含 `@media print .paper { page-break-after: always; }`，请保持 `.paper` 是页面最后一个顶层元素。

### 10.3 数据与计算

- `table` / `images` 的 `default` 不生效。
- `image` 单图 `default` 可用，适合模板内置图片。
- `sample` 只用于预览占位；设置了非空 `default` 后，预览通常显示 default。
- `required` 不拦截空数组，表格和图片需要模板自行处理空状态。
- `bool` 直接输出是 `true` / `false`。
- `select` / `multiselect` 的值是字符串。
- `datetime` 不能直接交给 `date_cn` / `date_dot`。
- 表达式中的变量优先使用 ASCII `key`，中文 alias 不能直接作为标识符。
- `computed` 表达式出错会被静默忽略，最多迭代 8 轮。
- `field` 的 key 使用变量 `key`，且只能用于文本位置。
- `field` 的输出是 `<span>`，注意对现有 CSS 选择器的影响。
- `money` 没有千分位和货币符号。
- `rmb_upper(0)` 当前输出 `零整`。
- `days_between` 包含首尾天数。
- 多行文本换行需要使用 `white-space: pre-wrap` 或自行处理。

### 10.4 图片资源

- 只支持 `png`、`jpg`、`jpeg`、`bmp`、`webp`、`gif`。
- 不支持 SVG 和 PDF。
- 上传资源按内容哈希命名，不要硬编码 `resources/<hash>.png`。
- 固定模板图片可以直接引用 `../assets/...`。
- 图片变量会自动改写为绝对 URL，直接用于 `src`。
- 不要对图片地址使用 `field`。

### 10.5 manifest 与发布

- manifest 必须是严格 JSON，不能有注释和尾逗号。
- 保存为 UTF-8 无 BOM。
- 字段名使用 camelCase：`schemaVersion`、`assetsPath`、`pagesPath`、`minAppVersion`、`contentHash`、`hooks`。
- 未知字段会被静默忽略，拼错不会报错。
- `schemaVersion` 当前没有版本校验，但必须存在，写 `1`。
- `id` 发布后不要修改。
- 模板内容更新后必须提升 `version`。
- `contentHash` 留空，安装时自动计算。
- `minAppVersion` 只影响远端同步。
- `variables[].key`、`pages[].file` 必须唯一。
- `toc[].title` 建议唯一，重复可能导致预览目录渲染异常。
- `toc.page` 是 1 基页码，增删页面后需要手动更新。

### 10.6 预览与导出差异

开发时建议至少验证三层：

1. 模板开发服务器：

   ```bash
   npm run dev:template -- src-tauri/templates/my-template
   ```

2. 桌面应用预览：

   ```bash
   npm run tauri dev
   ```

3. 导出 PDF / 系统打印。

三者渲染路径不完全一样：

- 开发服务器使用合并的多页文档。
- 桌面应用预览是每页单独的 iframe。
- PDF 导出是每页单独打印后合并。

因此开发服务器中正常，不代表导出一定正常。`flow` 页、A3、字体、边距和自动缩字号都需要用导出 PDF 再验证。

---

## 11. 上线前检查清单

- [ ] `variables[].key` 唯一
- [ ] `pages[].file` 唯一
- [ ] `toc[].title` 唯一
- [ ] `pages` 数组顺序和最终 PDF 顺序一致
- [ ] `toc.page` 已更新
- [ ] 所有图片资源存在，没有裂图
- [ ] 空值、超长文本、空表格、空图片数组都已预览
- [ ] 固定页没有内容被裁掉
- [ ] `flow` 页导出后第 2 页及以后留白正确
- [ ] 中文、数字、英文、标点字体正常
- [ ] A3 / 横版页面已用导出 PDF 验证
- [ ] PDF 页码、页眉页脚、目录符合预期
- [ ] manifest 是 UTF-8 无 BOM、严格 JSON
- [ ] `manifest.version` 已提升
- [ ] `manifest.id` 未修改
- [ ] zip 解压后 manifest.json 在根目录或唯一子目录下
- [ ] `hooks.validate` 文件存在且为经典脚本
- [ ] 校验的通过、警告、错误三种状态都已测试
- [ ] 多字段比较时两个字段都会刷新
- [ ] `beforeExport` 有错误时能阻止导出
- [ ] 模板开发服务器中能看到校验结果
- [ ] 校验脚本异常或超时会显示错误并阻止导出

最关键的三点：

1. 页面片段不要写完整 HTML 文档。
2. 固定页会裁切超出内容。
3. PDF 按 `pages` 数组顺序逐页单独打印。
