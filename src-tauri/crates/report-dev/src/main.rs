//! 模板开发服务器：渲染模板为可浏览的 HTML，文件变化时浏览器自动热重载。
//! 仅开发期使用，不属于桌面应用，也不会被打包。
//!
//! 用法：
//!   report-dev <模板目录> [--port 1735] [--values values.json] [--resources <资源目录>]

use report_core::model::TemplateManifest;
use report_core::render;
use report_core::templates;
use serde::Serialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const RESOURCE_EXT: &[&str] = &["png", "jpg", "jpeg", "bmp", "webp", "gif"];
const MAX_UPLOAD_BYTES: usize = 64 * 1024 * 1024;

struct DevState {
    template_dir: PathBuf,
    manifest: TemplateManifest,
    base_values: Map<String, Value>,
    base_sample_keys: HashSet<String>,
    /// 工程资源目录；可通过 --resources 或侧边栏配置
    resources_dir: Mutex<PathBuf>,
    /// 服务器根地址（http://127.0.0.1:<port>/），用于把资源值改写为绝对 URL
    base_url: String,
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if argv.first().map(String::as_str) == Some("pack") {
        run_pack(&argv[1..]);
        return;
    }

    let mut dir: Option<String> = None;
    let mut port: u16 = 1735;
    let mut values_file: Option<String> = None;
    let mut resources: Option<String> = None;

    let mut args = argv.into_iter();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--port" | "-p" => {
                if let Some(v) = args.next() {
                    port = v.parse().unwrap_or(1735);
                }
            }
            "--values" | "-v" => values_file = args.next(),
            "--resources" | "-r" => resources = args.next(),
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

    let (base_values, base_sample_keys) = load_values(&manifest, values_file.as_deref());
    let resources_dir = PathBuf::from(resources.unwrap_or_default());

    let listener = match TcpListener::bind(("127.0.0.1", port)) {
        Ok(l) => l,
        Err(_) => TcpListener::bind(("127.0.0.1", 0)).expect("无法绑定本地端口"),
    };
    let addr = listener.local_addr().unwrap();

    println!("模板开发服务器已启动：");
    println!("  地址：http://127.0.0.1:{}", addr.port());
    println!("  模板：{}", template_dir.display());
    println!(
        "  资源：{}",
        if resources_dir.as_os_str().is_empty() {
            "未配置（可在侧边栏资源目录中设置）"
        } else {
            resources_dir.to_str().unwrap_or("")
        }
    );
    println!(
        "  变量：默认取自 manifest.sample{}",
        if values_file.is_some() {
            "（并叠加 --values）"
        } else {
            ""
        }
    );
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
        base_sample_keys,
        resources_dir: Mutex::new(resources_dir),
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PackedTemplate {
    id: String,
    name: String,
    version: String,
    file: String,
    sha256: String,
    size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_app_version: Option<String>,
}

fn run_pack(args: &[String]) {
    let mut templates_dir = PathBuf::from("templates");
    let mut out_dir = PathBuf::from("dist/templates");
    let mut only: Option<Vec<String>> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--templates" | "-t" => {
                i += 1;
                if i >= args.len() {
                    pack_usage();
                    std::process::exit(1);
                }
                templates_dir = PathBuf::from(&args[i]);
            }
            "--out" | "-o" => {
                i += 1;
                if i >= args.len() {
                    pack_usage();
                    std::process::exit(1);
                }
                out_dir = PathBuf::from(&args[i]);
            }
            "--only" => {
                i += 1;
                if i >= args.len() {
                    pack_usage();
                    std::process::exit(1);
                }
                only = Some(
                    args[i]
                        .split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(ToString::to_string)
                        .collect(),
                );
            }
            "-h" | "--help" => {
                pack_usage();
                return;
            }
            other => {
                eprintln!("未知参数：{other}");
                pack_usage();
                std::process::exit(1);
            }
        }
        i += 1;
    }

    if !templates_dir.exists() {
        eprintln!("模板目录不存在：{}", templates_dir.display());
        std::process::exit(1);
    }
    if let Err(e) = std::fs::create_dir_all(&out_dir) {
        eprintln!("创建输出目录失败：{e}");
        std::process::exit(1);
    }

    let mut entries: Vec<(PathBuf, PackedTemplate)> = Vec::new();
    let mut dirs: Vec<PathBuf> = match std::fs::read_dir(&templates_dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir() && p.join("manifest.json").exists())
            .collect(),
        Err(e) => {
            eprintln!("读取模板目录失败：{e}");
            std::process::exit(1);
        }
    };
    dirs.sort();

    for dir in dirs {
        let manifest = match templates::read_manifest(&dir) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("读取 {} 失败：{e}", dir.display());
                std::process::exit(1);
            }
        };

        if let Some(list) = &only {
            if !list.iter().any(|id| id == &manifest.id) {
                continue;
            }
        }

        let id = templates::sanitize_id(&manifest.id);
        let file_name = format!("{}-{}.zip", id, manifest.version);
        let zip_path = out_dir.join(&file_name);
        if let Err(e) = templates::export_template(&templates_dir, &manifest.id, &zip_path) {
            eprintln!("打包模板 {} 失败：{e}", manifest.id);
            std::process::exit(1);
        }

        let bytes = match std::fs::read(&zip_path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("读取 {} 失败：{e}", zip_path.display());
                std::process::exit(1);
            }
        };
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let sha256 = hex::encode(hasher.finalize());

        entries.push((
            zip_path,
            PackedTemplate {
                id: manifest.id.clone(),
                name: manifest.name.clone(),
                version: manifest.version.clone(),
                file: file_name,
                sha256,
                size: bytes.len() as u64,
                min_app_version: manifest.min_app_version.clone(),
            },
        ));
    }

    if let Some(list) = &only {
        let found: Vec<&str> = entries.iter().map(|(_, e)| e.id.as_str()).collect();
        for id in list {
            if !found.iter().any(|x| *x == id.as_str()) {
                eprintln!("未找到模板：{id}");
                std::process::exit(1);
            }
        }
    }

    let metas: Vec<&PackedTemplate> = entries.iter().map(|(_, e)| e).collect();
    let json = serde_json::to_string_pretty(&metas).expect("序列化打包结果失败");
    let index_path = out_dir.join("packed.json");
    if let Err(e) = std::fs::write(&index_path, format!("{json}\n")) {
        eprintln!("写入 {} 失败：{e}", index_path.display());
        std::process::exit(1);
    }

    for (_, entry) in &entries {
        println!("已打包 {} v{} -> {}", entry.id, entry.version, entry.file);
    }
    println!("打包结果：{}", index_path.display());
}

fn pack_usage() {
    eprintln!(
        "打包模板\n\n用法:\n  report-dev pack --templates <模板根目录> --out <输出目录> [--only id1,id2]\n"
    );
}

fn print_help() {
    eprintln!(
        "模板开发服务器\n\n用法:\n  report-dev <模板目录> [--port 1735] [--values values.json] [--resources <资源目录>]\n\n参数:\n  --port, -p       监听端口（默认 1735，占用时自动挑空闲端口）\n  --values, -v     可选：JSON 变量文件（覆盖 sample 值）\n  --resources, -r  可选：图片资源目录（也可在侧边栏中配置）\n"
    );
}

fn load_values(
    manifest: &TemplateManifest,
    file: Option<&str>,
) -> (Map<String, Value>, HashSet<String>) {
    let mut values = Map::new();
    let mut samples = HashSet::new();
    for v in &manifest.variables {
        if v.ty == "computed" {
            continue;
        }
        if let Some(sample) = &v.sample {
            values.insert(v.key.clone(), sample.clone());
            samples.insert(v.key.clone());
        } else if let Some(default) = &v.default {
            values.insert(v.key.clone(), default.clone());
        }
    }
    if let Some(f) = file {
        match std::fs::read_to_string(f) {
            Ok(txt) => match serde_json::from_str::<Value>(&txt) {
                Ok(Value::Object(o)) => {
                    for (k, v) in o {
                        samples.remove(&k);
                        values.insert(k, v);
                    }
                }
                _ => eprintln!("--values 文件不是 JSON 对象，已忽略：{f}"),
            },
            Err(e) => eprintln!("读取 --values 文件失败：{e}"),
        }
    }
    (values, samples)
}

fn handle(mut stream: TcpStream, st: &DevState) -> std::io::Result<()> {
    let (head, body) = read_http_request(&mut stream)?;
    let first = head.lines().next().unwrap_or("");
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let target = parts.next().unwrap_or("/");
    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p, q),
        None => (target, ""),
    };

    if path == "/__upload" {
        if method != "POST" {
            return respond_json_error(&mut stream, "405 Method Not Allowed", "仅支持 POST");
        }

        let pairs = parse_query(query);
        if let Some((_, dir)) = pairs.iter().find(|(k, _)| k == "dir") {
            *st.resources_dir.lock().unwrap() = PathBuf::from(dir);
        }

        let filename = pairs
            .iter()
            .find(|(k, _)| k == "filename")
            .map(|(_, v)| v.clone())
            .unwrap_or_default();
        let ext = Path::new(&filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !RESOURCE_EXT.contains(&ext.as_str()) {
            return respond_json_error(
                &mut stream,
                "400 Bad Request",
                "仅支持 png/jpg/jpeg/bmp/webp/gif 图片",
            );
        }
        if body.is_empty() {
            return respond_json_error(&mut stream, "400 Bad Request", "上传内容为空");
        }

        let dir = st.resources_dir.lock().unwrap().clone();
        if dir.as_os_str().is_empty() {
            return respond_json_error(&mut stream, "400 Bad Request", "请先在侧边栏配置资源目录");
        }
        if let Err(e) = std::fs::create_dir_all(&dir) {
            return respond_json_error(
                &mut stream,
                "500 Internal Server Error",
                &format!("创建资源目录失败：{e}"),
            );
        }

        let mut hasher = Sha256::new();
        hasher.update(&body);
        let hash = hex::encode(hasher.finalize());
        let name = format!("{}.{}", &hash[..8], ext);
        let path_file = dir.join(&name);
        if !path_file.exists() {
            if let Err(e) = std::fs::write(&path_file, &body) {
                return respond_json_error(
                    &mut stream,
                    "500 Internal Server Error",
                    &format!("写入资源失败：{e}"),
                );
            }
        }
        let payload = serde_json::json!({ "name": format!("resources/{name}") }).to_string();
        return respond(
            &mut stream,
            "200 OK",
            "application/json; charset=utf-8",
            payload.as_bytes(),
        );
    }

    if path == "/__ver" {
        let v = fingerprint(&st.template_dir);
        return respond(
            &mut stream,
            "200 OK",
            "text/plain; charset=utf-8",
            v.as_bytes(),
        );
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

    // 其余路径：模板静态资源，或 resources/ 下的工程资源
    let rel = percent_decode(path.trim_start_matches('/'));
    if !rel.is_empty() && !rel.contains("..") {
        let (base, rel_path) = if let Some(res_rel) = rel.strip_prefix("resources/") {
            let dir = st.resources_dir.lock().unwrap().clone();
            if dir.as_os_str().is_empty() {
                return respond(
                    &mut stream,
                    "404 Not Found",
                    "text/plain; charset=utf-8",
                    b"resources dir is not configured",
                );
            }
            (dir, res_rel.to_string())
        } else {
            (st.template_dir.clone(), rel.clone())
        };

        let full = base.join(&rel_path);
        let root = base.canonicalize().unwrap_or_else(|_| base.clone());
        if let Ok(cp) = full.canonicalize() {
            if cp.starts_with(&root) {
                if let Ok(bytes) = std::fs::read(&cp) {
                    return respond(&mut stream, "200 OK", mime_of(&cp), &bytes);
                }
            }
        }
    }

    respond(
        &mut stream,
        "404 Not Found",
        "text/plain; charset=utf-8",
        b"404",
    )
}

fn read_http_request(stream: &mut TcpStream) -> std::io::Result<(String, Vec<u8>)> {
    stream.set_read_timeout(Some(Duration::from_secs(30))).ok();
    let mut buf: Vec<u8> = Vec::new();
    let mut tmp = [0u8; 8192];
    let mut header_end: Option<usize> = None;

    while header_end.is_none() && buf.len() <= 64 * 1024 {
        let n = stream.read(&mut tmp)?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = find_subslice(&buf, b"\r\n\r\n") {
            header_end = Some(pos + 4);
        }
    }

    let end = header_end.unwrap_or(buf.len());
    let head = String::from_utf8_lossy(&buf[..end]).into_owned();
    let content_length = head
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.eq_ignore_ascii_case("content-length") {
                value.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0);

    if content_length > MAX_UPLOAD_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "请求体过大",
        ));
    }

    let mut body = buf.get(end..).map(|s| s.to_vec()).unwrap_or_default();
    while body.len() < content_length {
        let n = stream.read(&mut tmp)?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&tmp[..n]);
    }
    body.truncate(content_length);
    Ok((head, body))
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn respond_json_error(stream: &mut TcpStream, status: &str, message: &str) -> std::io::Result<()> {
    let body = serde_json::json!({ "error": message }).to_string();
    respond(
        stream,
        status,
        "application/json; charset=utf-8",
        body.as_bytes(),
    )
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
    let mut sample_keys = st.base_sample_keys.clone();
    let mut resources_dir = st.resources_dir.lock().unwrap().clone();
    for (k, v) in &pairs {
        if k == "annot" {
            continue;
        }
        if k == "resourcesDir" {
            resources_dir = PathBuf::from(v);
            *st.resources_dir.lock().unwrap() = resources_dir.clone();
            continue;
        }
        if let Some(def) = st.manifest.variables.iter().find(|x| &x.key == k) {
            values.insert(k.clone(), coerce(&def.ty, v));
            sample_keys.remove(k);
        }
    }

    let mut doc = render::render_print_html_with_samples(
        &st.template_dir,
        &st.manifest,
        &render::rewrite_resource_urls(&st.manifest, &values, &st.base_url),
        Some(Arc::new(sample_keys)),
    )?;
    if let Some(pos) = doc.find("</head>") {
        doc.insert_str(pos, &format!("<style>\n{}</style>", render::PREVIEW_CSS));
    }
    let form = build_form(
        &st.manifest,
        &values,
        annot,
        resources_dir.to_str().unwrap_or(""),
    );
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
  window.rsUpload = function (input) {
    var key = input.getAttribute('data-rs-upload');
    var multi = input.getAttribute('data-multi') === '1';
    var valueInput = document.querySelector('[data-rs-input="' + key + '"]');
    var status = document.querySelector('[data-rs-status="' + key + '"]');
    var dirInput = document.querySelector('#rs-devpanel input[name="resourcesDir"]');
    var files = Array.prototype.slice.call(input.files || []);
    input.value = '';
    if (!files.length || !valueInput) return;
    if (status) status.textContent = '上传中…';
    var names = [];
    var chain = Promise.resolve();
    files.forEach(function (file) {
      chain = chain.then(function () {
        var url = '/__upload?filename=' + encodeURIComponent(file.name);
        if (dirInput) url += '&dir=' + encodeURIComponent(dirInput.value);
        return fetch(url, { method: 'POST', body: file }).then(function (res) {
          return res.json().then(function (data) {
            if (!res.ok) throw new Error(data.error || ('HTTP ' + res.status));
            return data.name;
          });
        }).then(function (name) { names.push(name); });
      });
    });
    chain.then(function () {
      if (multi) {
        var arr = [];
        try { arr = JSON.parse(valueInput.value || '[]'); } catch (e) { arr = []; }
        if (!Array.isArray(arr)) arr = [];
        names.forEach(function (name) {
          if (arr.indexOf(name) < 0) arr.push(name);
        });
        valueInput.value = JSON.stringify(arr);
      } else if (names.length) {
        valueInput.value = names[0];
      }
      if (status) status.textContent = '已上传 ' + names.length + ' 个文件';
      var form = document.getElementById('rs-devpanel');
      if (form) form.submit();
    }).catch(function (e) {
      if (status) status.textContent = '上传失败：' + e.message;
    });
  };
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
#rs-devpanel input[type=file]{max-width:100%;color:#8b9099;font:11px/1.5 'Microsoft YaHei',system-ui,sans-serif}
#rs-devpanel input[type=file]::file-selector-button{background:#2563eb;border:0;color:#fff;padding:4px 8px;
  border-radius:4px;cursor:pointer;font:inherit;margin-right:6px}
#rs-devpanel input[type=file]::file-selector-button:hover{background:#1d4ed8}
#rs-devpanel .upl{display:flex;align-items:center;gap:6px;flex-wrap:wrap;margin-top:2px}
#rs-devpanel .upl-status{color:#6b7078;font-size:11px}
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
    resources_dir: &str,
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
    s.push_str(&format!(
        "<label class=\"fld\"><span>资源目录</span><input type=\"text\" name=\"resourcesDir\" value=\"{}\" placeholder=\"例如 D:\\\\项目\\\\resources\"/><div class=\"hint\" style=\"text-align:left;margin-top:2px\">图片上传会写入此目录；修改后点“应用并刷新”。</div></label>",
        esc_attr(resources_dir)
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

        if v.ty == "image" || v.ty == "images" {
            let multi = if v.ty == "images" { "1" } else { "0" };
            let multiple = if v.ty == "images" { "multiple" } else { "" };
            s.push_str(&format!(
                "<label class=\"fld\"><span title=\"{}\">{}</span><input type=\"text\" data-rs-input=\"{}\" name=\"{}\" value=\"{}\" placeholder=\"可填写资源路径，或使用下方上传\"/><div class=\"upl\"><input type=\"file\" accept=\"image/*\" {} data-rs-upload=\"{}\" data-multi=\"{}\" onchange=\"rsUpload(this)\"/><span class=\"upl-status\" data-rs-status=\"{}\"></span></div></label>",
                esc_attr(&label),
                esc(&label),
                esc_attr(&v.key),
                esc_attr(&v.key),
                esc_attr(&val),
                multiple,
                esc_attr(&v.key),
                multi,
                esc_attr(&v.key)
            ));
        } else {
            s.push_str(&format!(
                "<label class=\"fld\"><span title=\"{}\">{}</span><input type=\"text\" name=\"{}\" value=\"{}\"/></label>",
                esc_attr(&label),
                esc(&label),
                esc_attr(&v.key),
                esc_attr(&val)
            ));
        }
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
        "images" | "table" | "multiselect" => {
            if s.trim().is_empty() {
                Value::Array(vec![])
            } else {
                serde_json::from_str::<Value>(s).unwrap_or_else(|_| Value::from(s))
            }
        }
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
        for b in name
            .as_bytes()
            .iter()
            .chain(len.to_le_bytes().iter())
            .chain(mtime.to_le_bytes().iter())
        {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    format!("{h:016x}")
}
