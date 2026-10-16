//! 报告工坊模板渲染核心：模型、渲染引擎、模板包管理。
//! 与 Tauri 完全解耦，可被桌面应用、CLI、开发服务器等复用。

pub mod model;
pub mod render;
pub mod templates;
