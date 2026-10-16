import { invoke } from "@tauri-apps/api/core";
import type {
  OpenedProject,
  PreviewPage,
  Project,
  TemplateInfo,
  TemplateManifest,
} from "./types";

export const listTemplates = () => invoke<TemplateInfo[]>("list_templates");

export const getManifest = (templateId: string) =>
  invoke<TemplateManifest>("get_manifest", { templateId });

export const newProject = (templateId: string) =>
  invoke<OpenedProject>("new_project", { templateId });

export const renderPreview = (templateDir: string, values: unknown) =>
  invoke<PreviewPage[]>("render_preview", { templateDir, values });

export const resolveValues = (templateDir: string, values: unknown) =>
  invoke<Record<string, unknown>>("resolve_values", { templateDir, values });

export const generatePdf = (
  templateDir: string,
  values: unknown,
  outPath: string,
) => invoke<string>("generate_pdf", { templateDir, values, outPath });

export const printReport = (templateDir: string, values: unknown) =>
  invoke<void>("print_report", { templateDir, values });

export const installTemplate = (zipPath: string) =>
  invoke<TemplateInfo>("install_template", { zipPath });

export const exportTemplate = (templateId: string, outPath: string) =>
  invoke<void>("export_template", { templateId, outPath });

export const saveProject = (
  path: string,
  project: Project,
  embedTemplate: boolean,
) => invoke<void>("save_project", { path, project, embedTemplate });

export const openProject = (path: string) =>
  invoke<OpenedProject>("open_project", { path });
