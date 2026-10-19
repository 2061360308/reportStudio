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
    /// image/images：允许的扩展名（如 ["png","jpg"]）
    #[serde(default)]
    pub accept: Option<Vec<String>>,
    /// images：最多张数
    #[serde(default)]
    pub max: Option<i64>,
}

/// 一页的几何信息（manifest 是唯一来源，后端据此生成 @page + CSS 变量）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageDef {
    /// 相对模板目录的页面片段文件路径，如 "pages/p01.html"
    pub file: String,
    /// 页面类型："fixed"（默认，固定一页）| "flow"（内容超出自动续页）
    #[serde(default)]
    pub mode: Option<String>,
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

impl PageDef {
    /// 是否为流式页：内容超出单页时由排版引擎自动续页。
    pub fn is_flow(&self) -> bool {
        matches!(self.mode.as_deref(), Some("flow") | Some("FLOW") | Some("Flow"))
    }
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
    /// 远端分发时要求的最低应用版本；为空表示不限制。
    #[serde(default)]
    pub min_app_version: Option<String>,
    /// 资源目录（相对模板根），默认 "assets"
    #[serde(default)]
    pub assets_path: Option<String>,
    /// 页面目录（相对模板根），默认 "pages"
    #[serde(default)]
    pub pages_path: Option<String>,
    #[serde(default)]
    pub variables: Vec<VariableDef>,
    /// 多页模板的页面清单（按 order 排序）
    #[serde(default)]
    pub pages: Vec<PageDef>,
    /// 目录（大纲）
    #[serde(default)]
    pub toc: Vec<TocEntry>,
    /// 生命周期钩子；`validate` 为变量校验脚本路径（相对模板根目录）。
    #[serde(default)]
    pub hooks: Map<String, Value>,
}

impl TemplateManifest {
    pub fn assets_path(&self) -> String {
        self.assets_path
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "assets".into())
    }
    pub fn pages_path(&self) -> String {
        self.pages_path
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "pages".into())
    }
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
    /// 挂载会话 id（协议资源路由 / 添加资源用）
    pub project_id: String,
}

/// 预览页：渲染后页面 URL + 尺寸信息（供前端 iframe 逐页渲染）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewPage {
    pub file: String,
    /// 补全后的完整 HTML（前端以 srcdoc 注入）
    pub html: String,
    /// 缩略图变体（不收集字段标记）
    pub thumb_html: String,
    pub size: String,
    pub orientation: String,
    pub margin: String,
    /// "fixed" | "flow"
    pub mode: String,
}
