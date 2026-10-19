use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
pub struct PrintPage {
    pub width_in: f64,
    pub height_in: f64,
    pub landscape: bool,
}

/// 在给定 webview 上弹出系统打印对话框（可选打印机/参数）。
#[cfg(windows)]
pub fn print_webview_dialog(webview: &tauri::webview::PlatformWebview) -> Result<(), String> {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2_16, COREWEBVIEW2_PRINT_DIALOG_KIND_SYSTEM,
    };
    use windows_core::Interface;

    let controller = webview.controller();
    let core = unsafe { controller.CoreWebView2() }
        .map_err(|e| format!("获取 CoreWebView2 失败: {e}"))?;
    let core16: ICoreWebView2_16 = core
        .cast()
        .map_err(|e| format!("转换为 ICoreWebView2_16 失败（WebView2 运行时可能过旧）: {e}"))?;
    unsafe { core16.ShowPrintUI(COREWEBVIEW2_PRINT_DIALOG_KIND_SYSTEM) }
        .map_err(|e| format!("显示打印对话框失败: {e}"))?;
    Ok(())
}

#[cfg(not(windows))]
pub fn print_webview_dialog(_: &tauri::webview::PlatformWebview) -> Result<(), String> {
    Err("仅支持 Windows".to_string())
}

#[cfg(windows)]
pub fn print_webview_to_pdf(
    webview: &tauri::webview::PlatformWebview,
    out: &Path,
    page: Option<PrintPage>,
) -> Result<(), String> {
    use tauri::webview::PlatformWebview;
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2_7, ICoreWebView2Environment6, COREWEBVIEW2_PRINT_ORIENTATION_LANDSCAPE,
        COREWEBVIEW2_PRINT_ORIENTATION_PORTRAIT,
    };
    use webview2_com::PrintToPdfCompletedHandler;
    use windows_core::{Interface, HSTRING, PCWSTR};

    let _: &PlatformWebview = webview;

    let controller = webview.controller();
    let core = unsafe { controller.CoreWebView2() }
        .map_err(|e| format!("获取 CoreWebView2 失败: {e}"))?;
    let core7: ICoreWebView2_7 = core
        .cast()
        .map_err(|e| format!("转换为 ICoreWebView2_7 失败: {e}"))?;

    let env6: ICoreWebView2Environment6 = webview
        .environment()
        .cast()
        .map_err(|e| format!("转换为 ICoreWebView2Environment6 失败: {e}"))?;
    let settings = unsafe { env6.CreatePrintSettings() }
        .map_err(|e| format!("创建打印设置失败: {e}"))?;
    unsafe {
        let _ = settings.SetShouldPrintBackgrounds(true);
        let _ = settings.SetShouldPrintHeaderAndFooter(false);
        if let Some(page) = page {
            let _ = settings.SetPageWidth(page.width_in);
            let _ = settings.SetPageHeight(page.height_in);
            let _ = settings.SetMarginTop(0.0);
            let _ = settings.SetMarginRight(0.0);
            let _ = settings.SetMarginBottom(0.0);
            let _ = settings.SetMarginLeft(0.0);
            let _ = settings.SetScaleFactor(1.0);
            let _ = settings.SetOrientation(if page.landscape {
                COREWEBVIEW2_PRINT_ORIENTATION_LANDSCAPE
            } else {
                COREWEBVIEW2_PRINT_ORIENTATION_PORTRAIT
            });
        }
    }

    let path_str = out.to_string_lossy().to_string();
    let (res_tx, res_rx) = std::sync::mpsc::channel::<Result<(), String>>();

    let result = PrintToPdfCompletedHandler::wait_for_async_operation(
        Box::new(move |handler| unsafe {
            let path_h = HSTRING::from(path_str);
            let pcwstr = PCWSTR::from_raw(path_h.as_ptr());
            core7
                .PrintToPdf(pcwstr, &settings, &handler)
                .map_err(webview2_com::Error::WindowsError)
        }),
        Box::new(move |hr, is_success| {
            let r = if let Err(e) = hr {
                Err(format!("打印失败: {e}"))
            } else if !is_success {
                Err("PrintToPdf 未成功".to_string())
            } else {
                Ok(())
            };
            let _ = res_tx.send(r);
            Ok(())
        }),
    );

    if let Err(e) = result {
        return Err(format!("等待打印完成失败: {e}"));
    }
    match res_rx.recv() {
        Ok(r) => r?,
        Err(_) => return Err("未收到打印结果".to_string()),
    }
    if !out.exists() {
        return Err("PDF 文件未生成".to_string());
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn print_webview_to_pdf(
    _: &tauri::webview::PlatformWebview,
    _: &Path,
    _: Option<PrintPage>,
) -> Result<(), String> {
    Err("仅支持 Windows".to_string())
}

/// 创建一个隐藏 webview 载入报告，待加载完成后用进程内 PrintToPdf 导出。
pub fn print_via_webview(
    app: &tauri::AppHandle,
    url: &str,
    out: &Path,
    page: Option<PrintPage>,
) -> Result<(), String> {
    use std::sync::mpsc;
    use std::time::Duration;
    use tauri::webview::PageLoadEvent;
    use tauri::{WebviewUrl, WebviewWindowBuilder};

    let parsed: url::Url = url.parse().map_err(|e: url::ParseError| e.to_string())?;
    let label = format!("pdfgen-{}", uuid::Uuid::new_v4());

    let (load_tx, load_rx) = mpsc::channel::<()>();
    let window = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(parsed))
        .visible(false)
        .on_page_load(move |_w, payload| {
            if matches!(payload.event(), PageLoadEvent::Finished) {
                let _ = load_tx.send(());
            }
        })
        .build()
        .map_err(|e| format!("创建打印窗口失败: {e}"))?;

    if load_rx.recv_timeout(Duration::from_secs(30)).is_err() {
        let _ = window.close();
        return Err("页面加载超时".to_string());
    }

    let (done_tx, done_rx) = mpsc::channel::<Result<(), String>>();
    let out2 = out.to_path_buf();
    window
        .with_webview(move |webview| {
            let res = print_webview_to_pdf(&webview, &out2, page);
            let _ = done_tx.send(res);
        })
        .map_err(|e| format!("with_webview 调用失败: {e}"))?;

    let result = done_rx
        .recv_timeout(Duration::from_secs(60))
        .map_err(|_| "打印超时".to_string())?;

    let _ = window.close();
    result
}

/// 打开一个可见的打印窗口载入报告，加载完成后弹出系统打印对话框（可选打印机/参数）。
/// 窗口保留给用户操作，不自动关闭。
pub fn print_via_webview_dialog(app: &tauri::AppHandle, url: &str) -> Result<(), String> {
    use std::sync::mpsc;
    use std::time::Duration;
    use tauri::webview::PageLoadEvent;
    use tauri::{WebviewUrl, WebviewWindowBuilder};

    let parsed: url::Url = url.parse().map_err(|e: url::ParseError| e.to_string())?;
    let label = format!("print-{}", uuid::Uuid::new_v4());

    let (load_tx, load_rx) = mpsc::channel::<()>();
    let window = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(parsed))
        .title("打印")
        .inner_size(900.0, 700.0)
        .on_page_load(move |_w, payload| {
            if matches!(payload.event(), PageLoadEvent::Finished) {
                let _ = load_tx.send(());
            }
        })
        .build()
        .map_err(|e| format!("创建打印窗口失败: {e}"))?;

    if load_rx.recv_timeout(Duration::from_secs(30)).is_err() {
        let _ = window.close();
        return Err("页面加载超时".to_string());
    }

    let (done_tx, done_rx) = mpsc::channel::<Result<(), String>>();
    window
        .with_webview(move |webview| {
            let res = print_webview_dialog(&webview);
            let _ = done_tx.send(res);
        })
        .map_err(|e| format!("with_webview 调用失败: {e}"))?;

    done_rx
        .recv_timeout(Duration::from_secs(30))
        .map_err(|_| "打印调用超时".to_string())?
}

/// 按传入顺序合并逐页生成的 PDF，保留每页自己的 MediaBox。
pub fn merge_pdfs(paths: &[PathBuf], out: &Path) -> Result<(), String> {
    use lopdf::{Document, Object, ObjectId};
    use std::collections::BTreeMap;

    if paths.is_empty() {
        return Err("没有可合并的 PDF".to_string());
    }
    if paths.len() == 1 {
        std::fs::copy(&paths[0], out)
            .map_err(|e| format!("写入 PDF 失败：{e}"))?;
        return Ok(());
    }

    let mut max_id = 1u32;
    let mut pages: Vec<(ObjectId, Object)> = Vec::new();
    let mut objects: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut document = Document::with_version("1.5");

    for path in paths {
        let mut doc = Document::load(path)
            .map_err(|e| format!("加载临时 PDF 失败 {}：{e}", path.display()))?;
        doc.renumber_objects_with(max_id);
        max_id = doc.max_id + 1;

        for (_, object_id) in doc.get_pages() {
            if let Ok(object) = doc.get_object(object_id) {
                pages.push((object_id, object.to_owned()));
            }
        }
        objects.extend(doc.objects);
    }

    if pages.is_empty() {
        return Err("临时 PDF 中没有页面".to_string());
    }

    let mut catalog: Option<(ObjectId, Object)> = None;
    let mut pages_root: Option<(ObjectId, Object)> = None;

    for (object_id, object) in objects.iter() {
        match object.type_name().unwrap_or(b"") {
            b"Catalog" => {
                if catalog.is_none() {
                    catalog = Some((*object_id, object.clone()));
                }
            }
            b"Pages" => {
                let mut dict = object
                    .as_dict()
                    .map_err(|e| e.to_string())?
                    .clone();
                if let Some((_, old)) = pages_root.as_ref() {
                    if let Ok(old_dict) = old.as_dict() {
                        dict.extend(old_dict);
                    }
                }
                let id = pages_root
                    .as_ref()
                    .map(|(id, _)| *id)
                    .unwrap_or(*object_id);
                pages_root = Some((id, Object::Dictionary(dict)));
            }
            b"Page" | b"Outlines" | b"Outline" => {}
            _ => {
                document.objects.insert(*object_id, object.clone());
            }
        }
    }

    let (pages_id, pages_object) =
        pages_root.ok_or_else(|| "临时 PDF 缺少 Pages 根节点".to_string())?;
    let (catalog_id, catalog_object) =
        catalog.ok_or_else(|| "临时 PDF 缺少 Catalog 根节点".to_string())?;

    let mut kids = Vec::with_capacity(pages.len());
    for (object_id, object) in pages {
        let mut dict = object
            .as_dict()
            .map_err(|e| e.to_string())?
            .clone();
        dict.set("Parent", Object::Reference(pages_id));
        document
            .objects
            .insert(object_id, Object::Dictionary(dict));
        kids.push(Object::Reference(object_id));
    }

    let mut pages_dict = pages_object
        .as_dict()
        .map_err(|e| e.to_string())?
        .clone();
    pages_dict.set("Count", kids.len() as u32);
    pages_dict.set("Kids", kids);
    document
        .objects
        .insert(pages_id, Object::Dictionary(pages_dict));

    let mut catalog_dict = catalog_object
        .as_dict()
        .map_err(|e| e.to_string())?
        .clone();
    catalog_dict.set("Pages", Object::Reference(pages_id));
    catalog_dict.remove(b"Outlines");
    document
        .objects
        .insert(catalog_id, Object::Dictionary(catalog_dict));

    document.trailer.set("Root", Object::Reference(catalog_id));
    document.max_id = document.objects.len() as u32;
    document.renumber_objects();
    document.adjust_zero_pages();
    document.compress();
    document
        .save(out)
        .map_err(|e| format!("合并 PDF 失败：{e}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{dictionary, Document, Object, Stream};

    fn write_test_pdf(path: &Path, width: i64, height: i64) {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let content_id = doc.add_object(Stream::new(dictionary! {}, Vec::new()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "MediaBox" => vec![0.into(), 0.into(), width.into(), height.into()],
        });
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => vec![page_id.into()],
                "Count" => 1,
            }),
        );
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);
        doc.save(path).unwrap();
    }

    #[test]
    fn merge_preserves_page_order_and_sizes() {
        let dir = std::env::temp_dir().join(format!("rs-pdf-merge-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.pdf");
        let b = dir.join("b.pdf");
        let out = dir.join("out.pdf");
        write_test_pdf(&a, 595, 842);
        write_test_pdf(&b, 842, 595);

        merge_pdfs(&[a, b], &out).unwrap();

        let doc = Document::load(&out).unwrap();
        let pages = doc.get_pages();
        assert_eq!(pages.len(), 2);
        let boxes: Vec<Vec<i64>> = pages
            .values()
            .map(|id| {
                let page = doc.get_object(*id).unwrap().as_dict().unwrap();
                page.get(b"MediaBox")
                    .unwrap()
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_i64().unwrap())
                    .collect()
            })
            .collect();
        assert_eq!(boxes[0], vec![0, 0, 595, 842]);
        assert_eq!(boxes[1], vec![0, 0, 842, 595]);

        let _ = std::fs::remove_dir_all(&dir);
    }
}

