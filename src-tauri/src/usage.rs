use report_core::templates::sanitize_id;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const MAX_RECENT_VALUES: usize = 5;
const MAX_USAGE_FILE_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecentValue {
    pub value: Value,
    pub used_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateUsage {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub template_id: String,
    #[serde(default)]
    pub last_used_at: Option<String>,
    #[serde(default)]
    pub variables: BTreeMap<String, Vec<RecentValue>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateUsageSummary {
    pub id: String,
    pub last_used_at: Option<String>,
}

fn default_schema_version() -> u32 {
    1
}

fn empty_usage(template_id: &str) -> TemplateUsage {
    TemplateUsage {
        schema_version: 1,
        template_id: template_id.to_string(),
        last_used_at: None,
        variables: BTreeMap::new(),
    }
}

fn now() -> String {
    chrono::Local::now().to_rfc3339()
}

fn usage_path(usage_dir: &Path, template_id: &str) -> PathBuf {
    usage_dir.join(format!("{}.json", sanitize_id(template_id)))
}

pub fn read_usage(usage_dir: &Path, template_id: &str) -> TemplateUsage {
    let path = usage_path(usage_dir, template_id);
    let Ok(metadata) = fs::metadata(&path) else {
        return empty_usage(template_id);
    };
    if metadata.len() > MAX_USAGE_FILE_BYTES {
        return empty_usage(template_id);
    }
    let Ok(text) = fs::read_to_string(&path) else {
        return empty_usage(template_id);
    };
    let Ok(mut usage) = serde_json::from_str::<TemplateUsage>(&text) else {
        return empty_usage(template_id);
    };
    if usage.template_id.trim().is_empty() {
        usage.template_id = template_id.to_string();
    }
    usage
}

fn write_usage(usage_dir: &Path, usage: &TemplateUsage) -> Result<(), String> {
    fs::create_dir_all(usage_dir).map_err(|e| e.to_string())?;
    let path = usage_path(usage_dir, &usage.template_id);
    let text = serde_json::to_string_pretty(usage).map_err(|e| e.to_string())?;
    let tmp = usage_dir.join(format!(
        ".{}.tmp-{}",
        sanitize_id(&usage.template_id),
        uuid::Uuid::new_v4()
    ));

    fs::write(&tmp, text.as_bytes()).map_err(|e| e.to_string())?;
    if path.exists() {
        if let Err(e) = fs::remove_file(&path) {
            let _ = fs::remove_file(&tmp);
            return Err(e.to_string());
        }
    }
    if let Err(e) = fs::rename(&tmp, &path) {
        let _ = fs::remove_file(&tmp);
        return Err(e.to_string());
    }
    Ok(())
}

pub fn touch_template(usage_dir: &Path, template_id: &str) -> Result<(), String> {
    let mut usage = read_usage(usage_dir, template_id);
    usage.last_used_at = Some(now());
    write_usage(usage_dir, &usage)
}

fn is_recordable(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(value) => !value.trim().is_empty(),
        Value::Bool(_) => false,
        Value::Array(_) | Value::Object(_) => false,
        Value::Number(_) => true,
    }
}

pub fn record_values(
    usage_dir: &Path,
    template_id: &str,
    values: &Map<String, Value>,
) -> Result<(), String> {
    let mut usage = read_usage(usage_dir, template_id);
    let used_at = now();

    for (key, value) in values {
        if !is_recordable(value) {
            continue;
        }
        let encoded = serde_json::to_string(value).unwrap_or_default();
        let list = usage.variables.entry(key.clone()).or_default();
        list.retain(|item| serde_json::to_string(&item.value).unwrap_or_default() != encoded);
        list.insert(
            0,
            RecentValue {
                value: value.clone(),
                used_at: used_at.clone(),
            },
        );
        list.truncate(MAX_RECENT_VALUES);
    }

    usage.last_used_at = Some(used_at);
    write_usage(usage_dir, &usage)
}

pub fn list_usage(usage_dir: &Path) -> Vec<TemplateUsageSummary> {
    let mut summaries = Vec::new();
    let Ok(entries) = fs::read_dir(usage_dir) else {
        return summaries;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };
        if metadata.len() > MAX_USAGE_FILE_BYTES {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(usage) = serde_json::from_str::<TemplateUsage>(&text) else {
            continue;
        };
        if usage.template_id.trim().is_empty() {
            continue;
        }
        summaries.push(TemplateUsageSummary {
            id: usage.template_id,
            last_used_at: usage.last_used_at,
        });
    }

    summaries.sort_by(|a, b| b.last_used_at.cmp(&a.last_used_at));
    summaries
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn temp_dir() -> PathBuf {
        std::env::temp_dir().join(format!("rs-usage-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn record_values_dedupes_and_orders_by_recency() {
        let dir = temp_dir();
        let mut values = Map::new();
        values.insert("name".into(), json!("A"));
        record_values(&dir, "tpl", &values).unwrap();

        values.insert("name".into(), json!("B"));
        record_values(&dir, "tpl", &values).unwrap();

        values.insert("name".into(), json!("A"));
        record_values(&dir, "tpl", &values).unwrap();

        let usage = read_usage(&dir, "tpl");
        let list = usage.variables.get("name").unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].value, json!("A"));
        assert_eq!(list[1].value, json!("B"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn record_values_caps_at_five_and_skips_empty_values() {
        let dir = temp_dir();
        let mut values = Map::new();
        for i in 0..8 {
            values.insert("name".into(), json!(format!("value-{i}")));
            record_values(&dir, "tpl", &values).unwrap();
        }
        values.insert("name".into(), json!(""));
        values.insert("table".into(), json!([{"a": 1}]));
        record_values(&dir, "tpl", &values).unwrap();

        let usage = read_usage(&dir, "tpl");
        let list = usage.variables.get("name").unwrap();
        assert_eq!(list.len(), MAX_RECENT_VALUES);
        assert_eq!(list[0].value, json!("value-7"));
        assert!(usage.variables.get("table").is_none());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn touch_and_list_usage() {
        let dir = temp_dir();
        touch_template(&dir, "b").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        touch_template(&dir, "a").unwrap();

        let list = list_usage(&dir);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, "a");
        assert!(list[0].last_used_at.is_some());

        let _ = fs::remove_dir_all(&dir);
    }
}
