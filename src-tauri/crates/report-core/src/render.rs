use crate::model::{ColumnDef, PageDef, TemplateManifest, VariableDef};
use chrono::Datelike;
use minijinja::{Environment, Value as MjValue};
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn make_env(
    loader: Option<PathBuf>,
    samples: Option<Arc<HashSet<String>>>,
) -> Environment<'static> {
    let mut env = Environment::new();
    if let Some(dir) = loader {
        env.set_loader(minijinja::path_loader(dir));
    }
    // 无法解析日期时返回 undefined（渲染为空），不阻断整个报表。
    env.add_function("days_between", |a: String, b: String| -> MjValue {
        match (parse_date(&a), parse_date(&b)) {
            (Some(pa), Some(pb)) => MjValue::from((pb - pa).num_days() + 1),
            _ => MjValue::UNDEFINED,
        }
    });
    env.add_filter("money", |v: Option<f64>| -> String {
        match v {
            Some(f) => format!("{:.2}", f),
            None => String::new(),
        }
    });
    // 日期：输入 ISO 或常见格式，输出中文/点号格式。
    env.add_filter("date_cn", |v: Option<String>| -> String {
        match v.as_deref().and_then(parse_date) {
            Some(d) => format!("{}年{}月{}日", d.year(), d.month(), d.day()),
            None => v.unwrap_or_default(),
        }
    });
    env.add_filter("date_dot", |v: Option<String>| -> String {
        match v.as_deref().and_then(parse_date) {
            Some(d) => format!("{}.{}.{}", d.year(), d.month(), d.day()),
            None => v.unwrap_or_default(),
        }
    });
    // 金额转中文大写（函数 + 过滤器两种用法）
    env.add_function("rmb_upper", |v: Option<f64>| -> String { rmb_upper(v) });
    env.add_filter("rmb_upper", |v: Option<f64>| -> String { rmb_upper(v) });
    // 字段标记：把变量出现处包成一个带 data-field 的 <span>（返回安全 HTML，绕过转义）。
    // 若该值来自 preview 占位（sample），额外加 rs-sample 标记（前端用橙色荧光笔）。
    env.add_filter(
        "field",
        move |value: MjValue, key: Option<String>| -> MjValue {
            let key = key.unwrap_or_default();
            let is_sample = samples.as_ref().map_or(false, |s| s.contains(&key));
            let text = if value.is_undefined() || value.is_none() {
                String::new()
            } else {
                value.to_string()
            };
            let class = if is_sample {
                "rs-field rs-sample"
            } else {
                "rs-field"
            };
            let sample_attr = if is_sample { " data-sample=\"1\"" } else { "" };
            MjValue::from_safe_string(format!(
                "<span class=\"{}\" data-field=\"{}\"{}>{}</span>",
                class,
                html_escape(&key),
                sample_attr,
                html_escape(&text)
            ))
        },
    );
    env
}

fn html_escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            _ => o.push(c),
        }
    }
    o
}

/// 数字金额转中文大写（如 135613.17 → 壹拾叁万伍仟陆佰壹拾叁元壹角柒分）。
fn rmb_upper(v: Option<f64>) -> String {
    let Some(mut num) = v else {
        return String::new();
    };
    if !num.is_finite() {
        return String::new();
    }
    let neg = num < 0.0;
    num = num.abs();
    num = (num * 100.0).round() / 100.0;

    let digits = ['零', '壹', '贰', '叁', '肆', '伍', '陆', '柒', '捌', '玖'];
    let unit_names = ["仟", "佰", "拾", ""];
    let group_units = ["", "万", "亿", "兆"];

    let mut yuan = num.trunc() as i64;
    let mut cents = ((num - num.trunc()) * 100.0).round() as i64;
    if cents >= 100 {
        cents -= 100;
        yuan += 1;
    }
    let jiao = (cents / 10) as usize;
    let fen = (cents % 10) as usize;

    let group_to = |g: i64| -> String {
        let ds = [(g / 1000) % 10, (g / 100) % 10, (g / 10) % 10, g % 10];
        let mut s = String::new();
        let mut zero = false;
        for i in 0..4 {
            let x = ds[i] as usize;
            if x == 0 {
                if !s.is_empty() {
                    zero = true;
                }
            } else {
                if zero {
                    s.push('零');
                    zero = false;
                }
                s.push(digits[x]);
                s.push_str(unit_names[i]);
            }
        }
        s
    };

    let mut out = String::new();
    if yuan == 0 {
        out.push('零');
    } else {
        // 拆分为 4 位一组（低位在前）
        let mut groups: Vec<i64> = Vec::new();
        let mut y = yuan;
        while y > 0 {
            groups.push(y % 10000);
            y /= 10000;
        }
        let mut need_zero = false;
        for idx in (0..groups.len()).rev() {
            let g = groups[idx];
            if g == 0 {
                if !out.is_empty() {
                    need_zero = true;
                }
                continue;
            }
            let s = group_to(g);
            if !out.is_empty() && (need_zero || g < 1000) && !out.ends_with('零') {
                out.push('零');
            }
            out.push_str(&s);
            if idx > 0 {
                out.push_str(group_units[idx]);
            }
            need_zero = false;
        }
        out.push('元');
    }

    if jiao == 0 && fen == 0 {
        out.push('整');
    } else if jiao == 0 {
        out.push('零');
        out.push(digits[fen]);
        out.push('分');
    } else {
        out.push(digits[jiao]);
        out.push('角');
        if fen > 0 {
            out.push(digits[fen]);
            out.push('分');
        }
    }

    if neg {
        format!("负{out}")
    } else {
        out
    }
}

fn parse_date(s: &str) -> Option<chrono::NaiveDate> {
    let s = s.trim();
    for fmt in ["%Y-%m-%d", "%Y.%m.%d", "%Y/%m/%d", "%Y年%m月%d日"] {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(s, fmt) {
            return Some(d);
        }
    }
    None
}

fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c == '_' || c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

/// 变量可用的顶层标识符：优先 alias；否则若 key 本身是 ASCII 标识符则用 key。
fn var_alias(v: &VariableDef) -> Option<String> {
    if let Some(a) = &v.alias {
        if is_ident(a) {
            return Some(a.clone());
        }
    }
    if is_ident(&v.key) {
        Some(v.key.clone())
    } else {
        None
    }
}

fn col_alias(c: &ColumnDef) -> Option<String> {
    if let Some(a) = &c.alias {
        if is_ident(a) {
            return Some(a.clone());
        }
    }
    if is_ident(&c.key) {
        Some(c.key.clone())
    } else {
        None
    }
}

/// 顶层上下文：所有带别名的标量作为标识符暴露，另附 `v` = 完整字段映射。
fn base_ctx(manifest: &TemplateManifest, ctx: &Map<String, Value>) -> Value {
    let mut m = Map::new();
    for v in &manifest.variables {
        if let Some(alias) = var_alias(v) {
            if let Some(val) = ctx.get(&v.key) {
                m.insert(alias, val.clone());
            }
        }
    }
    m.insert("v".to_string(), Value::Object(ctx.clone()));
    Value::Object(m)
}

/// 表格行上下文：行内列别名作为标识符暴露，另附 `r` = 行对象、`v` = 全局。
fn row_ctx(cols: &[ColumnDef], ctx: &Map<String, Value>, row: &Map<String, Value>) -> Value {
    let mut m = Map::new();
    for c in cols {
        if let Some(alias) = col_alias(c) {
            if let Some(val) = row.get(&c.key) {
                m.insert(alias, val.clone());
            }
        }
    }
    m.insert("r".to_string(), Value::Object(row.clone()));
    m.insert("v".to_string(), Value::Object(ctx.clone()));
    Value::Object(m)
}

fn eval_expr(env: &Environment, context: &Value, expr: &str) -> Result<Value, String> {
    let src = format!("{{{{ {} }}}}", expr);
    let rendered = env.render_str(&src, context).map_err(|e| e.to_string())?;
    let s = rendered.trim();
    if let Ok(i) = s.parse::<i64>() {
        return Ok(Value::from(i));
    }
    if let Ok(f) = s.parse::<f64>() {
        return Ok(Value::from(f));
    }
    if s == "true" {
        return Ok(Value::from(true));
    }
    if s == "false" {
        return Ok(Value::from(false));
    }
    Ok(Value::from(s))
}

/// 给每个表格行对象补上"列别名"键，便于模板用 row.send_amount 访问。
fn inject_row_aliases(manifest: &TemplateManifest, ctx: &mut Map<String, Value>) {
    for v in &manifest.variables {
        if v.ty != "table" {
            continue;
        }
        let Some(cols) = &v.columns else { continue };
        let Some(Value::Array(arr)) = ctx.get(&v.key).cloned() else {
            continue;
        };
        let mut new_arr = Vec::with_capacity(arr.len());
        for row in arr {
            match row {
                Value::Object(mut obj) => {
                    for c in cols {
                        if let Some(alias) = col_alias(c) {
                            if let Some(val) = obj.get(&c.key).cloned() {
                                obj.insert(alias, val);
                            }
                        }
                    }
                    new_arr.push(Value::Object(obj));
                }
                other => new_arr.push(other),
            }
        }
        ctx.insert(v.key.clone(), Value::Array(new_arr));
    }
}

/// 计算派生字段（computed 变量、table 列），迭代到稳定。
pub fn resolve_values(
    manifest: &TemplateManifest,
    values: &Map<String, Value>,
) -> Result<Map<String, Value>, String> {
    let env = make_env(None, None);
    let mut ctx = values.clone();

    for _ in 0..8 {
        let mut changed = false;

        for v in &manifest.variables {
            if v.ty == "computed" {
                if let Some(expr) = &v.expression {
                    if let Ok(val) = eval_expr(&env, &base_ctx(manifest, &ctx), expr) {
                        if ctx.get(&v.key) != Some(&val) {
                            ctx.insert(v.key.clone(), val);
                            changed = true;
                        }
                    }
                }
                continue;
            }

            if v.ty == "table" {
                if let Some(cols) = &v.columns {
                    if let Some(Value::Array(arr)) = ctx.get(&v.key).cloned() {
                        let mut new_arr = Vec::with_capacity(arr.len());
                        for row in arr {
                            match row {
                                Value::Object(mut obj) => {
                                    for c in cols {
                                        if let Some(cexpr) = &c.expression {
                                            if let Ok(val) =
                                                eval_expr(&env, &row_ctx(cols, &ctx, &obj), cexpr)
                                            {
                                                obj.insert(c.key.clone(), val);
                                            }
                                        }
                                    }
                                    new_arr.push(Value::Object(obj));
                                }
                                other => new_arr.push(other),
                            }
                        }
                        let nv = Value::Array(new_arr);
                        if ctx.get(&v.key) != Some(&nv) {
                            ctx.insert(v.key.clone(), nv);
                            changed = true;
                        }
                    }
                }
            }
        }

        if !changed {
            break;
        }
    }

    inject_row_aliases(manifest, &mut ctx);
    Ok(ctx)
}

/// A4（或指定纸张）在某方向下的宽高（mm）。
fn page_dims_mm(size: &str, orientation: &str) -> (f64, f64) {
    let landscape = orientation.eq_ignore_ascii_case("landscape");
    match size.to_ascii_uppercase().as_str() {
        "A3" => {
            if landscape {
                (420.0, 297.0)
            } else {
                (297.0, 420.0)
            }
        }
        _ => {
            // 默认 A4
            if landscape {
                (297.0, 210.0)
            } else {
                (210.0, 297.0)
            }
        }
    }
}

fn page_dims(size: &str, orientation: &str) -> (String, String) {
    let (w, h) = page_dims_mm(size, orientation);
    (format!("{w}mm"), format!("{h}mm"))
}

/// 返回页面实际宽高（mm），用于 PDF 打印任务设置纸张尺寸。
pub fn page_size_mm(page: &PageDef) -> (f64, f64) {
    let size = page.size.clone().unwrap_or_else(|| "A4".into());
    let orientation = page
        .orientation
        .clone()
        .unwrap_or_else(|| "portrait".into());
    page_dims_mm(&size, &orientation)
}

/// 由页面几何生成 @page 规则与 CSS 变量。
/// `named = Some(n)` 用于打印合并文档（命名页 pgN，作用于 `[data-page="pgN"]`）；
/// `None` 用于单页预览（直接作用于 `.paper`）。
fn page_geometry(page: &PageDef, named: Option<i64>) -> String {
    let size = page.size.clone().unwrap_or_else(|| "A4".into());
    let orientation = page
        .orientation
        .clone()
        .unwrap_or_else(|| "portrait".into());
    let margin = page.margin.clone().unwrap_or_else(|| "0".into());
    let (w, h) = page_dims(&size, &orientation);
    let mt = page.mt.clone().unwrap_or_else(|| "0mm".into());
    let mr = page.mr.clone().unwrap_or_else(|| "0mm".into());
    let mb = page.mb.clone().unwrap_or_else(|| "0mm".into());
    let ml = page.ml.clone().unwrap_or_else(|| "0mm".into());
    let vars = format!("--a4w: {w}; --a4h: {h}; --mt: {mt}; --mr: {mr}; --mb: {mb}; --ml: {ml};");
    match named {
        Some(n) => format!(
            "@page pg{n} {{ size: {size} {orientation}; margin: {margin}; }}\n[data-page=\"pg{n}\"] {{ page: pg{n}; {vars} }}"
        ),
        None => format!("@page {{ size: {size} {orientation}; margin: {margin}; }}\n.paper {{ {vars} }}"),
    }
}

/// 流式页样式：解除固定页高/裁切，让排版引擎在超出页框时自动续页；
/// 表格行禁止跨页断开；屏幕下叠加每页高度的分页参考线（打印时不输出）。
const FLOW_CSS: &str = r#"
.paper[data-flow]{height:auto;}
[data-flow] .page{height:auto;overflow:visible;}
[data-flow] table.doc-tbl tr,
[data-flow] table.doc-tbl thead{break-inside:avoid;page-break-inside:avoid;}
@media screen{
  .paper[data-flow]{position:relative;}
  .paper[data-flow]::after{content:"";position:absolute;inset:0;pointer-events:none;z-index:5;
    background-image:repeating-linear-gradient(to bottom,
      transparent 0,transparent calc(var(--a4h) - 1px),
      rgba(10,114,239,.35) calc(var(--a4h) - 1px),rgba(10,114,239,.35) var(--a4h));}
}
"#;

/// 给片段的根 `.paper` 打上流式标记。
fn with_flow_attr(content: &str) -> String {
    content.replacen("class=\"paper\"", "class=\"paper\" data-flow", 1)
}

/// 把单页片段补全成完整 HTML 文档字符串（供前端 srcdoc 注入）。
/// `base_href` 为绝对基准（如 `http://report.localhost/m/<id>/pages/`），
/// 使片段内 `../assets/…` 等相对引用可解析到挂载路由。
fn wrap_page(
    content: &str,
    page: &PageDef,
    base_href: &str,
    assets_path: &str,
    thumb: bool,
) -> String {
    // 若模板本身是完整文档则原样返回（向后兼容旧模板）。
    if content.to_ascii_lowercase().contains("<html") {
        return content.to_string();
    }
    let script = PREVIEW_JS.replace("__RS_PAGE__", &page.file);
    let thumb_flag = if thumb {
        "<script>window.__RS_THUMB=1;</script>"
    } else {
        ""
    };
    let (content, flow_css) = if page.is_flow() {
        (with_flow_attr(content), FLOW_CSS)
    } else {
        (content.to_string(), "")
    };
    let preview_css = if thumb { "" } else { PREVIEW_CSS };
    format!(
        "<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\">\n<base href=\"{base_href}\">\n<link rel=\"stylesheet\" href=\"../{assets_path}/base.css\">\n<style>\n{}{}{}\n</style></head><body>\n{}\n{thumb_flag}<script>\n{}\n</script>\n</body></html>\n",
        page_geometry(page, None),
        flow_css,
        preview_css,
        content,
        script
    )
}

/// 预览用内置样式：字段标记、hover/全部高亮、sample 蓝色高亮。
/// 由渲染层统一注入，模板无需重复定义。
pub const PREVIEW_CSS: &str = r#"
/* 预览字段标记（缩略图不加载 .rs-annot，因而不显示标记） */
.rs-annot .rs-field{border-bottom:1px dashed rgba(10,114,239,.55);}
.rs-annot .rs-field:hover,
.rs-annot .rs-field.rs-hit,
.rs-annot .rs-field.rs-hover,
.rs-annot.rs-all .rs-field{
  border-bottom-color:transparent;
  background-image:linear-gradient(180deg,transparent 56%,#ffe58f 56%);
  border-radius:2px;
}
/* 由 sample 占位渲染的字段：始终蓝色荧光笔（与黄色区分） */
.rs-annot .rs-field.rs-sample{
  border-bottom-color:transparent;
  background-image:linear-gradient(180deg,transparent 56%,#a5d8ff 56%);
  border-radius:2px;
}
"#;

/// 预览页注入脚本：收集 `.rs-field` 标记（含页面内矩形）并上报父窗口；响应定位/悬停高亮。
const PREVIEW_JS: &str = r#"(function () {
  if (window.__RS_THUMB) return;
  var PAGE = "__RS_PAGE__";
  document.documentElement.classList.add("rs-annot");
  var hoverEl = null;
  function contextFor(el) {
    var c = el.closest("td,p,li") || el.parentElement || el;
    var full = (c.textContent || "").replace(/\s+/g, " ").trim();
    var val = (el.textContent || "").trim();
    var idx = val ? full.indexOf(val) : -1;
    var a = idx >= 0 ? idx : 0;
    var b = a + val.length;
    return { before: full.slice(Math.max(0, a - 16), a), after: full.slice(b, b + 16) };
  }
  function collect() {
    var els = document.querySelectorAll(".rs-field");
    var items = [];
    for (var i = 0; i < els.length; i++) {
      var el = els[i];
      var id = "rsf-" + i;
      el.setAttribute("data-rs-id", id);
      var ctx = contextFor(el);
      var r = el.getBoundingClientRect();
      items.push({
        id: id,
        field: el.getAttribute("data-field") || "",
        text: (el.textContent || "").trim(),
        before: ctx.before,
        after: ctx.after,
        x: r.left,
        y: r.top,
        w: r.width,
        h: r.height
      });
    }
    if (parent !== window) parent.postMessage({ type: "rs-fields", page: PAGE, items: items }, "*");
  }
  function locate(id) {
    var el = document.querySelector('[data-rs-id="' + id + '"]');
    if (!el) return;
    el.classList.add("rs-hit");
    window.setTimeout(function () { el.classList.remove("rs-hit"); }, 1600);
    var r = el.getBoundingClientRect();
    var top = r.top + (window.pageYOffset || 0);
    if (parent !== window) parent.postMessage({ type: "rs-located", page: PAGE, id: id, top: top }, "*");
  }
  function setHover(id) {
    if (hoverEl) { hoverEl.classList.remove("rs-hover"); hoverEl = null; }
    if (id) {
      var el = document.querySelector('[data-rs-id="' + id + '"]');
      if (el) { el.classList.add("rs-hover"); hoverEl = el; }
    }
  }
  window.addEventListener("message", function (ev) {
    var d = ev.data || {};
    if (d.type === "rs-locate") locate(d.id);
    else if (d.type === "rs-hover") setHover(d.id || null);
    else if (d.type === "rs-highlight-all")
      document.documentElement.classList.toggle("rs-all", !!d.on);
    else if (d.type === "rs-clear") {
      var hs = document.querySelectorAll(".rs-hit");
      for (var i = 0; i < hs.length; i++) hs[i].classList.remove("rs-hit");
    }
  });
  function flowRoot() {
    return document.querySelector("[data-flow] .page") || document.querySelector("[data-flow]");
  }
  function reportSize() {
    var root = flowRoot();
    if (!root) return;
    var h = Math.ceil(root.getBoundingClientRect().height);
    if (!h) return;
    if (parent !== window) parent.postMessage({ type: "rs-flow-size", page: PAGE, height: h }, "*");
  }
  function settle() { collect(); reportSize(); }
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", settle);
  else settle();
  window.addEventListener("load", settle);
  if (document.fonts && document.fonts.ready) { try { document.fonts.ready.then(settle); } catch (e) {} }
  try {
    var observed = flowRoot() || document.body;
    new ResizeObserver(function () { reportSize(); }).observe(observed);
  } catch (e) {}
})();"#;

/// 逐页渲染成完整文档字符串（不落盘、不拷贝模板）：返回 (file, html, thumb_html)。
pub fn render_preview_html(
    template_dir: &Path,
    manifest: &TemplateManifest,
    values: &Map<String, Value>,
    pages: &[PageDef],
    base_href: &str,
) -> Result<Vec<(String, String, String)>, String> {
    // 空值用 sample 占位填充（仅预览），并记录哪些键用了占位。
    let mut filled = values.clone();
    let mut sample_keys: HashSet<String> = HashSet::new();
    for v in &manifest.variables {
        if v.ty == "computed" {
            continue;
        }
        let empty = match filled.get(&v.key) {
            None | Some(Value::Null) => true,
            Some(Value::String(s)) => s.is_empty(),
            Some(Value::Array(a)) => a.is_empty(),
            _ => false,
        };
        if empty {
            if let Some(sample) = &v.sample {
                filled.insert(v.key.clone(), sample.clone());
                sample_keys.insert(v.key.clone());
            }
        }
    }

    let resolved = resolve_values(manifest, &filled)?;
    let env = make_env(
        Some(template_dir.to_path_buf()),
        Some(Arc::new(sample_keys)),
    );
    let context = base_ctx(manifest, &resolved);
    let assets_path = manifest.assets_path();

    let mut out = Vec::with_capacity(pages.len());
    for page in pages {
        let tmpl = env
            .get_template(&page.file)
            .map_err(|e| format!("加载页面 {} 失败: {e}", page.file))?;
        let raw = tmpl
            .render(&context)
            .map_err(|e| format!("渲染页面 {} 失败: {e}", page.file))?;
        let html = wrap_page(&raw, page, base_href, &assets_path, false);
        let thumb_html = wrap_page(&raw, page, base_href, &assets_path, true);
        out.push((page.file.clone(), html, thumb_html));
    }
    Ok(out)
}

/// 由所有页面组装成一份合并文档字符串（命名 @page 支持横竖混排）。
pub fn render_print_html(
    template_dir: &Path,
    manifest: &TemplateManifest,
    values: &Map<String, Value>,
) -> Result<String, String> {
    render_print_html_with_samples(template_dir, manifest, values, None)
}

/// 同 `render_print_html`，但允许传入 sample 字段集合，用于开发服务器等预览场景。
pub fn render_print_html_with_samples(
    template_dir: &Path,
    manifest: &TemplateManifest,
    values: &Map<String, Value>,
    samples: Option<Arc<HashSet<String>>>,
) -> Result<String, String> {
    let resolved = resolve_values(manifest, values)?;
    let env = make_env(Some(template_dir.to_path_buf()), samples);
    let context = base_ctx(manifest, &resolved);
    let assets_path = manifest.assets_path();
    let pages_path = manifest.pages_path();

    let mut body = String::new();
    let mut geometry = String::new();
    let mut has_flow = false;
    for (i, page) in manifest.pages.iter().enumerate() {
        let n = i as i64 + 1;
        let tmpl = env
            .get_template(&page.file)
            .map_err(|e| format!("加载页面 {} 失败: {e}", page.file))?;
        let raw = tmpl
            .render(&context)
            .map_err(|e| format!("渲染页面 {} 失败: {e}", page.file))?;
        // 给该页的 .paper 打上命名页标记，供 [data-page] 选择器命中
        let mut frag = raw.replacen(
            "class=\"paper\"",
            &format!("class=\"paper\" data-page=\"pg{n}\""),
            1,
        );
        if page.is_flow() {
            frag = frag.replacen("class=\"paper\"", "class=\"paper\" data-flow", 1);
            has_flow = true;
        }
        body.push_str(&frag);
        body.push('\n');
        geometry.push_str(&page_geometry(page, Some(n)));
        geometry.push('\n');
    }
    let flow_css = if has_flow { FLOW_CSS } else { "" };

    // 页面片段里的相对路径是相对页面目录写的，合并文档经 <base> 统一基准。
    Ok(format!(
        "<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\">\n<base href=\"{pages_path}/\">\n<link rel=\"stylesheet\" href=\"../{assets_path}/base.css\">\n<style>\n{geometry}{flow_css}\n</style></head><body>\n{body}</body></html>\n"
    ))
}

/// 只渲染 manifest 中的一个 page，用于按页设置纸张尺寸后单独打印。
/// flow page 仍由浏览器在打印时自动续页。
pub fn render_print_page_html(
    template_dir: &Path,
    manifest: &TemplateManifest,
    values: &Map<String, Value>,
    page: &PageDef,
    base_href: &str,
) -> Result<String, String> {
    let resolved = resolve_values(manifest, values)?;
    let env = make_env(Some(template_dir.to_path_buf()), None);
    let context = base_ctx(manifest, &resolved);
    let assets_path = manifest.assets_path();

    let tmpl = env
        .get_template(&page.file)
        .map_err(|e| format!("加载页面 {} 失败: {e}", page.file))?;
    let raw = tmpl
        .render(&context)
        .map_err(|e| format!("渲染页面 {} 失败: {e}", page.file))?;

    let (content, flow_css) = if page.is_flow() {
        (with_flow_attr(&raw), FLOW_CSS)
    } else {
        (raw, "")
    };

    Ok(format!(
        "<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\">\n<base href=\"{base_href}\">\n<link rel=\"stylesheet\" href=\"../{assets_path}/base.css\">\n<style>\n{}{}\n</style></head><body>\n{content}\n</body></html>\n",
        page_geometry(page, None),
        flow_css,
    ))
}

/// 把 image/images 变量值改写为绝对挂载 URL，供模板直接用于 `src`。
/// 值以路由前缀区分来源：`assets/…`=模板资源、`resources/…`=工程资源；
/// 统一改写为 `mount_root + 值`（挂载根形如 `http://report.localhost/m/<id>/`）。
/// 空值、已是绝对 URL 或 data URI 的值保持不变。
pub fn rewrite_resource_urls(
    manifest: &TemplateManifest,
    values: &Map<String, Value>,
    mount_root: &str,
) -> Map<String, Value> {
    let is_abs =
        |s: &str| s.starts_with("http://") || s.starts_with("https://") || s.starts_with("data:");
    let mut out = values.clone();
    for v in &manifest.variables {
        if v.ty != "image" && v.ty != "images" {
            continue;
        }
        match out.get(&v.key) {
            Some(Value::Array(arr)) => {
                let na: Vec<Value> = arr
                    .iter()
                    .map(|x| match x {
                        Value::String(s) if !s.is_empty() && !is_abs(s) => {
                            Value::String(format!("{mount_root}{s}"))
                        }
                        other => other.clone(),
                    })
                    .collect();
                out.insert(v.key.clone(), Value::Array(na));
            }
            Some(Value::String(s)) if !s.is_empty() && !is_abs(s) => {
                out.insert(v.key.clone(), Value::String(format!("{mount_root}{s}")));
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{PageDef, TemplateManifest};

    fn manifest_with(page_mode: Option<&str>) -> TemplateManifest {
        TemplateManifest {
            schema_version: 1,
            id: "t".into(),
            version: "1".into(),
            content_hash: String::new(),
            name: "t".into(),
            min_app_version: None,
            assets_path: None,
            pages_path: None,
            variables: vec![],
            pages: vec![PageDef {
                file: "pages/f1.html".into(),
                mode: page_mode.map(|s| s.to_string()),
                ..Default::default()
            }],
            toc: vec![],
            hooks: Default::default(),
        }
    }

    #[test]
    fn page_size_mm_respects_orientation() {
        let portrait = PageDef {
            size: Some("A4".into()),
            orientation: Some("portrait".into()),
            ..Default::default()
        };
        let landscape = PageDef {
            orientation: Some("landscape".into()),
            ..portrait.clone()
        };
        assert_eq!(page_size_mm(&portrait), (210.0, 297.0));
        assert_eq!(page_size_mm(&landscape), (297.0, 210.0));
    }

    #[test]
    fn is_flow_flag() {
        assert!(!PageDef::default().is_flow());
        assert!(PageDef {
            mode: Some("flow".into()),
            ..Default::default()
        }
        .is_flow());
        assert!(!PageDef {
            mode: Some("fixed".into()),
            ..Default::default()
        }
        .is_flow());
    }

    #[test]
    fn print_page_uses_explicit_base_href() {
        let dir = std::env::temp_dir().join(format!("rs-print-page-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("pages")).unwrap();
        std::fs::write(
            dir.join("pages/f1.html"),
            "<div class=\"paper\"><div class=\"page\">x</div></div>",
        )
        .unwrap();

        let m = manifest_with(None);
        let base = "http://report.localhost/m/t/pages/";
        let html = render_print_page_html(&dir, &m, &Map::new(), &m.pages[0], base).unwrap();

        assert!(html.contains(&format!("<base href=\"{base}\">")));
        assert!(html.contains("../assets/base.css"));
        assert!(!html.contains("__RS_THUMB"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn flow_print_doc_marks_and_unfixes() {
        let dir = std::env::temp_dir().join(format!("rs-flow-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("pages")).unwrap();
        std::fs::write(
            dir.join("pages/f1.html"),
            "<div class=\"paper\"><div class=\"page\">x</div></div>",
        )
        .unwrap();

        let flow = render_print_html(&dir, &manifest_with(Some("flow")), &Map::new()).unwrap();
        assert!(flow.contains("data-flow"));
        assert!(flow.contains("repeating-linear-gradient"));
        assert!(flow.contains("data-page=\"pg1\""));

        let fixed = render_print_html(&dir, &manifest_with(None), &Map::new()).unwrap();
        assert!(!fixed.contains("data-flow"));
        assert!(!fixed.contains("repeating-linear-gradient"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rewrite_resource_urls_prefixes_names() {
        let m: TemplateManifest = serde_json::from_str(
            r#"{
              "schemaVersion": 1, "id": "t", "version": "1", "name": "t",
              "variables": [
                { "key": "logo", "type": "image" },
                { "key": "photos", "type": "images" },
                { "key": "title", "type": "string" }
              ]
            }"#,
        )
        .unwrap();

        let mut values = Map::new();
        values.insert("logo".into(), Value::from("assets/logo.png"));
        values.insert(
            "photos".into(),
            Value::Array(vec![Value::from("resources/11aa22bb.jpg"), Value::from("")]),
        );
        values.insert("title".into(), Value::from("hello"));

        let out = rewrite_resource_urls(&m, &values, "http://x/m/id/");
        assert_eq!(
            out.get("logo").unwrap(),
            &Value::from("http://x/m/id/assets/logo.png")
        );
        assert_eq!(
            out.get("photos").unwrap(),
            &Value::Array(vec![
                Value::from("http://x/m/id/resources/11aa22bb.jpg"),
                Value::from("")
            ])
        );
        // 非图片变量不受影响
        assert_eq!(out.get("title").unwrap(), &Value::from("hello"));
    }

    #[test]
    fn assets_and_pages_path_override() {
        let mut m = manifest_with(None);
        m.assets_path = Some("static".into());
        m.pages_path = Some("docs".into());
        let dir = std::env::temp_dir().join(format!("rs-paths-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("docs")).unwrap();
        std::fs::write(
            dir.join("docs/f1.html"),
            "<div class=\"paper\"><div class=\"page\">x</div></div>",
        )
        .unwrap();
        m.pages[0].file = "docs/f1.html".into();

        let html = render_print_html(&dir, &m, &Map::new()).unwrap();
        assert!(html.contains("<base href=\"docs/\">"));
        assert!(html.contains("../static/base.css"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn flow_preview_doc_reports_size() {
        let dir = std::env::temp_dir().join(format!("rs-flow-prev-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("pages")).unwrap();
        std::fs::write(
            dir.join("pages/f1.html"),
            "<div class=\"paper\"><div class=\"page\">x</div></div>",
        )
        .unwrap();

        let base = "http://report.localhost/m/t/pages/";
        let m = manifest_with(Some("flow"));
        let pages = render_preview_html(&dir, &m, &Map::new(), &m.pages, base).unwrap();
        assert_eq!(pages.len(), 1);
        let (file, html, thumb) = &pages[0];
        assert_eq!(file, "pages/f1.html");
        assert!(html.contains("data-flow"));
        assert!(html.contains("repeating-linear-gradient"));
        assert!(html.contains(&format!("<base href=\"{base}\">")));
        assert!(html.contains("rs-flow-size"));
        assert!(html.contains("flowRoot"));
        assert!(html.contains("getBoundingClientRect().height"));
        assert!(thumb.contains("window.__RS_THUMB=1"));
        assert!(!html.contains("window.__RS_THUMB=1"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn preview_injects_builtin_css_and_marks_samples() {
        let dir = std::env::temp_dir().join(format!("rs-preview-css-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("pages")).unwrap();
        std::fs::write(
            dir.join("pages/f1.html"),
            "<div class=\"paper\"><div class=\"page\">{{ name|field('name') }}</div></div>",
        )
        .unwrap();

        let m: TemplateManifest = serde_json::from_str(
            r#"{
              "schemaVersion": 1, "id": "t", "version": "1", "name": "t",
              "variables": [
                { "key": "name", "alias": "名称", "type": "string", "sample": "示例" }
              ],
              "pages": [{ "file": "pages/f1.html" }]
            }"#,
        )
        .unwrap();

        let pages = render_preview_html(&dir, &m, &Map::new(), &m.pages, "http://x/").unwrap();
        let (_, html, thumb) = &pages[0];

        assert!(html.contains("rs-field rs-sample"), "sample 标记未生成");
        assert!(
            html.contains(".rs-annot .rs-field.rs-sample"),
            "内置预览样式未注入"
        );
        assert!(html.contains("示例"));
        assert!(
            !thumb.contains(".rs-annot .rs-field.rs-sample"),
            "缩略图不应注入预览样式"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
