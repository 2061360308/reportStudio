use crate::commands::AppPaths;
use futures_util::StreamExt;
use report_core::model::TemplateInfo;
use report_core::templates;
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::ipc::Channel;
use tauri::{AppHandle, State};

const DEFAULT_USER_AGENT: &str = "report-studio-template-sync";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LatestManifest {
    #[serde(default)]
    templates: Vec<RemoteTemplate>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteTemplate {
    id: String,
    name: String,
    version: String,
    #[serde(default)]
    notes: String,
    url: String,
    sha256: String,
    #[serde(default)]
    size: Option<u64>,
    #[serde(default)]
    min_app_version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateUpdateInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub current_version: Option<String>,
    pub notes: String,
    pub size: Option<u64>,
    pub min_app_version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncEvent {
    pub phase: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub current: usize,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl SyncEvent {
    fn new(phase: &str, current: usize, total: usize) -> Self {
        Self {
            phase: phase.to_string(),
            id: None,
            current,
            total,
            percent: None,
            message: None,
        }
    }

    fn for_template(mut self, id: &str) -> Self {
        self.id = Some(id.to_string());
        self
    }

    fn with_percent(mut self, percent: Option<u32>) -> Self {
        self.percent = percent;
        self
    }

    fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

struct SyncGuard<'a>(&'a AtomicBool);

impl Drop for SyncGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

fn manifest_urls(app: &AppHandle) -> Result<Vec<String>, String> {
    let plugins = &app.config().plugins.0;
    let updater = plugins
        .get("updater")
        .ok_or_else(|| "tauri.conf.json 缺少 plugins.updater 配置".to_string())?;
    let endpoints = updater
        .get("endpoints")
        .and_then(|value| value.as_array())
        .ok_or_else(|| "tauri.conf.json 缺少 updater.endpoints".to_string())?;
    let urls = endpoints
        .iter()
        .filter_map(|value| value.as_str())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if urls.is_empty() {
        return Err("updater.endpoints 为空".to_string());
    }
    Ok(urls)
}

fn with_cache_bust(url: &str) -> String {
    let sep = if url.contains('?') { '&' } else { '?' };
    format!("{url}{sep}t={}", chrono::Utc::now().timestamp())
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .user_agent(DEFAULT_USER_AGENT)
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))
}

async fn fetch_manifest_from(
    client: &reqwest::Client,
    url: &str,
) -> Result<LatestManifest, String> {
    let url = with_cache_bust(url);
    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .map_err(|e| format!("获取更新清单失败: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("获取更新清单失败: HTTP {}", response.status()));
    }
    response
        .json::<LatestManifest>()
        .await
        .map_err(|e| format!("解析更新清单失败: {e}"))
}

async fn fetch_manifest(app: &AppHandle) -> Result<LatestManifest, String> {
    let client = http_client()?;
    let mut errors = Vec::new();
    for url in manifest_urls(app)? {
        match fetch_manifest_from(&client, &url).await {
            Ok(manifest) => return Ok(manifest),
            Err(error) => errors.push(format!("{url}: {error}")),
        }
    }
    Err(format!("获取更新清单失败：{}", errors.join("；")))
}

fn current_app_version(app: &AppHandle) -> Result<Version, String> {
    Version::parse(&app.package_info().version.to_string())
        .map_err(|e| format!("当前应用版本无效: {e}"))
}

fn is_eligible(remote: &RemoteTemplate, app_version: &Version) -> bool {
    let Some(min) = remote.min_app_version.as_deref() else {
        return true;
    };
    match Version::parse(min) {
        Ok(min) => app_version >= &min,
        Err(_) => true,
    }
}

fn update_available(
    remote: &RemoteTemplate,
    local: Option<&TemplateInfo>,
    app_version: &Version,
) -> Result<Option<TemplateUpdateInfo>, String> {
    if !is_eligible(remote, app_version) {
        return Ok(None);
    }

    let remote_version = Version::parse(&remote.version)
        .map_err(|e| format!("模板 {} 的远端版本无效: {e}", remote.id))?;

    let Some(local) = local else {
        return Ok(Some(TemplateUpdateInfo {
            id: remote.id.clone(),
            name: remote.name.clone(),
            version: remote.version.clone(),
            current_version: None,
            notes: remote.notes.clone(),
            size: remote.size,
            min_app_version: remote.min_app_version.clone(),
        }));
    };

    let local_version = Version::parse(&local.version)
        .map_err(|e| format!("模板 {} 的本地版本无效: {e}", local.id))?;
    if remote_version <= local_version {
        return Ok(None);
    }

    Ok(Some(TemplateUpdateInfo {
        id: remote.id.clone(),
        name: remote.name.clone(),
        version: remote.version.clone(),
        current_version: Some(local.version.clone()),
        notes: remote.notes.clone(),
        size: remote.size,
        min_app_version: remote.min_app_version.clone(),
    }))
}

fn pending_updates(
    manifest: &LatestManifest,
    state: &AppPaths,
    app_version: &Version,
) -> Result<Vec<(RemoteTemplate, TemplateUpdateInfo)>, String> {
    let locals = templates::list_templates(&state.templates_dir)?;
    let local_map: std::collections::HashMap<&str, &TemplateInfo> =
        locals.iter().map(|item| (item.id.as_str(), item)).collect();

    let mut pending = Vec::new();
    for remote in &manifest.templates {
        if let Some(info) = update_available(remote, local_map.get(remote.id.as_str()).copied(), app_version)? {
            pending.push((remote.clone(), info));
        }
    }
    Ok(pending)
}

#[tauri::command]
pub async fn check_template_updates(
    app: AppHandle,
    state: State<'_, AppPaths>,
) -> Result<Vec<TemplateUpdateInfo>, String> {
    let manifest = fetch_manifest(&app).await?;
    let app_version = current_app_version(&app)?;
    let pending = pending_updates(&manifest, &state, &app_version)?;
    Ok(pending.into_iter().map(|(_, info)| info).collect())
}

#[tauri::command]
pub async fn sync_templates(
    app: AppHandle,
    state: State<'_, AppPaths>,
    on_event: Channel<SyncEvent>,
) -> Result<Vec<TemplateInfo>, String> {
    if state.template_sync.swap(true, Ordering::SeqCst) {
        return Err("模板正在同步，请稍候".to_string());
    }
    let _guard = SyncGuard(&state.template_sync);

    let manifest = fetch_manifest(&app).await?;
    let app_version = current_app_version(&app)?;
    let pending = pending_updates(&manifest, &state, &app_version)?;
    let total = pending.len();

    let _ = on_event.send(SyncEvent::new("start", 0, total));

    if total == 0 {
        let installed = templates::list_templates(&state.templates_dir)?;
        let _ = on_event.send(SyncEvent::new("done", 0, 0));
        return Ok(installed);
    }

    let client = http_client()?;
    for (index, (remote, info)) in pending.into_iter().enumerate() {
        let _ = on_event.send(
            SyncEvent::new("downloading", index, total)
                .for_template(&remote.id)
                .with_percent(Some(0))
                .with_message(format!("下载 {} v{}", info.name, info.version)),
        );

        let response = client
            .get(&remote.url)
            .header("Accept", "application/octet-stream")
            .send()
            .await
            .map_err(|e| format!("下载模板 {} 失败: {e}", remote.id))?;
        if !response.status().is_success() {
            return Err(format!(
                "下载模板 {} 失败: HTTP {}",
                remote.id,
                response.status()
            ));
        }

        let content_length = response
            .content_length()
            .or(remote.size)
            .filter(|size| *size > 0);
        let mut data = Vec::with_capacity(
            content_length
                .map(|size| size.min(64 * 1024 * 1024) as usize)
                .unwrap_or(0),
        );
        let mut downloaded: u64 = 0;
        let mut last_percent: Option<u32> = None;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("下载模板 {} 失败: {e}", remote.id))?;
            data.extend_from_slice(&chunk);
            downloaded += chunk.len() as u64;
            if let Some(total_bytes) = content_length {
                let percent = ((downloaded.saturating_mul(100) / total_bytes).min(100)) as u32;
                if last_percent != Some(percent) {
                    last_percent = Some(percent);
                    let _ = on_event.send(
                        SyncEvent::new("downloading", index, total)
                            .for_template(&remote.id)
                            .with_percent(Some(percent)),
                    );
                }
            }
        }

        if let Some(expected) = remote.size {
            if expected > 0 && data.len() as u64 != expected {
                return Err(format!(
                    "模板 {} 大小不匹配：期望 {expected}，实际 {}",
                    remote.id,
                    data.len()
                ));
            }
        }

        let actual_hash = hex::encode(Sha256::digest(&data));
        if !actual_hash.eq_ignore_ascii_case(&remote.sha256) {
            return Err(format!("模板 {} 的 sha256 校验失败", remote.id));
        }

        let _ = on_event.send(
            SyncEvent::new("installing", index, total)
                .for_template(&remote.id)
                .with_message(format!("安装 {} v{}", info.name, info.version)),
        );

        install_archive(&state.templates_dir, &remote, &data)?;

        let _ = on_event.send(
            SyncEvent::new("installed", index + 1, total)
                .for_template(&remote.id)
                .with_percent(Some(100)),
        );
    }

    let installed = templates::list_templates(&state.templates_dir)?;
    let _ = on_event.send(SyncEvent::new("done", total, total));
    Ok(installed)
}

fn install_archive(
    templates_dir: &Path,
    remote: &RemoteTemplate,
    bytes: &[u8],
) -> Result<TemplateInfo, String> {
    let token = uuid::Uuid::new_v4();
    let tmp_root = templates_dir.join(format!(".tmp-{token}"));
    let zip_path = templates_dir.join(format!(".download-{token}.zip"));
    let extract_dir = tmp_root.join("extract");
    let dest = templates::template_dir(templates_dir, &remote.id);
    let backup = templates_dir.join(format!(
        ".bak-{}-{token}",
        templates::sanitize_id(&remote.id)
    ));

    let result = (|| -> Result<TemplateInfo, String> {
        fs::create_dir_all(&extract_dir).map_err(|e| e.to_string())?;
        fs::write(&zip_path, bytes).map_err(|e| e.to_string())?;
        templates::extract_zip(&zip_path, &extract_dir)?;

        let root = templates::find_template_root(&extract_dir);
        let mut manifest = templates::read_manifest(&root)?;
        if manifest.id != remote.id {
            return Err(format!(
                "模板 id 不匹配：期望 {}，实际 {}",
                remote.id, manifest.id
            ));
        }
        let local_version = Version::parse(&manifest.version)
            .map_err(|e| format!("模板 {} 的 manifest 版本无效: {e}", remote.id))?;
        let remote_version = Version::parse(&remote.version)
            .map_err(|e| format!("模板 {} 的远端版本无效: {e}", remote.id))?;
        if local_version != remote_version {
            return Err(format!(
                "模板 {} 版本不匹配：远端 {}，包内 {}",
                remote.id, remote.version, manifest.version
            ));
        }

        manifest.content_hash = templates::compute_content_hash(&root)?;
        templates::write_manifest(&root, &manifest)?;

        if dest.exists() {
            if backup.exists() {
                let _ = fs::remove_dir_all(&backup);
            }
            fs::rename(&dest, &backup).map_err(|e| format!("备份旧模板失败: {e}"))?;
        }

        if let Err(e) = fs::rename(&root, &dest) {
            if backup.exists() {
                let _ = fs::rename(&backup, &dest);
            }
            return Err(format!("安装模板 {} 失败: {e}", remote.id));
        }

        if backup.exists() {
            let _ = fs::remove_dir_all(&backup);
        }

        Ok(TemplateInfo {
            id: manifest.id.clone(),
            version: manifest.version.clone(),
            name: manifest.name.clone(),
            content_hash: manifest.content_hash.clone(),
        })
    })();

    let _ = fs::remove_file(&zip_path);
    let _ = fs::remove_dir_all(&tmp_root);
    if result.is_err() && backup.exists() {
        let _ = fs::remove_dir_all(&backup);
    }
    result
}
