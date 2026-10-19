# 更新日志

## 0.2.1

- feat: 新增模板变量校验钩子，支持 Worker 隔离执行、输入即时提示、导出/打印前拦截
- feat: 新增统一校验结果 Dialog，错误阻止导出/打印，警告允许确认后继续
- feat: 字段增加错误/警告图标与 Tooltip，支持多字段关联提示
- feat: 移除顶部校验汇总条，校验信息改为字段状态、Tooltip 和 Dialog 展示
- feat: 模板开发服务器支持校验钩子，并新增“校验导出”按钮
- feat: 新建项目模板列表按最近使用时间排序
- feat: 每个模板记录变量最近使用值，输入框支持最近值候选菜单
- feat: 最近值菜单支持聚焦显示、输入过滤、鼠标点击和键盘选择
- feat: 新增 GeneralTooltip03、GeneralNotification06、RecentValueMenu 组件
- feat: PDF 生成成功改为 Notification，提供“打开文件”和“打开文件夹”操作
- feat: 移除 Toast，统一使用 Notification 展示操作结果
- feat: 安装时关联 .rproj 工程文件，支持双击打开并显示独立文件图标
- fix: 修复 opener 权限 scope 未配置导致无法打开 PDF 路径
- fix: 修复原生 date input 输入不存在的日期时 value 为空、无法触发校验的问题
- chore: 新增模板使用记录模块，数据保存到用户数据目录
- chore: 示例模板同步更新：竣工结算审核报告 1.0.1、开竣工报告 1.0.2

## 0.2.0

- feat: PDF 输出改为按 manifest 页面逐页设置纸张尺寸/方向，并合并为单个 PDF
- feat: 修复 PDF 输出纸张为 Letter、页边距不一致的问题，统一按模板页面尺寸输出 A4
- feat: PDF 输出支持 flow 页面自动续页和纵向/横向页面混排
- feat: “打印…”改为使用与“输出为 PDF”相同的合并 PDF，并调起系统打印对话框
- feat: 新增单页打印渲染和临时 PDF 协议路由，修复打印文档模板 CSS 不加载的问题
- feat: report-dev 新增 `--resources` 参数和侧边栏资源目录配置
- feat: report-dev 支持 image/images 变量直接上传图片，并提供工程资源访问
- feat: 图片变量编辑器重做：单图放大、多图堆叠预览、缩略图列表、拖拽排序、删除和放大查看
- feat: 新增 ButtonGroup、ImageGalleryDialog、ImageLightbox 组件
- feat: 图片变量新增“添加/重置”操作，重置可恢复模板默认值
- feat: 移除 PDF 作为图片变量上传类型，仅保留图片格式
- fix: 修复 PDF 单页打印路由 base href 错误导致模板样式失效的问题
- fix: 修复 report-dev 图片变量无法保存为数组的问题
- chore: 新增 lopdf 依赖用于合并逐页 PDF
- feat: 新增完整“关于报告工坊”弹窗，展示版本、环境、模板、数据目录和诊断信息
- feat: 关于弹窗支持打开 GitHub、文档、Releases、问题反馈及模板目录
- feat: 导出 PDF 成功后自动用系统默认程序打开
- feat: 软件更新与模板更新改为一次性并发检查，并汇总到统一更新对话框
- feat: 更新对话框支持直接安装应用更新和同步模板更新
- feat: 新增 opener 插件和 `get_app_info` 命令
- fix: 关于弹窗改用 `src-tauri/icons` 中的应用图标，并移除静态图标依赖

## 0.1.3

- feat: 重新设计新建工程弹窗，支持模板列表、目录选择与固定文件名
- feat: 新建工程时检测同名文件，需勾选“覆盖存在文件”后才能覆盖
- feat: 新增 Input、Textarea、Label、Checkbox 等基础组件
- feat: 预览字段与示例值高亮改由渲染层统一注入，report-dev 同步支持
- feat: 文件菜单新增“关闭项目”，视图菜单移除“返回欢迎”
- feat: 窗口标题显示项目名，未保存更改显示星号，过长自动省略

## 0.1.2

feat:自动更新，优先使用国内CDN加速，兜底使用Github Release

## 0.1.1

feat:抽取report-core渲染库，增加report-dev模板开发服务器与协议挂载
feat:变量支持文件
feat:自动检查更新


## 0.1.0
- 初始版本
