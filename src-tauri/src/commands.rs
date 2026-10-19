use crate::pdf;
use report_core::model::*;
use report_core::render;
use report_core::templates;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, State};
use zip::write::SimpleFileOptions;

/// 允许上传的资源扩展名（后端安全白名单；模板 accept 仅约束前端选择）。
const ALLOWED_EXT: &[&str] = &["png", "jpg", "jpeg", "bmp", "webp", "gif"];

/// 一次挂载会话：暴露给 `report` 协议的模板目录 + 工程资源目录 + 渲染上下文。
pub struct Mount {
    pub template_dir: PathBuf,
    pub resources_dir: PathBuf,
    pub manifest: TemplateManifest,
    pub render_values: Map<String, Value>,
}

pub struct AppPaths {
    pub templates_dir: PathBuf,
    pub renders_dir: PathBuf,
    /// 工程内嵌模板快照（数据目录，避免依赖安装目录或缓存目录）。
    pub embedded_templates_dir: PathBuf,
    pub mounts: Mutex<HashMap<String, Mount>>,
    /// 供系统打印窗口加载的临时合并 PDF。
    pub generated_pdfs: Mutex<HashMap<String, PathBuf>>,
    /// 模板同步/安装串行锁，避免并发覆盖同一目录。
    pub template_sync: std::sync::atomic::AtomicBool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub os: String,
    pub arch: String,
    pub templates_dir: String,
    pub renders_dir: String,
    pub embedded_templates_dir: String,
}

#[tauri::command]
pub fn get_app_info(state: State<AppPaths>) -> AppInfo {
    AppInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        templates_dir: state.templates_dir.to_string_lossy().to_string(),
        renders_dir: state.renders_dir.to_string_lossy().to_string(),
        embedded_templates_dir: state
            .embedded_templates_dir
            .to_string_lossy()
            .to_string(),
    }
}

fn now() -> String {
    chrono::Local::now().to_rfc3339()
}

fn default_values(m: &TemplateManifest) -> Map<String, Value> {
    let mut map = Map::new();
    for v in &m.variables {
        if v.ty == "table" || v.ty == "images" {
            map.insert(v.key.clone(), Value::Array(vec![]));
        } else if let Some(d) = &v.default {
            map.insert(v.key.clone(), d.clone());
        } else if v.ty == "bool" {
            map.insert(v.key.clone(), Value::Bool(false));
        }
    }
    map
}

fn work_dir(renders_dir: &Path, id: &str) -> PathBuf {
    renders_dir.join("work").join(templates::sanitize_id(id))
}

fn mount_root_url(id: &str) -> String {
    format!("http://report.localhost/m/{id}/")
}
fn pages_base_url(id: &str, manifest: &TemplateManifest) -> String {
    format!("http://report.localhost/m/{id}/{}/", manifest.pages_path())
}
/// 确保存在挂载会话，并写入最新渲染上下文（已解析 + 资源链接改写）。返回 (id, manifest)。
fn ensure_mount(
    state: &AppPaths,
    template_dir: &str,
    values: &Map<String, Value>,
    project_id: &str,
) -> Result<(String, TemplateManifest), String> {
    let tdir = PathBuf::from(template_dir);
    let manifest = templates::read_manifest(&tdir)?;
    let id = if project_id.is_empty() {
        format!("eph-{}", uuid::Uuid::new_v4())
    } else {
        project_id.to_string()
    };
    let resources_dir = work_dir(&state.renders_dir, &id).join("resources");
    let resolved = render::resolve_values(&manifest, values)?;
    let rewritten = render::rewrite_resource_urls(&manifest, &resolved, &mount_root_url(&id));
    let mut mounts = state.mounts.lock().unwrap();
    mounts.insert(
        id.clone(),
        Mount {
            template_dir: tdir,
            resources_dir,
            manifest: manifest.clone(),
            render_values: rewritten,
        },
    );
    Ok((id, manifest))
}

#[tauri::command]
pub fn list_templates(state: State<AppPaths>) -> Result<Vec<TemplateInfo>, String> {
    templates::list_templates(&state.templates_dir)
}

#[tauri::command]
pub fn get_manifest(state: State<AppPaths>, template_id: String) -> Result<TemplateManifest, String> {
    let dir = templates::template_dir(&state.templates_dir, &template_id);
    templates::read_manifest(&dir)
}

#[tauri::command]
pub fn install_template(state: State<AppPaths>, zip_path: String) -> Result<TemplateInfo, String> {
    templates::install_template(&state.templates_dir, &PathBuf::from(zip_path))
}

#[tauri::command]
pub fn export_template(
    state: State<AppPaths>,
    template_id: String,
    out_path: String,
) -> Result<(), String> {
    templates::export_template(&state.templates_dir, &template_id, &PathBuf::from(out_path))
}

#[tauri::command]
pub fn new_project(state: State<AppPaths>, template_id: String) -> Result<OpenedProject, String> {
    let dir = templates::template_dir(&state.templates_dir, &template_id);
    let manifest = templates::read_manifest(&dir)?;
    let project_id = uuid::Uuid::new_v4().to_string();

    // 建立工作目录与资源目录
    let res_dir = work_dir(&state.renders_dir, &project_id).join("resources");
    fs::create_dir_all(&res_dir).map_err(|e| e.to_string())?;

    state.mounts.lock().unwrap().insert(
        project_id.clone(),
        Mount {
            template_dir: dir.clone(),
            resources_dir: res_dir,
            manifest: manifest.clone(),
            render_values: default_values(&manifest),
        },
    );

    let ts = now();
    let project = Project {
        schema_version: 1,
        meta: ProjectMeta {
            name: manifest.name.clone(),
            created_at: ts.clone(),
            updated_at: ts,
        },
        template_ref: TemplateRef {
            id: manifest.id.clone(),
            version: manifest.version.clone(),
            content_hash: manifest.content_hash.clone(),
        },
        values: default_values(&manifest),
        embedded_template: None,
    };
    Ok(OpenedProject {
        project,
        manifest,
        template_dir: dir.to_string_lossy().to_string(),
        project_id,
    })
}

/// 渲染多页模板：每页补全为完整 HTML 字符串（前端 srcdoc 注入），并挂载资源。
#[tauri::command]
pub fn render_preview(
    state: State<AppPaths>,
    template_dir: String,
    values: Map<String, Value>,
    project_id: Option<String>,
) -> Result<Vec<PreviewPage>, String> {
    let (id, manifest) = ensure_mount(
        &state,
        &template_dir,
        &values,
        project_id.as_deref().unwrap_or(""),
    )?;

    let mut pages: Vec<PageDef> = manifest.pages.clone();
    if pages.is_empty() {
        return Err("模板未定义 pages".to_string());
    }
    pages.sort_by_key(|p| p.order.unwrap_or(0));

    // 供渲染的取值：解析派生 + 资源链接改写为绝对挂载 URL
    let resolved = render::resolve_values(&manifest, &values)?;
    let rewritten = render::rewrite_resource_urls(&manifest, &resolved, &mount_root_url(&id));

    let tdir = PathBuf::from(&template_dir);
    let base = pages_base_url(&id, &manifest);
    let rendered = render::render_preview_html(&tdir, &manifest, &rewritten, &pages, &base)?;

    let mut out = Vec::with_capacity(pages.len());
    for ((file, html, thumb_html), p) in rendered.into_iter().zip(pages.into_iter()) {
        let is_flow = p.is_flow();
        out.push(PreviewPage {
            file,
            html,
            thumb_html,
            size: p.size.unwrap_or_else(|| "A4".into()),
            orientation: p.orientation.unwrap_or_else(|| "portrait".into()),
            margin: p.margin.unwrap_or_else(|| "0".into()),
            mode: if is_flow { "flow".into() } else { "fixed".into() },
        });
    }
    Ok(out)
}

/// 解析派生（computed）变量：输入当前值，返回含计算结果的完整取值表（供编辑界面实时显示）。
#[tauri::command]
pub fn resolve_values(
    template_dir: String,
    values: Map<String, Value>,
) -> Result<Map<String, Value>, String> {
    let tdir = PathBuf::from(&template_dir);
    let manifest = templates::read_manifest(&tdir)?;
    render::resolve_values(&manifest, &values)
}

/// 上传一个资源文件：内容寻址命名（sha256 前 8 位 + 扩展名），写入工程资源目录。
#[tauri::command]
pub fn add_resource(
    state: State<AppPaths>,
    project_id: String,
    filename: String,
    data_url: String,
) -> Result<String, String> {
    let ext = Path::new(&filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !ALLOWED_EXT.contains(&ext.as_str()) {
        return Err(format!("不支持的文件类型: {ext}"));
    }
    let b64 = data_url
        .split_once(',')
        .map(|(_, b)| b)
        .ok_or_else(|| "资源数据格式错误".to_string())?;
    if !data_url.starts_with("data:") || !data_url[..data_url.find(',').unwrap_or(0)].contains("base64") {
        return Err("资源必须是 base64 data URL".to_string());
    }
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|e| format!("base64 解码失败: {e}"))?;

    let name = templates::resource_name(&bytes, &ext);
    let rdir = work_dir(&state.renders_dir, &project_id).join("resources");
    fs::create_dir_all(&rdir).map_err(|e| e.to_string())?;
    let path = rdir.join(&name);
    if !path.exists() {
        fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    }
    // 值带路由前缀：工程资源统一走 resources/ 路由
    Ok(format!("resources/{name}"))
}

#[tauri::command]
pub fn remove_resource(
    state: State<AppPaths>,
    project_id: String,
    name: String,
) -> Result<(), String> {
    // 只接受工程资源（resources/ 前缀）；模板资源不可删
    let rel = name.strip_prefix("resources/").unwrap_or(&name);
    let safe = Path::new(rel)
        .file_name()
        .and_then(|e| e.to_str())
        .ok_or_else(|| "资源名非法".to_string())?;
    let path = work_dir(&state.renders_dir, &project_id)
        .join("resources")
        .join(safe);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 按 manifest 逐页打印，并按页面顺序合并为一个 PDF。
pub(crate) fn build_pdf(
    app: &AppHandle,
    mount_id: &str,
    manifest: &TemplateManifest,
    temp_root: &Path,
    out: &Path,
) -> Result<(), String> {
    if manifest.pages.is_empty() {
        return Err("模板未定义 pages".to_string());
    }

    let result = (|| -> Result<(), String> {
        fs::create_dir_all(temp_root).map_err(|e| e.to_string())?;
        let mut page_files: Vec<PathBuf> = Vec::with_capacity(manifest.pages.len());

        for (index, page) in manifest.pages.iter().enumerate() {
            let url = format!(
                "http://report.localhost/m/{mount_id}/__print_page/{index}.html"
            );
            let (width_mm, height_mm) = render::page_size_mm(page);
            let landscape = page
                .orientation
                .as_deref()
                .map(|v| v.eq_ignore_ascii_case("landscape"))
                .unwrap_or(false);
            let print_page = pdf::PrintPage {
                width_in: width_mm / 25.4,
                height_in: height_mm / 25.4,
                landscape,
            };
            let page_file = temp_root.join(format!("page-{index}.pdf"));
            pdf::print_via_webview(app, &url, &page_file, Some(print_page))?;
            page_files.push(page_file);
        }

        pdf::merge_pdfs(&page_files, out)
    })();

    let _ = fs::remove_dir_all(temp_root);
    result
}

/// 进程内 WebView2 PrintToPdf：逐页设置纸张尺寸后合并。
#[tauri::command]
pub async fn generate_pdf(
    app: AppHandle,
    state: State<'_, AppPaths>,
    template_dir: String,
    values: Map<String, Value>,
    project_id: Option<String>,
    out_path: String,
) -> Result<String, String> {
    let (id, manifest) = ensure_mount(
        &state,
        &template_dir,
        &values,
        project_id.as_deref().unwrap_or(""),
    )?;
    let out = PathBuf::from(out_path);

    tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
        let temp = std::env::temp_dir().join(format!("rs-pdf-{}", uuid::Uuid::new_v4()));
        build_pdf(&app, &id, &manifest, &temp, &out)?;
        Ok(out.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 打印：生成与“输出为 PDF”相同尺寸的合并 PDF，再交给系统打印对话框。
#[tauri::command]
pub async fn print_report(
    app: AppHandle,
    state: State<'_, AppPaths>,
    template_dir: String,
    values: Map<String, Value>,
    project_id: Option<String>,
) -> Result<(), String> {
    let (id, manifest) = ensure_mount(
        &state,
        &template_dir,
        &values,
        project_id.as_deref().unwrap_or(""),
    )?;

    let token = uuid::Uuid::new_v4().to_string();
    let print_dir = state.renders_dir.join("print");
    fs::create_dir_all(&print_dir).map_err(|e| e.to_string())?;
    let pdf_path = print_dir.join(format!("{token}.pdf"));

    let app_for_build = app.clone();
    let id_for_build = id.clone();
    let manifest_for_build = manifest.clone();
    let pdf_for_build = pdf_path.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let temp = std::env::temp_dir().join(format!("rs-pdf-{}", uuid::Uuid::new_v4()));
        build_pdf(
            &app_for_build,
            &id_for_build,
            &manifest_for_build,
            &temp,
            &pdf_for_build,
        )
    })
    .await
    .map_err(|e| e.to_string())??;

    state
        .generated_pdfs
        .lock()
        .unwrap()
        .insert(token.clone(), pdf_path);
    let url = format!("http://report.localhost/__pdf/{token}");

    tauri::async_runtime::spawn_blocking(move || pdf::print_via_webview_dialog(&app, &url))
        .await
        .map_err(|e| e.to_string())?
}

fn open_project_file(path: &str, overwrite: bool) -> Result<fs::File, String> {
    let file = if overwrite {
        fs::File::create(path)
    } else {
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
    };
    file.map_err(|e| {
        if e.kind() == std::io::ErrorKind::AlreadyExists {
            format!("目标工程文件已存在：{path}")
        } else {
            e.to_string()
        }
    })
}

#[tauri::command]
pub fn project_file_exists(path: String) -> bool {
    Path::new(&path).is_file()
}

#[tauri::command]
pub fn save_project(
    state: State<AppPaths>,
    path: String,
    mut project: Project,
    embed_template: bool,
    project_id: Option<String>,
    overwrite: Option<bool>,
) -> Result<(), String> {
    project.meta.updated_at = now();
    let file = open_project_file(&path, overwrite.unwrap_or(true))?;
    let mut zip = zip::ZipWriter::new(file);
    let opts: SimpleFileOptions =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let json = serde_json::to_string_pretty(&project).map_err(|e| e.to_string())?;
    zip.start_file("project.json", opts).map_err(|e| e.to_string())?;
    zip.write_all(json.as_bytes()).map_err(|e| e.to_string())?;

    // 工程资源（图片等）随包自包含
    if let Some(pid) = project_id.as_deref().filter(|s| !s.is_empty()) {
        let rdir = work_dir(&state.renders_dir, pid).join("resources");
        if rdir.exists() {
            for item in walkdir::WalkDir::new(&rdir).into_iter().filter_map(|e| e.ok()) {
                if !item.file_type().is_file() {
                    continue;
                }
                let rel = match item.path().strip_prefix(&rdir) {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                let name = format!("resources/{}", rel.to_string_lossy().replace('\\', "/"));
                let bytes = fs::read(item.path()).map_err(|e| e.to_string())?;
                zip.start_file(name, opts).map_err(|e| e.to_string())?;
                zip.write_all(&bytes).map_err(|e| e.to_string())?;
            }
        }
    }

    if embed_template {
        let tdir = templates::template_dir(&state.templates_dir, &project.template_ref.id);
        for item in walkdir::WalkDir::new(&tdir).into_iter().filter_map(|e| e.ok()) {
            let p = item.path();
            let rel = match p.strip_prefix(&tdir) {
                Ok(r) => r,
                Err(_) => continue,
            };
            let name = format!("template/{}", rel.to_string_lossy().replace('\\', "/"));
            if item.file_type().is_dir() {
                zip.add_directory(format!("{name}/"), opts).map_err(|e| e.to_string())?;
            } else {
                let bytes = fs::read(p).map_err(|e| e.to_string())?;
                zip.start_file(name, opts).map_err(|e| e.to_string())?;
                zip.write_all(&bytes).map_err(|e| e.to_string())?;
            }
        }
    }

    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn open_project(state: State<AppPaths>, path: String) -> Result<OpenedProject, String> {
    let file = fs::File::open(&path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    let mut project: Project = {
        let mut s = String::new();
        archive
            .by_name("project.json")
            .map_err(|_| "工程文件缺少 project.json".to_string())?
            .read_to_string(&mut s)
            .map_err(|e| e.to_string())?;
        serde_json::from_str(&s).map_err(|e| format!("解析工程失败: {e}"))?
    };

    let has_embedded = archive.by_name("template/manifest.json").is_ok();

    let tdir = if has_embedded {
        let base = state
            .embedded_templates_dir
            .join(templates::sanitize_id(&project.template_ref.id));
        fs::create_dir_all(&base).map_err(|e| e.to_string())?;

        // 先解压到临时子目录，算快照指纹后再按 <hash> 归档（内容寻址，避免串版本/旧缓存复用）。
        let tmp = base.join(format!(".tmp-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
        for i in 0..archive.len() {
            let mut f = archive.by_index(i).map_err(|e| e.to_string())?;
            let name = f.name().to_string();
            if let Some(rest) = name.strip_prefix("template/") {
                if rest.is_empty() {
                    continue;
                }
                let outp = tmp.join(rest);
                if f.is_dir() {
                    fs::create_dir_all(&outp).map_err(|e| e.to_string())?;
                } else {
                    if let Some(parent) = outp.parent() {
                        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                    }
                    let mut buf = Vec::new();
                    f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
                    fs::write(&outp, buf).map_err(|e| e.to_string())?;
                }
            }
        }

        let hash = templates::compute_snapshot_hash(&tmp)?;
        let dest = base.join(&hash);
        if dest.join("manifest.json").exists() {
            let _ = fs::remove_dir_all(&tmp);
        } else if fs::rename(&tmp, &dest).is_err() {
            // 跨卷等情形回退为复制
            templates::copy_dir(&tmp, &dest)?;
            let _ = fs::remove_dir_all(&tmp);
        }
        dest
    } else {
        templates::template_dir(&state.templates_dir, &project.template_ref.id)
    };

    let manifest = templates::read_manifest(&tdir)?;
    project.template_ref.version = manifest.version.clone();
    project.template_ref.content_hash = manifest.content_hash.clone();

    // 建立工作目录并解出工程资源
    let project_id = uuid::Uuid::new_v4().to_string();
    let res_dir = work_dir(&state.renders_dir, &project_id).join("resources");
    fs::create_dir_all(&res_dir).map_err(|e| e.to_string())?;
    for i in 0..archive.len() {
        let mut f = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = f.name().to_string();
        if let Some(rest) = name.strip_prefix("resources/") {
            if rest.is_empty() || f.is_dir() {
                continue;
            }
            let outp = res_dir.join(rest);
            if let Some(parent) = outp.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut buf = Vec::new();
            f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            fs::write(&outp, buf).map_err(|e| e.to_string())?;
        }
    }

    let resolved = render::resolve_values(&manifest, &project.values)?;
    let rewritten = render::rewrite_resource_urls(&manifest, &resolved, &mount_root_url(&project_id));
    state.mounts.lock().unwrap().insert(
        project_id.clone(),
        Mount {
            template_dir: tdir.clone(),
            resources_dir: res_dir,
            manifest: manifest.clone(),
            render_values: rewritten,
        },
    );

    Ok(OpenedProject {
        project,
        manifest,
        template_dir: tdir.to_string_lossy().to_string(),
        project_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_project_file_requires_overwrite_for_existing_file() {
        let dir = std::env::temp_dir().join(format!("rs-save-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("project.rproj");
        fs::write(&path, b"old").unwrap();

        let err = open_project_file(path.to_str().unwrap(), false).unwrap_err();
        assert!(err.contains("已存在"), "unexpected error: {err}");

        let file = open_project_file(path.to_str().unwrap(), true).unwrap();
        drop(file);

        let _ = fs::remove_dir_all(&dir);
    }
}
