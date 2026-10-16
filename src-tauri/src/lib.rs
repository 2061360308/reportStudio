mod commands;
mod pdf;

use report_core::{render, templates};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::Manager;

/// 程序目录（exe 所在的 `templates` 子目录）；取不到时退回系统临时目录。
fn program_templates_dir(app: &tauri::AppHandle) -> PathBuf {
    if let Ok(dir) = app.path().resource_dir() {
        return dir.join("templates");
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            return parent.join("templates");
        }
    }
    std::env::temp_dir().join("templates")
}

/// 目录是否可写（用于只读安装目录的兜底）。
fn is_writable(dir: &Path) -> bool {
    if fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(format!(".write-test-{}", std::process::id()));
    match fs::write(&probe, b"") {
        Ok(_) => {
            let _ = fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

fn copy_dir_all(src: &Path, dest: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let target = dest.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &target)?;
        } else {
            fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

/// 将旧 `%APPDATA%/.../templates` 下的模板迁移到新目录（不覆盖已存在的）。
fn migrate_templates(old: &Path, new: &Path) {
    let Ok(rd) = fs::read_dir(old) else {
        return;
    };
    for entry in rd.flatten() {
        let src = entry.path();
        if !src.is_dir() {
            continue;
        }
        let dest = new.join(entry.file_name());
        if dest.exists() {
            continue;
        }
        match copy_dir_all(&src, &dest) {
            Ok(_) => eprintln!("已迁移模板 {} → {}", src.display(), dest.display()),
            Err(e) => eprintln!("迁移模板 {} 失败: {e}", src.display()),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .register_uri_scheme_protocol("report", |ctx, req| {
            let not_found = || {
                tauri::http::Response::builder()
                    .status(404)
                    .body(Vec::new())
                    .unwrap()
            };
            let state = ctx.app_handle().state::<commands::AppPaths>();
            let path = req.uri().path().to_string();
            let decoded = percent_encoding::percent_decode_str(path.trim_start_matches('/'))
                .decode_utf8_lossy()
                .to_string();
            // 期望路由：m/<mountId>/<rest>
            let (id, rest) = match decoded.strip_prefix("m/").and_then(|s| s.split_once('/')) {
                Some((id, rest)) => (id.to_string(), rest.to_string()),
                None => return not_found(),
            };

            // 虚拟打印文档
            if rest == "__print.html" {
                let ctx2 = {
                    let mounts = state.mounts.lock().unwrap();
                    mounts.get(&id).map(|m| {
                        (m.template_dir.clone(), m.manifest.clone(), m.render_values.clone())
                    })
                };
                return match ctx2 {
                    Some((tdir, manifest, values)) => {
                        match render::render_print_html(&tdir, &manifest, &values) {
                            Ok(html) => tauri::http::Response::builder()
                                .header("Content-Type", "text/html; charset=utf-8")
                                .body(html.into_bytes())
                                .unwrap(),
                            Err(e) => tauri::http::Response::builder()
                                .status(500)
                                .body(e.into_bytes())
                                .unwrap(),
                        }
                    }
                    None => not_found(),
                };
            }

            let root = {
                let mounts = state.mounts.lock().unwrap();
                match mounts.get(&id) {
                    Some(m) => {
                        if let Some(r) = rest.strip_prefix("resources/") {
                            (m.resources_dir.clone(), r.to_string())
                        } else {
                            (m.template_dir.clone(), rest.clone())
                        }
                    }
                    None => return not_found(),
                }
            };
            let (base, rel) = root;
            let full = base.join(&rel);
            let base_c = base.canonicalize().unwrap_or_else(|_| base.clone());
            match full.canonicalize() {
                Ok(cp) if cp.starts_with(&base_c) => match fs::read(&cp) {
                    Ok(bytes) => {
                        let mime = mime_guess::from_path(&cp)
                            .first_or_octet_stream()
                            .to_string();
                        tauri::http::Response::builder()
                            .header("Content-Type", mime)
                            .body(bytes)
                            .unwrap()
                    }
                    Err(_) => not_found(),
                },
                _ => not_found(),
            }
        })
        .setup(|app| {
            let data = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("report-studio"));
            let cache = app
                .path()
                .app_cache_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("report-studio-cache"));

            // 模板目录：开发时用仓库 `src-tauri/templates`（稳定，不随 cargo clean 丢失），
            // 发布时用程序目录（exe 同级的 `templates`）。
            let mut templates_dir = if cfg!(debug_assertions) {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates")
            } else {
                program_templates_dir(app.handle())
            };

            // 程序目录可能只读（如 Program Files 机器级安装）→ 回退到应用数据目录。
            if !is_writable(&templates_dir) {
                let fallback = data.join("templates");
                eprintln!(
                    "模板目录 {} 不可写，回退到 {}",
                    templates_dir.display(),
                    fallback.display()
                );
                templates_dir = fallback;
            }

            let renders_dir = cache.join("renders");
            fs::create_dir_all(&templates_dir).ok();
            fs::create_dir_all(&renders_dir).ok();

            // 清理旧版内嵌模板缓存（旧的平铺布局 `<id>/manifest.json`）；
            // 新版为内容寻址 `<id>/<hash>/`，不会命中这里。
            let embedded = renders_dir.join("embedded");
            if let Ok(rd) = fs::read_dir(&embedded) {
                for entry in rd.flatten() {
                    let p = entry.path();
                    if p.is_dir() && p.join("manifest.json").exists() {
                        let _ = fs::remove_dir_all(&p);
                        eprintln!("已清理旧版内嵌模板缓存: {}", p.display());
                    }
                }
            }

            // 发布版：把旧的 %APPDATA% 模板迁移到程序目录（已存在的跳过）。
            // 开发版不迁移，避免把用户目录的模板写进仓库源码目录。
            if !cfg!(debug_assertions) {
                let legacy = data.join("templates");
                if legacy != templates_dir {
                    migrate_templates(&legacy, &templates_dir);
                }
            }

            // 清理上一会话遗留的工程工作目录（资源工作区）
            let work = renders_dir.join("work");
            if work.exists() {
                let _ = fs::remove_dir_all(&work);
            }

            app.manage(commands::AppPaths {
                templates_dir,
                renders_dir,
                mounts: Mutex::new(HashMap::new()),
            });

            // 无头自检：RS_SELFTEST=1 时，启动后自动走一遍"渲染 + 进程内 PrintToPdf"。
            if std::env::var("RS_SELFTEST").is_ok() {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(800));
                    let result = self_test(&handle);
                    let msg = match result {
                        Ok(info) => format!("SELFTEST OK {info}"),
                        Err(e) => format!("SELFTEST FAIL: {e}"),
                    };
                    let _ = std::fs::write(std::env::temp_dir().join("rs-selftest.log"), msg);
                    handle.exit(0);
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_templates,
            commands::get_manifest,
            commands::install_template,
            commands::export_template,
            commands::new_project,
            commands::render_preview,
            commands::resolve_values,
            commands::add_resource,
            commands::remove_resource,
            commands::generate_pdf,
            commands::print_report,
            commands::save_project,
            commands::open_project,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn self_test(handle: &tauri::AppHandle) -> Result<String, String> {
    use tauri::Manager;
    let state = handle.state::<commands::AppPaths>();
    let tdir = state.templates_dir.join("settle-audit-report");
    let manifest = templates::read_manifest(&tdir)?;

    let values = serde_json::Map::new();

    // 注册一个挂载会话（__print.html 由协议现场生成）
    let project_id = format!("selftest-{}", uuid::Uuid::new_v4());
    let res_dir = state
        .renders_dir
        .join("work")
        .join(&project_id)
        .join("resources");
    fs::create_dir_all(&res_dir).map_err(|e| e.to_string())?;
    let resolved = render::resolve_values(&manifest, &values)?;
    let rewritten = render::rewrite_resource_urls(
        &manifest,
        &resolved,
        &format!("http://report.localhost/m/{project_id}/resources/"),
    );
    state.mounts.lock().unwrap().insert(
        project_id.clone(),
        commands::Mount {
            template_dir: tdir.clone(),
            resources_dir: res_dir,
            manifest: manifest.clone(),
            render_values: rewritten,
        },
    );
    let url = format!("http://report.localhost/m/{project_id}/__print.html");
    let parsed: url::Url = url.parse().map_err(|e: url::ParseError| e.to_string())?;

    // 方案 A：同窗口子 webview（多 webview）
    let mut preview_ok = false;
    if let Some(window) = handle.get_window("main") {
        let builder = tauri::WebviewBuilder::new("preview", tauri::WebviewUrl::External(parsed));
        window
            .add_child(
                builder,
                tauri::LogicalPosition::new(0.0, 0.0),
                tauri::LogicalSize::new(400.0, 300.0),
            )
            .map_err(|e| format!("add_child 失败: {e}"))?;
        preview_ok = handle.get_webview("preview").is_some();
        if let Some(pv) = handle.get_webview("preview") {
            let _ = pv.close();
        }
    }

    let out = std::env::temp_dir().join("rs-selftest.pdf");
    let _ = std::fs::remove_file(&out);
    pdf::print_via_webview(handle, &url, &out)?;
    let size = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
    if size == 0 {
        return Err("PDF 为空".to_string());
    }
    Ok(format!("size={size} preview={preview_ok}"))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    #[test]
    fn settle_audit_manifest_is_valid() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/settle-audit-report");
        let manifest = report_core::templates::read_manifest(&dir).unwrap();

        assert_eq!(manifest.id, "settle-audit-report");
        assert_eq!(manifest.pages_path(), "pages");
        assert_eq!(manifest.assets_path(), "assets");
        assert!(dir.join("pages/p01.html").exists(), "入口文件不存在");

        // 总表 22 行 + 营业执照图片变量 → 23 个变量
        assert_eq!(manifest.variables.len(), 23);

        // 分页清单 34 页，且每页都带几何信息
        assert_eq!(manifest.pages.len(), 34);
        assert!(manifest.pages.iter().all(|p| p.file.starts_with("pages/")
            && p.orientation.is_some()
            && p.mt.is_some()));

        // 目录（大纲）
        assert_eq!(manifest.toc.len(), 16);
        assert_eq!(manifest.toc[0].title, "结算审核报告（封面）");
        assert_eq!(manifest.toc[0].page, 1);
        assert!(manifest.toc.iter().all(|t| t.page >= 1 && t.page <= 34));

        // 6 个由其他变量派生的项（引用/计算）
        let computed: Vec<&str> = manifest
            .variables
            .iter()
            .filter(|v| v.ty == "computed")
            .map(|v| v.key.as_str())
            .collect();
        assert_eq!(
            computed,
            vec![
                "audit_date",
                "audit_fee",
                "deducted_amount",
                "survey_date",
                "audit_start_date",
                "audit_end_date"
            ]
        );

        // 每个变量都有中文别名（显示名）
        assert!(manifest
            .variables
            .iter()
            .all(|v| v.alias.as_deref().map_or(false, |a| !a.is_empty())));
    }

    #[test]
    fn print_document_assembles_pages() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/settle-audit-report");
        let manifest = report_core::templates::read_manifest(&dir).unwrap();
        let values = serde_json::Map::new();

        let html = report_core::render::render_print_html(&dir, &manifest, &values).unwrap();

        assert!(html.contains("<base href=\"pages/\">"));
        assert!(html.contains("../assets/base.css"));
        assert!(html.contains("@page pg1 { size: A4 portrait; margin: 0; }"));
        assert!(html.contains("@page pg2 { size: A4 landscape; margin: 0; }"));
        assert!(html.contains("data-page=\"pg1\""));
        assert!(html.contains("data-page=\"pg34\""));
    }

    #[test]
    fn pages_render_placeholders() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates/settle-audit-report");
        let manifest = report_core::templates::read_manifest(&dir).unwrap();

        let mut values = serde_json::Map::new();
        values.insert("project_name".into(), serde_json::json!("怀仁市城镇第六小学校外墙修缮改造工程"));
        values.insert("site".into(), serde_json::json!("怀仁市城镇第六小学校"));
        values.insert("owner".into(), serde_json::json!("怀仁市城镇第六小学校"));
        values.insert("contractor".into(), serde_json::json!("山西尊盛达建设工程有限公司"));
        values.insert("agency".into(), serde_json::json!("山西红实工程项目管理有限公司"));
        values.insert("contract_no".into(), serde_json::json!("SXHS-2026-243"));
        values.insert("reported_amount".into(), serde_json::json!(140422.90));
        values.insert("approved_amount".into(), serde_json::json!(135613.17));
        values.insert("plan_start_date".into(), serde_json::json!("2026-09-07"));
        values.insert("plan_end_date".into(), serde_json::json!("2026-09-11"));
        values.insert("auditor".into(), serde_json::json!("张美青"));
        values.insert("scope".into(), serde_json::json!("拆除文化墙砖面层、涂料面层、雨棚、挑檐涂料面层，恢复墙面、挑檐面层"));
        values.insert("contract_date".into(), serde_json::json!("2026-09-07"));

        let html = report_core::render::render_print_html(&dir, &manifest, &values).unwrap();

        assert!(html.contains("怀仁市城镇第六小学校外墙修缮改造工程"));
        assert!(html.contains("山西尊盛达建设工程有限公司"));
        assert!(html.contains("山西红实工程项目管理有限公司"));
        assert!(html.contains("SXHS-2026-243"));
        assert!(html.contains("140422.90"));
        assert!(html.contains("壹拾叁万伍仟陆佰壹拾叁元壹角柒分"), "大写未生成");
        assert!(html.contains("2026年9月11日"), "date_cn 未生成");
        assert!(html.contains("2026.9.11"), "date_dot 未生成");
        // 字段标记
        assert!(
            html.contains("class=\"rs-field\" data-field=\"project_name\""),
            "字段标记未生成"
        );
        // 不应残留未替换的占位符
        assert!(!html.contains("{{"));
    }
}

