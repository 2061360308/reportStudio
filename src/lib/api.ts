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

export const renderPreview = (templateDir: string, values: unknown, projectId: string) =>
  invoke<PreviewPage[]>("render_preview", { templateDir, values, projectId });

export const resolveValues = (templateDir: string, values: unknown) =>
  invoke<Record<string, unknown>>("resolve_values", { templateDir, values });

export const addResource = (
  projectId: string,
  filename: string,
  dataUrl: string,
) => invoke<string>("add_resource", { projectId, filename, dataUrl });

export const removeResource = (projectId: string, name: string) =>
  invoke<void>("remove_resource", { projectId, name });

export const generatePdf = (
  templateDir: string,
  values: unknown,
  projectId: string,
  outPath: string,
) => invoke<string>("generate_pdf", { templateDir, values, projectId, outPath });

export const printReport = (
  templateDir: string,
  values: unknown,
  projectId: string,
) => invoke<void>("print_report", { templateDir, values, projectId });

export const installTemplate = (zipPath: string) =>
  invoke<TemplateInfo>("install_template", { zipPath });

export const exportTemplate = (templateId: string, outPath: string) =>
  invoke<void>("export_template", { templateId, outPath });

export const saveProject = (
  path: string,
  project: Project,
  embedTemplate: boolean,
  projectId: string,
) => invoke<void>("save_project", { path, project, embedTemplate, projectId });

export const openProject = (path: string) =>
  invoke<OpenedProject>("open_project", { path });
