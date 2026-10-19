export type VarType =
  | "string"
  | "text"
  | "number"
  | "integer"
  | "money"
  | "percent"
  | "date"
  | "datetime"
  | "bool"
  | "select"
  | "multiselect"
  | "table"
  | "image"
  | "images"
  | "computed";

export interface SelectOption {
  value: string;
  label: string;
}

export interface ColumnDef {
  key: string;
  alias?: string;
  label?: string;
  type: VarType;
  expression?: string;
  options?: SelectOption[];
  format?: string;
}

export interface VariableDef {
  key: string;
  alias?: string;
  label?: string;
  type: VarType;
  required?: boolean;
  default?: unknown;
  sample?: unknown;
  options?: SelectOption[];
  columns?: ColumnDef[];
  expression?: string;
  format?: string;
  unit?: string;
  placeholder?: string;
  help?: string;
  group?: string;
  order?: number;
  readonly?: boolean;
  /** image/images：允许的扩展名（如 ["png","jpg"]） */
  accept?: string[];
  /** images：最多张数 */
  max?: number;
}

export interface TemplateHooks {
  /** 变量校验钩子文件，相对模板根目录，例如 assets/validate.js */
  validate?: string;
  [key: string]: unknown;
}

export interface TemplateManifest {
  schemaVersion: number;
  id: string;
  version: string;
  contentHash: string;
  name: string;
  minAppVersion?: string;
  assetsPath?: string;
  pagesPath?: string;
  variables: VariableDef[];
  pages?: PageDef[];
  toc?: TocEntry[];
  hooks?: TemplateHooks;
}

export type ValidationStatus = "ok" | "warning" | "error";

export interface ValidationIssue {
  status: ValidationStatus;
  /** 受影响字段，多字段联动时全部刷新 */
  keys: string[];
  /** 主显示字段；不填时使用 keys[0] */
  key?: string;
  msg: string;
}

export interface HookContext {
  event: "change" | "preview" | "beforeExport";
  key?: string;
  value?: unknown;
  /** 已解析的完整变量值，含 computed */
  values: Record<string, unknown>;
  /** 原始 project.values */
  rawValues: Record<string, unknown>;
  variables: VariableDef[];
}

export interface PageDef {
  file: string;
  mode?: "fixed" | "flow";
  order?: number;
  size?: string;
  orientation?: string;
  margin?: string;
  mt?: string;
  mr?: string;
  mb?: string;
  ml?: string;
}

export interface TocEntry {
  title: string;
  page: number;
}

export interface TemplateInfo {
  id: string;
  version: string;
  name: string;
  contentHash: string;
}

export interface RecentValue {
  value: unknown;
  usedAt: string;
}

export interface TemplateUsage {
  schemaVersion: number;
  templateId: string;
  lastUsedAt?: string | null;
  variables: Record<string, RecentValue[]>;
}

export interface TemplateUsageSummary {
  id: string;
  lastUsedAt?: string | null;
}

export interface TemplateUpdateInfo {
  id: string;
  name: string;
  version: string;
  currentVersion?: string | null;
  notes: string;
  size?: number | null;
  minAppVersion?: string | null;
}

export interface TemplateSyncEvent {
  phase: "start" | "downloading" | "installing" | "installed" | "done";
  id?: string;
  current: number;
  total: number;
  percent?: number;
  message?: string;
}

export interface TemplateRef {
  id: string;
  version: string;
  contentHash: string;
}

export interface ProjectMeta {
  name: string;
  createdAt: string;
  updatedAt: string;
}

export interface Project {
  schemaVersion: number;
  meta: ProjectMeta;
  templateRef: TemplateRef;
  values: Record<string, unknown>;
}

export interface OpenedProject {
  project: Project;
  manifest: TemplateManifest;
  templateDir: string;
  projectId: string;
}

export interface RecentItem {
  name: string;
  path: string;
  template?: string;
}

export interface AppInfo {
  os: string;
  arch: string;
  templatesDir: string;
  rendersDir: string;
  embeddedTemplatesDir: string;
}

export interface PreviewPage {
  file: string;
  html: string;
  thumbHtml: string;
  size: string;
  orientation: string;
  margin: string;
  mode: "fixed" | "flow";
}
