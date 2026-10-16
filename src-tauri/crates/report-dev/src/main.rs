//! 模板开发服务器：渲染模板为可浏览的 HTML，文件变化时浏览器自动热重载。
//! 仅开发期使用，不属于桌面应用，也不会被打包。
//!
//! 用法：
//!   report-dev <模板目录> [--port 1735] [--values values.json]

use report_core::model::TemplateManifest;
use report_core::render;
use report_core::templates;
use serde_json::{Map, Value};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

struct DevState {
    template_dir: PathBuf,
    manifest: TemplateManifest,
    base_values: Map<String, Value>,
    /// 服务器根地址（http://127.0.0.1:<port>/），用于把资源值改写为绝对 URL
    base_url: String,
}

fn main() {
    let mut dir: Option<String> = None;
    let mut port: u16 = 1735;
    let mut values_file: Option<String> = None;

    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--port" | "-p" => {
                if let Some(v) = args.next() {
                    port = v.parse().unwrap_or(1735);
                }
            }
            "--values" | "-v" => values_file = args.next(),
            "-h" | "--help" => {
                print_help();
                return;
            }
            other => {
                if dir.is_none() {
                    dir = Some(other.to_string());
                }
            }
        }
    }

    let Some(dir) = dir else {
        print_help();
        std::process::exit(1);
    };
    let template_dir = PathBuf::from(&dir);

    let manifest = match templates::read_manifest(&template_dir) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("读取模板失败（{}）: {e}", template_dir.display());
            std::process::exit(1);
        }
    };
    if manifest.pages.is_empty() {
        eprintln!("该模板未定义 pages，无法开发预览。");
        std::process::exit(1);
    }

    let base_values = load_values(&manifest, values_file.as_deref());

    let listener = match TcpListener::bind(("127.0.0.1", port)) {
        Ok(l) => l,
        Err(_) => TcpListener::bind(("127.0.0.1", 0)).expect("无法绑定本地端口"),
    };
    let addr = listener.local_addr().unwrap();

    println!("模板开发服务器已启动：");
    println!("  地址：http://127.0.0.1:{}", addr.port());
    println!("  模板：{}", template_dir.display());
    println!("  变量：默认取自 manifest.sample{}", if values_file.is_some() { "（并叠加 --values）" } else { "" });
    println!("改动模板文件后浏览器会自动重载（Ctrl+C 退出）");

    // 优雅退出：Ctrl+C 时以 0 退出，避免 cargo/npm 报 “process didn't exit successfully”。
    let _ = ctrlc::set_handler(|| {
        println!("\n模板开发服务器已退出");
        std::process::exit(0);
    });

    let state = Arc::new(DevState {
        template_dir,
        manifest,
        base_values,
        base_url: format!("http://127.0.0.1:{}/", addr.port()),
    });

    for stream in listener.incoming() {
        if let Ok(s) = stream {
            let st = Arc::clone(&state);
            std::thread::spawn(move || {
                let _ = handle(s, &st);
            });
        }
    }
}

fn print_help() {
    eprintln!(
        "模板开发服务器\n\n用法:\n  report-dev <模板目录> [--port 1735] [--values values.json]\n\n参数:\n  --port, -p    监听端口（默认 1735，占用时自动挑空闲端口）\n  --values, -v  可选：JSON 变量文件（覆盖 sample 值）\n"
    );
}

fn sample_values(manifest: &TemplateManifest) -> Map<String, Value> {
    let mut m = Map::new();
    for v in &manifest.variables {
        if v.ty == "computed" {
            continue;
        }
        if let Some(s) = &v.sample {
            m.insert(v.key.clone(), s.clone());
        } else if let Some(d) = &v.default {
            m.insert(v.key.clone(), d.clone());
        }
    }
    m
}

fn load_values(manifest: &TemplateManifest, file: Option<&str>) -> Map<String, Value> {
    let mut m = sample_values(manifest);
    if let Some(f) = file {
        match std::fs::read_to_string(f) {
            Ok(txt) => match serde_json::from_str::<Value>(&txt) {
                Ok(Value::Object(o)) => {
                    for (k, v) in o {
                        m.insert(k, v);
                    }
                }
                _ => eprintln!("--values 文件不是 JSON 对象，已忽略：{f}"),
            },
            Err(e) => eprintln!("读取 --values 文件失败：{e}"),
        }
    }
    m
}

fn handle(mut stream: TcpStream, st: &DevState) -> std::io::Result<()> {
    let req = read_request(&mut stream)?;
    let first = req.lines().next().unwrap_or("");
    let mut parts = first.split_whitespace();
    let _method = parts.next().unwrap_or("GET");
    let target = parts.next().unwrap_or("/");
    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p, q),
        None => (target, ""),
    };

    if path == "/__ver" {
        let v = fingerprint(&st.template_dir);
        return respond(&mut stream, "200 OK", "text/plain; charset=utf-8", v.as_bytes());
    }

    if path == "/" || path == "/index.html" {
        return match build_page(st, query) {
            Ok(html) => respond(
                &mut stream,
                "200 OK",
                "text/html; charset=utf-8",
                html.as_bytes(),
            ),
            Err(e) => respond(
                &mut stream,
                "500 Internal Server Error",
                "text/plain; charset=utf-8",
                format!("渲染失败: {e}").as_bytes(),
            ),
        };
    }

    // 其余任何路径都尝试作为模板目录下的静态资源（assets/pages/media 等）
    let rel = percent_decode(path.trim_start_matches('/'));
    if !rel.is_empty() && !rel.contains("..") {
        let full = st.template_dir.join(&rel);
        let root = st
            .template_dir
            .canonicalize()
            .unwrap_or_else(|_| st.template_dir.clone());
        if let Ok(cp) = full.canonicalize() {
            if cp.starts_with(&root) {
                if let Ok(bytes) = std::fs::read(&cp) {
                    return respond(&mut stream, "200 OK", mime_of(&cp), &bytes);
                }
            }
        }
    }

    respond(&mut stream, "404 Not Found", "text/plain; charset=utf-8", b"404")
}

fn read_request(stream: &mut TcpStream) -> std::io::Result<String> {
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
    let mut buf: Vec<u8> = Vec::new();
    let mut tmp = [0u8; 2048];
    loop {
        let n = stream.read(&mut tmp)?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") || buf.len() > 65536 {
            break;
        }
    }
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn respond(stream: &mut TcpStream, status: &str, ctype: &str, body: &[u8]) -> std::io::Result<()> {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

fn build_page(st: &DevState, query: &str) -> Result<String, String> {
    let pairs = parse_query(query);
    let annot = pairs
        .iter()
        .any(|(k, v)| k == "annot" && (v == "1" || v == "true" || v == "on"));

    let mut values = st.base_values.clone();
    for (k, v) in &pairs {
        if k == "annot" {
            continue;
        }
        if let Some(def) = st.manifest.variables.iter().find(|x| &x.key == k) {
            values.insert(k.clone(), coerce(&def.ty, v));
        }
    }

    let mut doc = render::render_print_html(
        &st.template_dir,
        &st.manifest,
        &render::rewrite_resource_urls(&st.manifest, &values, &st.base_url),
    )?;
    let form = build_form(&st.manifest, &values, annot);
    if let Some(pos) = doc.find("<body>") {
        doc.insert_str(
            pos + "<body>".len(),
            &format!("{form}<div id=\"rs-pages\">"),
        );
    }
    if let Some(pos) = doc.rfind("</body>") {
        doc.insert_str(pos, &format!("</div>{LIVERELOAD_JS}"));
    }
    Ok(doc)
}

const LIVERELOAD_JS: &str = r#"<script>
(function () {
  var root = document.documentElement;
  if (location.search.indexOf('annot') >= 0) root.classList.add('rs-annot');
  var KEY = 'rs-dev-panel';
  try { if (localStorage.getItem(KEY) === '1') root.classList.add('rs-panel-open'); } catch (e) {}
  window.rsTogglePanel = function () {
    root.classList.toggle('rs-panel-open');
    try { localStorage.setItem(KEY, root.classList.contains('rs-panel-open') ? '1' : '0'); } catch (e) {}
  };
  // Ctrl + 滚轮/加号/减号 缩放页面（Ctrl+0 复位）
  var pages = document.getElementById('rs-pages');
  var z = 1;
  try { var sz = parseFloat(localStorage.getItem('rs-dev-zoom') || '1'); if (sz >= 0.25 && sz <= 4) z = sz; } catch (e) {}
  function apply() { if (pages) pages.style.zoom = z; }
  window.rsSetZoom = function (nz) {
    z = Math.max(0.25, Math.min(4, nz));
    apply();
    try { localStorage.setItem('rs-dev-zoom', String(z)); } catch (e) {}
  };
  apply();
  window.addEventListener('wheel', function (e) {
    if (!e.ctrlKey) return;
    e.preventDefault();
    window.rsSetZoom(z * (e.deltaY < 0 ? 1.1 : 1 / 1.1));
  }, { passive: false });
  window.addEventListener('keydown', function (e) {
    if (!e.ctrlKey) return;
    if (e.key === '=' || e.key === '+') { e.preventDefault(); window.rsSetZoom(z * 1.1); }
    else if (e.key === '-' || e.key === '_') { e.preventDefault(); window.rsSetZoom(z / 1.1); }
    else if (e.key === '0') { e.preventDefault(); window.rsSetZoom(1); }
  });
  var v = null;
  setInterval(function () {
    fetch('/__ver', { cache: 'no-store' }).then(function (r) { return r.text(); }).then(function (t) {
      if (v === null) { v = t; } else if (t !== v) { location.reload(); }
    }).catch(function () {});
  }, 800);
})();
</script>"#;

const DEV_STYLE: &str = r#"<style>
@media screen{
  html,body{background:#e9ebee}
  .paper{margin:18px auto;background:#fff;box-shadow:0 1px 2px rgba(0,0,0,.14),0 10px 28px rgba(0,0,0,.12);outline:1px solid #d5d8dd}
  .paper .page{outline:1px dashed rgba(0,0,0,.10);outline-offset:-1px}
}
#rs-devtoggle{position:fixed;left:12px;bottom:12px;z-index:2147483647;display:flex;align-items:center;gap:8px;
  background:#1b1d21;color:#e6e6e6;border:1px solid #33373d;border-radius:999px;padding:7px 14px;cursor:pointer;
  font:12px/1 'Microsoft YaHei',system-ui,sans-serif;box-shadow:0 3px 10px rgba(0,0,0,.35);transition:left .22s ease}
#rs-devtoggle:hover{background:#26292e}
#rs-devtoggle .dot{width:8px;height:8px;border-radius:50%;background:#2563eb;box-shadow:0 0 0 3px rgba(37,99,235,.25)}
html.rs-panel-open #rs-devtoggle{left:312px}
#rs-devpanel{position:fixed;top:0;left:0;bottom:0;width:300px;z-index:2147483647;display:flex;flex-direction:column;
  background:#1b1d21;color:#e6e6e6;font:13px/1.7 'Microsoft YaHei',system-ui,sans-serif;
  box-shadow:2px 0 14px rgba(0,0,0,.35);transform:translateX(-100%);transition:transform .22s ease}
html.rs-panel-open #rs-devpanel{transform:translateX(0)}
#rs-devpanel .hd{padding:10px 14px;font-weight:600;border-bottom:1px solid #2c2f34;display:flex;align-items:baseline;gap:8px}
#rs-devpanel .hd small{color:#8b9099;font-weight:400}
#rs-devpanel .bd{flex:1 1 auto;overflow:auto;padding:10px 14px}
#rs-devpanel .fld{display:flex;flex-direction:column;gap:3px;margin-bottom:9px}
#rs-devpanel .fld>span{color:#8b9099;font-size:12px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
#rs-devpanel input[type=text]{width:100%;box-sizing:border-box;background:#26292e;border:1px solid #3a3e44;color:#e6e6e6;
  padding:4px 8px;border-radius:5px;font:inherit}
#rs-devpanel input[type=text]:focus{outline:none;border-color:#2563eb}
#rs-devpanel .ft{padding:10px 14px;border-top:1px solid #2c2f34;display:flex;flex-direction:column;gap:8px}
#rs-devpanel label.chk{display:flex;align-items:center;gap:8px;color:#c3c7cc;cursor:pointer}
#rs-devpanel button.apply{background:#2563eb;border:0;color:#fff;padding:7px;border-radius:6px;cursor:pointer;font:inherit}
#rs-devpanel button.apply:hover{background:#1d4ed8}
#rs-devpanel .hint{color:#6b7078;font-size:11px;text-align:center}
body{transition:padding-left .22s ease}
html.rs-panel-open body{padding-left:300px}
</style>"#;

fn build_form(
    manifest: &TemplateManifest,
    values: &Map<String, Value>,
    annot: bool,
) -> String {
    let fields: Vec<_> = manifest
        .variables
        .iter()
        .filter(|v| v.ty != "computed")
        .collect();

    let mut s = String::new();
    s.push_str(DEV_STYLE);
    s.push_str(
        "<button id=\"rs-devtoggle\" type=\"button\" onclick=\"rsTogglePanel()\"><span class=\"dot\"></span>模板变量</button>",
    );
    s.push_str(&format!(
        "<form id=\"rs-devpanel\" method=\"get\"><div class=\"hd\">模板开发 <small>共 {} 个变量</small></div><div class=\"bd\">",
        fields.len()
    ));
    for v in fields {
        let label = v.alias.clone().unwrap_or_else(|| v.key.clone());
        let val = values
            .get(&v.key)
            .map(|x| match x {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .unwrap_or_default();
        s.push_str(&format!(
            "<label class=\"fld\"><span title=\"{}\">{}</span><input type=\"text\" name=\"{}\" value=\"{}\"/></label>",
            esc_attr(&label),
            esc(&label),
            esc_attr(&v.key),
            esc_attr(&val)
        ));
    }
    s.push_str("</div><div class=\"ft\">");
    s.push_str(&format!(
        "<label class=\"chk\"><input type=\"checkbox\" name=\"annot\" value=\"1\" {}/> 显示字段标记</label>",
        if annot { "checked" } else { "" }
    ));
    s.push_str("<button class=\"apply\" type=\"submit\">应用并刷新</button>");
    s.push_str("<div class=\"hint\">改模板文件会自动热重载</div></div></form>");
    s
}

fn coerce(ty: &str, s: &str) -> Value {
    match ty {
        "number" | "money" | "percent" => s
            .parse::<f64>()
            .map(Value::from)
            .unwrap_or_else(|_| Value::from(s)),
        "integer" => s
            .parse::<i64>()
            .map(Value::from)
            .unwrap_or_else(|_| Value::from(s)),
        "bool" => Value::from(matches!(s, "1" | "true" | "on" | "yes")),
        _ => Value::from(s),
    }
}

fn parse_query(q: &str) -> Vec<(String, String)> {
    q.split('&')
        .filter(|s| !s.is_empty())
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (percent_decode(k), percent_decode(v)))
        .collect()
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                if let (Some(h), Some(l)) = (hexval(bytes[i + 1]), hexval(bytes[i + 2])) {
                    out.push(h * 16 + l);
                    i += 3;
                    continue;
                }
                out.push(b'%');
                i += 1;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hexval(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn mime_of(p: &Path) -> &'static str {
    match p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
fn esc_attr(s: &str) -> String {
    esc(s).replace('"', "&quot;")
}

/// 模板目录内容指纹（路径 + 大小 + mtime），用于浏览器热重载。
fn fingerprint(dir: &Path) -> String {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, u64, u64)>) {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                let md = match e.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                if md.is_dir() {
                    walk(base, &p, out);
                } else if md.is_file() {
                    let rel = p
                        .strip_prefix(base)
                        .map(|r| r.to_string_lossy().replace('\\', "/"))
                        .unwrap_or_default();
                    let mtime = md
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);
                    out.push((rel, md.len(), mtime));
                }
            }
        }
    }

    let mut files: Vec<(String, u64, u64)> = Vec::new();
    walk(dir, dir, &mut files);
    files.sort();
    let mut h: u64 = 0xcbf29ce484222325;
    for (name, len, mtime) in files {
        for b in name.as_bytes().iter().chain(len.to_le_bytes().iter()).chain(mtime.to_le_bytes().iter()) {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    format!("{h:016x}")
}
