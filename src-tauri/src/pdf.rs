use std::path::Path;

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
pub fn print_webview_to_pdf(webview: &tauri::webview::PlatformWebview, out: &Path) -> Result<(), String> {
    use tauri::webview::PlatformWebview;
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2_7, ICoreWebView2Environment6,
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
pub fn print_webview_to_pdf(_: &tauri::webview::PlatformWebview, _: &Path) -> Result<(), String> {
    Err("仅支持 Windows".to_string())
}

/// 创建一个隐藏 webview 载入报告，待加载完成后用进程内 PrintToPdf 导出。
pub fn print_via_webview(
    app: &tauri::AppHandle,
    url: &str,
    out: &Path,
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
            let res = print_webview_to_pdf(&webview, &out2);
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

