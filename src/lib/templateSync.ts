import { Channel } from "@tauri-apps/api/core";
import * as api from "./api";
import type { TemplateInfo, TemplateSyncEvent, TemplateUpdateInfo } from "./types";

export function findTemplateUpdates(): Promise<TemplateUpdateInfo[]> {
  return api.checkTemplateUpdates();
}

export function syncAllTemplates(
  onEvent: (event: TemplateSyncEvent) => void,
): Promise<TemplateInfo[]> {
  const channel = new Channel<TemplateSyncEvent>();
  channel.onmessage = onEvent;
  return api.syncTemplates(channel);
}
