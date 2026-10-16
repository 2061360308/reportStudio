use crate::model::*;
use crate::pdf;
use crate::render;
use crate::templates;
use serde_json::{Map, Value};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use tauri::{AppHandle, State};
use zip::write::SimpleFileOptions;

pub struct AppPaths {
    pub templates_dir: PathBuf,
    pub renders_dir: PathBuf,
}

fn now() -> String {
    chrono::Local::now().to_rfc3339()
}

fn default_values(m: &TemplateManifest) -> Map<String, Value> {
    let mut map = Map::new();
    for v in &m.variables {
        if v.ty == "table" {
            map.insert(v.key.clone(), Value::Array(vec![]));
        } else if let Some(d) = &v.default {
            map.insert(v.key.clone(), d.clone());
        } else if v.ty == "bool" {
            map.insert(v.key.clone(), Value::Bool(false));
        }
    }
    map
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
    })
}

/// 渲染多页模板，逐页返回可加载的 URL 与尺寸信息（供前端 iframe 逐页预览）。
#[tauri::command]
pub fn render_preview(
    state: State<AppPaths>,
    template_dir: String,
    values: Map<String, Value>,
) -> Result<Vec<PreviewPage>, String> {
    let tdir = PathBuf::from(&template_dir);
    let manifest = templates::read_manifest(&tdir)?;

    // 页面清单：优先 manifest.pages（按 order 排序），否则回退 entry 单页。
    let mut pages: Vec<PageDef> = if manifest.pages.is_empty() {
        vec![PageDef {
            file: manifest.entry.clone(),
            order: Some(1),
            size: Some("A4".into()),
            orientation: Some("portrait".into()),
            margin: Some("0".into()),
            ..Default::default()
        }]
    } else {
        manifest.pages.clone()
    };
    pages.sort_by_key(|p| p.order.unwrap_or(0));

    let dir = render::new_render_dir(&state.renders_dir)?;
    render::render_preview_pages(&tdir, &manifest, &values, &dir, &pages)?;

    let mut out = Vec::with_capacity(pages.len());
    for p in pages {
        let path = dir.join(&p.file);
        let rel = path
            .strip_prefix(&state.renders_dir)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        out.push(PreviewPage {
            file: p.file,
            url: format!("http://report.localhost/{rel}"),
            size: p.size.unwrap_or_else(|| "A4".into()),
            orientation: p.orientation.unwrap_or_else(|| "portrait".into()),
            margin: p.margin.unwrap_or_else(|| "0".into()),
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

/// 进程内 WebView2 PrintToPdf。
#[tauri::command]
pub async fn generate_pdf(
    app: AppHandle,
    state: State<'_, AppPaths>,
    template_dir: String,
    values: Map<String, Value>,
    out_path: String,
) -> Result<String, String> {
    let renders = state.renders_dir.clone();
    let out = PathBuf::from(out_path);

    tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
        let tdir = PathBuf::from(&template_dir);
        let manifest = templates::read_manifest(&tdir)?;
        let dir = render::new_render_dir(&renders)?;
        // 有分页清单时由 pages 组装合并文档；否则回退单页 entry。
        let entry = if manifest.pages.is_empty() {
            render::render_to_dir(&tdir, &manifest, &values, &dir)?
        } else {
            render::render_print_document(&tdir, &manifest, &values, &dir)?
        };
        let rel = entry
            .strip_prefix(&renders)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        let url = format!("http://report.localhost/{rel}");
        pdf::print_via_webview(&app, &url, &out)?;
        let _ = fs::remove_dir_all(&dir);
        Ok(out.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 打印：渲染报告后用 WebView2 系统打印对话框打印（可选打印机/参数）。
#[tauri::command]
pub async fn print_report(
    app: AppHandle,
    state: State<'_, AppPaths>,
    template_dir: String,
    values: Map<String, Value>,
) -> Result<(), String> {
    let renders = state.renders_dir.clone();

    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let tdir = PathBuf::from(&template_dir);
        let manifest = templates::read_manifest(&tdir)?;
        let dir = render::new_render_dir(&renders)?;
        let entry = if manifest.pages.is_empty() {
            render::render_to_dir(&tdir, &manifest, &values, &dir)?
        } else {
            render::render_print_document(&tdir, &manifest, &values, &dir)?
        };
        let rel = entry
            .strip_prefix(&renders)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        let url = format!("http://report.localhost/{rel}");
        // 打印窗口会持续引用该渲染目录，故此处不删除
        pdf::print_via_webview_dialog(&app, &url)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn save_project(
    state: State<AppPaths>,
    path: String,
    mut project: Project,
    embed_template: bool,
) -> Result<(), String> {
    project.meta.updated_at = now();
    let file = fs::File::create(&path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let opts: SimpleFileOptions =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let json = serde_json::to_string_pretty(&project).map_err(|e| e.to_string())?;
    zip.start_file("project.json", opts).map_err(|e| e.to_string())?;
    zip.write_all(json.as_bytes()).map_err(|e| e.to_string())?;

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
            .renders_dir
            .join("embedded")
            .join(templates::sanitize_id(&project.template_ref.id));
        fs::create_dir_all(&base).map_err(|e| e.to_string())?;

        // 先解压到临时子目录，算快照指纹后再按 <hash> 归档（内容寻址，避免串版本/旧缓存复用）。
        let tmp = base.join(format!(".tmp-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
        for i in 0..archive.len() {
            let mut f = archive.by_index(i).map_err(|e| e.to_string())?;
            let name = f.name().to_string();
            let Some(rest) = name.strip_prefix("template/") else {
                continue;
            };
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

    Ok(OpenedProject {
        project,
        manifest,
        template_dir: tdir.to_string_lossy().to_string(),
    })
}
