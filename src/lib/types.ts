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
  hooks?: Record<string, string>;
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

export interface PreviewPage {
  file: string;
  html: string;
  thumbHtml: string;
  size: string;
  orientation: string;
  margin: string;
  mode: "fixed" | "flow";
}
