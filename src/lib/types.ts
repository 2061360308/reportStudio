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
}

export interface OutputDef {
  pageSize?: string;
  filename?: string;
  printBackgrounds?: boolean;
  headerFooter?: boolean;
}

export interface TemplateManifest {
  schemaVersion: number;
  id: string;
  version: string;
  contentHash: string;
  name: string;
  entry: string;
  variables: VariableDef[];
  pages?: PageDef[];
  toc?: TocEntry[];
  hooks?: Record<string, string>;
  output?: OutputDef;
}

export interface PageDef {
  file: string;
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
}

export interface RecentItem {
  name: string;
  path: string;
  template?: string;
}

export interface PreviewPage {
  file: string;
  url: string;
  size: string;
  orientation: string;
  margin: string;
}
