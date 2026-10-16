use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnDef {
    pub key: String,
    #[serde(default)]
    pub alias: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(default)]
    pub expression: Option<String>,
    #[serde(default)]
    pub options: Option<Vec<SelectOption>>,
    #[serde(default)]
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableDef {
    pub key: String,
    #[serde(default)]
    pub alias: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<Value>,
    /// 预览占位值：字段为空时，预览渲染用它代替（不写入工程数据）
    #[serde(default)]
    pub sample: Option<Value>,
    #[serde(default)]
    pub options: Option<Vec<SelectOption>>,
    #[serde(default)]
    pub columns: Option<Vec<ColumnDef>>,
    #[serde(default)]
    pub expression: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub help: Option<String>,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub order: Option<i64>,
    #[serde(default)]
    pub readonly: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputDef {
    #[serde(default)]
    pub page_size: Option<String>,
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default)]
    pub print_backgrounds: Option<bool>,
    #[serde(default)]
    pub header_footer: Option<bool>,
}

/// 一页的几何信息（manifest 是唯一来源，后端据此生成 @page + CSS 变量）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageDef {
    /// 相对模板目录的页面片段文件路径，如 "pages/p01.html"
    pub file: String,
    /// 排序号
    #[serde(default)]
    pub order: Option<i64>,
    /// 纸张尺寸，如 "A4"
    #[serde(default)]
    pub size: Option<String>,
    /// 方向："portrait" | "landscape"
    #[serde(default)]
    pub orientation: Option<String>,
    /// @page 页边距，如 "0"
    #[serde(default)]
    pub margin: Option<String>,
    /// 正文内容四边留白（.paper padding），mm 值字符串
    #[serde(default)]
    pub mt: Option<String>,
    #[serde(default)]
    pub mr: Option<String>,
    #[serde(default)]
    pub mb: Option<String>,
    #[serde(default)]
    pub ml: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TocEntry {
    pub title: String,
    /// 目标页码（1 基，对应 `pages` 的顺序）
    pub page: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateManifest {
    pub schema_version: u32,
    pub id: String,
    pub version: String,
    #[serde(default)]
    pub content_hash: String,
    pub name: String,
    pub entry: String,
    #[serde(default)]
    pub variables: Vec<VariableDef>,
    /// 多页模板的页面清单（按 order 排序）
    #[serde(default)]
    pub pages: Vec<PageDef>,
    /// 目录（大纲）
    #[serde(default)]
    pub toc: Vec<TocEntry>,
    #[serde(default)]
    pub hooks: Map<String, Value>,
    #[serde(default)]
    pub output: OutputDef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateInfo {
    pub id: String,
    pub version: String,
    pub name: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateRef {
    pub id: String,
    pub version: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMeta {
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub schema_version: u32,
    pub meta: ProjectMeta,
    pub template_ref: TemplateRef,
    pub values: Map<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedded_template: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedProject {
    pub project: Project,
    pub manifest: TemplateManifest,
    /// 实际使用的模板目录（内嵌快照优先，否则已安装模板目录）
    pub template_dir: String,
}

/// 预览页：渲染后页面 URL + 尺寸信息（供前端 iframe 逐页渲染）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewPage {
    pub file: String,
    pub url: String,
    pub size: String,
    pub orientation: String,
    pub margin: String,
}
