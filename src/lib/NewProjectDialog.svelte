<script lang="ts">
  import { onMount } from "svelte";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { join } from "@tauri-apps/api/path";
  import FilePlus from "@lucide/svelte/icons/file-plus";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import * as api from "./api";
  import type {
    OpenedProject,
    TemplateInfo,
    TemplateUsageSummary,
  } from "./types";
  import { GeneralDialog17 } from "$lib/components/elements/dialog";
  import { Button } from "$lib/components/elements/button";
  import { GeneralCheckbox01 } from "$lib/components/elements/checkbox";
  import { Input } from "$lib/components/elements/input";
  import { Label } from "$lib/components/elements/label";
  import { cn } from "$lib/utils.js";

  let {
    oncreated,
    oncancel,
    onsync,
  }: {
    oncreated: (o: OpenedProject, path: string) => void;
    oncancel: () => void;
    onsync?: () => Promise<void>;
  } = $props();

  let dialogOpen = $state(true);
  let templates = $state<TemplateInfo[]>([]);
  let selected = $state("");
  let saveDir = $state("");
  let projectName = $state("默认项目");
  let busy = $state(false);
  let syncBusy = $state(false);
  let loading = $state(true);
  let error = $state("");
  let duplicatePath = $state("");
  let overwriteConfirmed = $state(false);
  let checkingPath = $state(false);
  let checkSeq = 0;

  const chosen = $derived(templates.find((t) => t.id === selected) ?? null);
  const safeProjectName = $derived(sanitize(projectName) || "默认项目");
  const templateLabel = $derived(
    chosen ? sanitize(chosen.name) || sanitize(chosen.id) || "模板" : "",
  );
  const fileName = $derived(
    chosen ? `${safeProjectName}-${templateLabel}.rproj` : "",
  );
  const canCreate = $derived(
    !busy &&
      !checkingPath &&
      !!selected &&
      !!saveDir &&
      !!projectName.trim() &&
      !!fileName &&
      (!duplicatePath || overwriteConfirmed),
  );

  $effect(() => {
    const dir = saveDir;
    const name = fileName;
    const seq = ++checkSeq;
    let cancelled = false;

    duplicatePath = "";
    overwriteConfirmed = false;

    if (!dir || !name) {
      checkingPath = false;
      return;
    }

    checkingPath = true;
    const timer = setTimeout(async () => {
      try {
        const path = await join(dir, name);
        const exists = await api.projectFileExists(path);
        if (cancelled || seq !== checkSeq) return;
        duplicatePath = exists ? path : "";
      } catch (e) {
        if (cancelled || seq !== checkSeq) return;
        error = String(e);
      } finally {
        if (!cancelled && seq === checkSeq) checkingPath = false;
      }
    }, 250);

    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  });

  onMount(load);

  function sanitize(value: string): string {
    return value
      .replace(/[\\/:*?"<>|]/g, "-")
      .replace(/[\u0000-\u001f]/g, "")
      .replace(/\s+/g, " ")
      .trim()
      .replace(/[. ]+$/g, "");
  }

  async function load() {
    error = "";
    loading = true;
    try {
      const list = await api.listTemplates();
      let usage: TemplateUsageSummary[] = [];
      try {
        usage = await api.listTemplateUsage();
      } catch {
        /* 使用记录不可用时按名称排序 */
      }

      const lastUsed = new Map(
        usage.map((item) => [
          item.id,
          item.lastUsedAt ? Date.parse(item.lastUsedAt) || 0 : 0,
        ]),
      );

      templates = list.sort((a, b) => {
        const aTime = lastUsed.get(a.id) ?? 0;
        const bTime = lastUsed.get(b.id) ?? 0;
        if (aTime !== bTime) return bTime - aTime;
        return (
          a.name.localeCompare(b.name, "zh-CN") ||
          a.id.localeCompare(b.id)
        );
      });

      if (!templates.some((template) => template.id === selected)) {
        selected = templates[0]?.id ?? "";
      }
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function retrySync() {
    if (!onsync) return;
    syncBusy = true;
    error = "";
    try {
      await onsync();
    } catch (e) {
      error = String(e);
    } finally {
      syncBusy = false;
    }
  }

  async function pickPath() {
    error = "";
    try {
      const out = await openDialog({
        directory: true,
        multiple: false,
        defaultPath: saveDir || undefined,
      });
      if (typeof out === "string") saveDir = out;
    } catch (e) {
      error = String(e);
    }
  }

  async function start() {
    if (!canCreate) return;
    busy = true;
    error = "";
    try {
      const path = await join(saveDir, fileName);
      const exists = await api.projectFileExists(path);

      if (exists) {
        if (!overwriteConfirmed || duplicatePath !== path) {
          duplicatePath = path;
          overwriteConfirmed = false;
          return;
        }
      } else {
        duplicatePath = "";
        overwriteConfirmed = false;
      }

      const o = await api.newProject(selected);
      o.project.meta.name = projectName.trim();
      await api.saveProject(path, o.project, true, o.projectId, exists);
      oncreated(o, path);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function requestClose() {
    if (!busy) oncancel();
  }
</script>

<GeneralDialog17
  bind:open={dialogOpen}
  title="新建报告工程"
  description="选择模板并保存到本地，创建后即可编辑。"
  onclose={requestClose}
  dismissible={!busy}
  class="h-[min(90vh,640px)] max-w-xl"
>
  {#snippet icon()}
    <FilePlus class="h-4 w-4" aria-hidden="true" />
  {/snippet}

  <form
    class="flex min-h-0 flex-1 flex-col"
    onsubmit={(event) => {
      event.preventDefault();
      start();
    }}
  >
    <div class="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-6 py-5">
      <div class="space-y-2">
        <Label for="new-project-name">工程名称</Label>
        <Input
          id="new-project-name"
          bind:value={projectName}
          placeholder="默认项目"
          autofocus
        />
        <p class="text-xs text-muted-foreground">
          用于工程标题和保存文件名，可随时修改。
        </p>
      </div>

      <div class="flex min-h-36 flex-1 flex-col gap-2">
        <div class="flex items-center justify-between gap-3">
          <span class="text-sm font-medium leading-none">模板</span>
          {#if !loading && templates.length}
            <span class="text-xs text-muted-foreground">
              {templates.length} 个可用
            </span>
          {/if}
        </div>

        {#if loading}
          <div
            class="flex min-h-0 flex-1 items-center justify-center rounded-md border border-dashed border-border px-4 py-6 text-center text-sm text-muted-foreground"
          >
            正在读取模板…
          </div>
        {:else if templates.length === 0}
          <div
            class="flex min-h-0 flex-1 flex-col items-center justify-center rounded-md border border-dashed border-border px-4 py-6 text-center"
          >
            <p class="text-sm text-muted-foreground">
              暂无已安装模板。首次使用需要从更新源同步模板。
            </p>
            {#if onsync}
              <Button
                type="button"
                variant="outline"
                size="sm"
                class="mt-3"
                onclick={retrySync}
                disabled={syncBusy}
              >
                {syncBusy ? "正在同步…" : "同步模板"}
              </Button>
            {/if}
          </div>
        {:else}
          <div
            class="flex min-h-0 flex-1 flex-col overflow-hidden rounded-md border border-border"
          >
            <div
              class="min-h-0 flex-1 divide-y divide-border overflow-y-auto overscroll-contain"
            >
              {#each templates as t (t.id)}
                <button
                  type="button"
                  class={cn(
                    "flex w-full items-center gap-3 rounded-none px-3 py-2.5 text-left shadow-none transition-colors",
                    selected === t.id
                      ? "bg-primary/5"
                      : "bg-background hover:bg-muted/60",
                  )}
                  aria-pressed={selected === t.id}
                  onclick={() => (selected = t.id)}
                  ondblclick={start}
                >
                  <span
                    class={cn(
                      "flex size-4 shrink-0 items-center justify-center rounded-full border transition-colors",
                      selected === t.id
                        ? "border-primary bg-primary"
                        : "border-input bg-background",
                    )}
                    aria-hidden="true"
                  >
                    {#if selected === t.id}
                      <span class="size-1.5 rounded-full bg-primary-foreground"
                      ></span>
                    {/if}
                  </span>
                  <span class="flex min-w-0 flex-1 flex-col gap-0.5">
                    <span class="truncate text-sm font-medium">{t.name}</span>
                    <span class="mono truncate text-xs text-muted-foreground">
                      {t.id}
                    </span>
                  </span>
                  <span
                    class="shrink-0 rounded-full bg-muted px-2 py-0.5 text-[11px] font-medium text-muted-foreground"
                  >
                    v{t.version}
                  </span>
                </button>
              {/each}
            </div>
          </div>
        {/if}
      </div>

      <div class="space-y-2">
        <Label for="new-project-dir">保存位置</Label>
        <div class="flex gap-2">
          <Input
            id="new-project-dir"
            readonly
            value={saveDir}
            placeholder="未选择文件夹"
            title={saveDir}
            class="flex-1 text-xs text-muted-foreground"
          />
          <Button
            type="button"
            variant="outline"
            class="shrink-0"
            onclick={pickPath}
            disabled={busy}
          >
            <FolderOpen class="h-4 w-4" aria-hidden="true" />
            浏览…
          </Button>
        </div>
        {#if fileName}
          <p class="text-xs text-muted-foreground">
            文件名：
            <span class="mono break-all text-foreground">{fileName}</span>
          </p>
        {/if}
      </div>

      {#if error}
        <div
          class="flex gap-2 rounded-md border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive"
        >
          <TriangleAlert
            class="mt-0.5 h-4 w-4 shrink-0"
            aria-hidden="true"
          />
          <pre class="whitespace-pre-wrap break-words font-sans">{error}</pre>
        </div>
      {/if}
    </div>

    <div
      class="flex items-center justify-between gap-4 border-t border-border bg-muted/40 px-6 py-4"
    >
      <div class="min-w-0" title={duplicatePath}>
        {#if duplicatePath}
          <GeneralCheckbox01
            id="overwrite-existing-project"
            bind:checked={overwriteConfirmed}
            label="覆盖存在文件"
            disabled={busy}
          />
        {/if}
      </div>
      <div class="flex shrink-0 justify-end gap-2">
        <Button
          type="button"
          variant="outline"
          onclick={requestClose}
          disabled={busy}
        >
          取消
        </Button>
        <Button type="submit" disabled={!canCreate}>
          {busy ? "正在创建…" : "创建工程"}
        </Button>
      </div>
    </div>
  </form>
</GeneralDialog17>
