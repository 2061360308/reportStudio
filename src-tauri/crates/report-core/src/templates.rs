use crate::model::{TemplateInfo, TemplateManifest};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;

pub fn sanitize_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// 资源文件名：内容 sha256 前 8 位 hex + 扩展名（内容寻址，天然去重）。
pub fn resource_name(bytes: &[u8], ext: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let hexs = hex::encode(hasher.finalize());
    let ext = ext.trim_start_matches('.').to_ascii_lowercase();
    if ext.is_empty() {
        hexs[..8].to_string()
    } else {
        format!("{}.{}", &hexs[..8], ext)
    }
}

pub fn template_dir(templates_dir: &Path, id: &str) -> PathBuf {
    templates_dir.join(sanitize_id(id))
}

/// 规范化内容指纹：按相对路径排序，逐个混入 路径+长度+内容，忽略 manifest.json（避免自引用）。
pub fn compute_content_hash(dir: &Path) -> Result<String, String> {
    let mut files: Vec<PathBuf> = WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect();
    files.sort();

    let mut hasher = Sha256::new();
    for f in files {
        let rel = f
            .strip_prefix(dir)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        if rel == "manifest.json" {
            continue;
        }
        let bytes = fs::read(&f).map_err(|e| e.to_string())?;
        hasher.update(rel.as_bytes());
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(&bytes);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// 快照指纹：**包含 manifest.json** 在内的完整内容指纹（用于内嵌模板缓存分桶）。
/// 与 `compute_content_hash` 不同，这里不忽略 manifest，因为变量定义的变化也必须区分。
pub fn compute_snapshot_hash(dir: &Path) -> Result<String, String> {
    let mut files: Vec<PathBuf> = WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect();
    files.sort();

    let mut hasher = Sha256::new();
    for f in files {
        let rel = f
            .strip_prefix(dir)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        let bytes = fs::read(&f).map_err(|e| e.to_string())?;
        hasher.update(rel.as_bytes());
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(&bytes);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn read_manifest(dir: &Path) -> Result<TemplateManifest, String> {
    let p = dir.join("manifest.json");
    let txt = fs::read_to_string(&p).map_err(|e| format!("读取 manifest 失败: {e}"))?;
    serde_json::from_str(&txt).map_err(|e| format!("解析 manifest 失败: {e}"))
}

pub fn write_manifest(dir: &Path, m: &TemplateManifest) -> Result<(), String> {
    let p = dir.join("manifest.json");
    let txt = serde_json::to_string_pretty(m).map_err(|e| e.to_string())?;
    fs::write(p, txt).map_err(|e| e.to_string())
}

pub fn list_templates(templates_dir: &Path) -> Result<Vec<TemplateInfo>, String> {
    let mut out = Vec::new();
    if !templates_dir.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(templates_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            continue;
        }
        if let Ok(m) = read_manifest(&entry.path()) {
            out.push(TemplateInfo {
                id: m.id,
                version: m.version,
                name: m.name,
                content_hash: m.content_hash,
            });
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

pub fn extract_zip(zip_path: &Path, dest: &Path) -> Result<(), String> {
    let file = fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    archive.extract(dest).map_err(|e| e.to_string())?;
    Ok(())
}

/// 若解压根下没有 manifest.json，但只有一个子目录夹着它，则返回该子目录。
pub fn find_template_root(dir: &Path) -> PathBuf {
    if dir.join("manifest.json").exists() {
        return dir.to_path_buf();
    }
    if let Ok(rd) = fs::read_dir(dir) {
        let dirs: Vec<_> = rd
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .collect();
        if dirs.len() == 1 && dirs[0].path().join("manifest.json").exists() {
            return dirs[0].path();
        }
    }
    dir.to_path_buf()
}

pub fn install_template(templates_dir: &Path, zip_path: &Path) -> Result<TemplateInfo, String> {
    let tmp = std::env::temp_dir().join(format!("rs-tpl-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let result = (|| {
        extract_zip(zip_path, &tmp)?;
        let root = find_template_root(&tmp);
        let mut manifest = read_manifest(&root)?;
        manifest.content_hash = compute_content_hash(&root)?;

        let dest = template_dir(templates_dir, &manifest.id);
        if dest.exists() {
            fs::remove_dir_all(&dest).map_err(|e| e.to_string())?;
        }
        fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
        copy_dir(&root, &dest)?;
        write_manifest(&dest, &manifest)?;

        Ok(TemplateInfo {
            id: manifest.id.clone(),
            version: manifest.version.clone(),
            name: manifest.name.clone(),
            content_hash: manifest.content_hash.clone(),
        })
    })();
    let _ = fs::remove_dir_all(&tmp);
    result
}

pub fn export_template(templates_dir: &Path, id: &str, out: &Path) -> Result<(), String> {
    let src = template_dir(templates_dir, id);
    if !src.exists() {
        return Err(format!("模板不存在: {id}"));
    }
    let file = fs::File::create(out).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let opts: SimpleFileOptions =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for item in WalkDir::new(&src).into_iter().filter_map(|e| e.ok()) {
        let path = item.path();
        let rel = path.strip_prefix(&src).map_err(|e| e.to_string())?;
        let name = rel.to_string_lossy().replace('\\', "/");
        if name.is_empty() {
            continue;
        }
        if item.file_type().is_dir() {
            zip.add_directory(format!("{name}/"), opts).map_err(|e| e.to_string())?;
        } else {
            zip.start_file(name, opts).map_err(|e| e.to_string())?;
            let bytes = fs::read(path).map_err(|e| e.to_string())?;
            use std::io::Write;
            zip.write_all(&bytes).map_err(|e| e.to_string())?;
        }
    }
    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn copy_dir(src: &Path, dest: &Path) -> Result<(), String> {
    for item in WalkDir::new(src).into_iter().filter_map(|e| e.ok()) {
        let path = item.path();
        let rel = path.strip_prefix(src).map_err(|e| e.to_string())?;
        let target = dest.join(rel);
        if item.file_type().is_dir() {
            fs::create_dir_all(&target).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::copy(path, &target).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
