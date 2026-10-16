use crate::model::{ColumnDef, PageDef, TemplateManifest, VariableDef};
use chrono::Datelike;
use minijinja::{Environment, Value as MjValue};
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn make_env(loader: Option<PathBuf>, samples: Option<Arc<HashSet<String>>>) -> Environment<'static> {
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
    env.add_filter("field", move |value: MjValue, key: Option<String>| -> MjValue {
        let key = key.unwrap_or_default();
        let is_sample = samples
            .as_ref()
            .map_or(false, |s| s.contains(&key));
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
    });
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

/// 将模板渲染进 out_dir，返回渲染后的入口 HTML 路径。
pub fn render_to_dir(
    template_dir: &Path,
    manifest: &TemplateManifest,
    values: &Map<String, Value>,
    out_dir: &Path,
) -> Result<PathBuf, String> {
    crate::templates::copy_dir(template_dir, out_dir)?;

    let resolved = resolve_values(manifest, values)?;
    let env = make_env(Some(template_dir.to_path_buf()), None);
    let tmpl = env
        .get_template(&manifest.entry)
        .map_err(|e| format!("加载入口模板失败: {e}"))?;
    let html = tmpl
        .render(&base_ctx(manifest, &resolved))
        .map_err(|e| format!("渲染失败: {e}"))?;

    let entry_path = out_dir.join(&manifest.entry);
    if let Some(parent) = entry_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&entry_path, html).map_err(|e| e.to_string())?;
    Ok(entry_path)
}

pub fn new_render_dir(base: &Path) -> Result<PathBuf, String> {
    let dir = base.join(format!("render-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// A4（或指定纸张）在某方向下的宽高（mm）。
fn page_dims(size: &str, orientation: &str) -> (String, String) {
    let landscape = orientation.eq_ignore_ascii_case("landscape");
    match size.to_ascii_uppercase().as_str() {
        "A3" => {
            if landscape {
                ("420mm".into(), "297mm".into())
            } else {
                ("297mm".into(), "420mm".into())
            }
        }
        _ => {
            // 默认 A4
            if landscape {
                ("297mm".into(), "210mm".into())
            } else {
                ("210mm".into(), "297mm".into())
            }
        }
    }
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
    let vars = format!(
        "--a4w: {w}; --a4h: {h}; --mt: {mt}; --mr: {mr}; --mb: {mb}; --ml: {ml};"
    );
    match named {
        Some(n) => format!(
            "@page pg{n} {{ size: {size} {orientation}; margin: {margin}; }}\n[data-page=\"pg{n}\"] {{ page: pg{n}; {vars} }}"
        ),
        None => format!("@page {{ size: {size} {orientation}; margin: {margin}; }}\n.paper {{ {vars} }}"),
    }
}

/// 把单页片段包成一个完整 HTML 文档（供预览 iframe 加载）。
fn wrap_page(content: &str, page: &PageDef) -> String {
    // 若模板本身是完整文档则原样返回（向后兼容旧模板）。
    if content.to_ascii_lowercase().contains("<html") {
        return content.to_string();
    }
    let script = PREVIEW_JS.replace("__RS_PAGE__", &page.file);
    format!(
        "<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\">\n<link rel=\"stylesheet\" href=\"../assets/base.css\">\n<style>\n{}\n</style></head><body>\n{}\n<script>\n{}\n</script>\n</body></html>\n",
        page_geometry(page, None),
        content,
        script
    )
}

/// 预览页注入脚本：收集 `.rs-field` 标记（含页面内矩形）并上报父窗口；响应定位/悬停高亮。
const PREVIEW_JS: &str = r#"(function () {
  if (location.search.indexOf("thumb") >= 0) return;
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
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", collect);
  else collect();
})();"#;

/// 逐页渲染成独立的预览文档并写入 out_dir（页面文件原样路径）。
pub fn render_preview_pages(
    template_dir: &Path,
    manifest: &TemplateManifest,
    values: &Map<String, Value>,
    out_dir: &Path,
    pages: &[PageDef],
) -> Result<(), String> {
    crate::templates::copy_dir(template_dir, out_dir)?;

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

    for page in pages {
        let tmpl = env
            .get_template(&page.file)
            .map_err(|e| format!("加载页面 {} 失败: {e}", page.file))?;
        let raw = tmpl
            .render(&context)
            .map_err(|e| format!("渲染页面 {} 失败: {e}", page.file))?;
        let doc = wrap_page(&raw, page);
        let path = out_dir.join(&page.file);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&path, doc).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 由所有页面组装成一份可打印的合并文档（命名 @page 支持横竖混排），返回入口路径。
pub fn render_print_document(
    template_dir: &Path,
    manifest: &TemplateManifest,
    values: &Map<String, Value>,
    out_dir: &Path,
) -> Result<PathBuf, String> {
    crate::templates::copy_dir(template_dir, out_dir)?;

    let resolved = resolve_values(manifest, values)?;
    let env = make_env(Some(template_dir.to_path_buf()), None);
    let context = base_ctx(manifest, &resolved);

    let mut body = String::new();
    let mut geometry = String::new();
    for (i, page) in manifest.pages.iter().enumerate() {
        let n = i as i64 + 1;
        let tmpl = env
            .get_template(&page.file)
            .map_err(|e| format!("加载页面 {} 失败: {e}", page.file))?;
        let raw = tmpl
            .render(&context)
            .map_err(|e| format!("渲染页面 {} 失败: {e}", page.file))?;
        // 给该页的 .paper 打上命名页标记，供 [data-page] 选择器命中
        let frag = raw.replacen(
            "class=\"paper\"",
            &format!("class=\"paper\" data-page=\"pg{n}\""),
            1,
        );
        body.push_str(&frag);
        body.push('\n');
        geometry.push_str(&page_geometry(page, Some(n)));
        geometry.push('\n');
    }

    // 页面片段里的相对路径是相对 `pages/` 写的（图片为 ../media/…），
    // 打印合并文档在根目录，用 <base> 统一基准。
    let doc = format!(
        "<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\">\n<base href=\"pages/\">\n<link rel=\"stylesheet\" href=\"../assets/base.css\">\n<style>\n{geometry}\n</style></head><body>\n{body}</body></html>\n"
    );
    let path = out_dir.join("print.html");
    fs::write(&path, doc).map_err(|e| e.to_string())?;
    Ok(path)
}
