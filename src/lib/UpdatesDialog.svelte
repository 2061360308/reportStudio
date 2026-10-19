<script lang="ts">
  import type { Update } from "@tauri-apps/plugin-updater";
  import Download from "@lucide/svelte/icons/download";
  import LoaderCircle from "@lucide/svelte/icons/loader-circle";
  import Package from "@lucide/svelte/icons/package";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import { Button } from "$lib/components/elements/button";
  import { GeneralDialog17 } from "$lib/components/elements/dialog";
  import type { TemplateUpdateInfo } from "./types";

  let {
    open = $bindable(false),
    checking = false,
    appUpdate = null,
    appInstalling = false,
    appPercent = 0,
    appError = "",
    templateUpdates = [],
    templateSyncing = false,
    templatePercent = 0,
    templateError = "",
    oninstall,
    onskip,
    onsync,
    onrecheck,
    onclose,
  }: {
    open?: boolean;
    checking?: boolean;
    appUpdate?: Update | null;
    appInstalling?: boolean;
    appPercent?: number;
    appError?: string;
    templateUpdates?: TemplateUpdateInfo[];
    templateSyncing?: boolean;
    templatePercent?: number;
    templateError?: string;
    oninstall: () => void;
    onskip: () => void;
    onsync: () => void;
    onrecheck: () => void;
    onclose: () => void;
  } = $props();

  const busy = $derived(checking || appInstalling || templateSyncing);
  const hasUpdates = $derived(!!appUpdate || templateUpdates.length > 0);
</script>

<GeneralDialog17
  bind:open
  title="检查更新"
  description="应用与模板更新会在这里统一汇总。"
  {onclose}
  dismissible={!busy}
  class="h-[min(90vh,720px)] max-w-2xl"
>
  <div class="flex min-h-0 flex-1 flex-col">
    <div class="min-h-0 flex-1 space-y-5 overflow-y-auto px-6 py-5">
      {#if checking}
        <div
          class="flex items-center gap-2 rounded-md border border-border bg-muted/40 px-3 py-2 text-sm text-muted-foreground"
        >
          <LoaderCircle class="h-4 w-4 animate-spin" aria-hidden="true" />
          正在检查应用和模板更新…
        </div>
      {/if}

      <section class="space-y-3">
        <div class="flex items-start justify-between gap-3">
          <div>
            <h3 class="text-sm font-semibold">应用更新</h3>
            <p class="text-xs text-muted-foreground">
              报告工坊桌面应用
            </p>
          </div>
          {#if appUpdate}
            <span class="rounded-full bg-primary/10 px-2 py-0.5 text-xs font-medium text-primary">
              可更新
            </span>
          {:else if !checking && !appError}
            <span class="text-xs text-muted-foreground">已是最新</span>
          {/if}
        </div>

        {#if appUpdate}
          <div class="rounded-md border border-border bg-muted/30 p-4">
            <div class="flex flex-wrap items-baseline justify-between gap-2">
              <span class="text-sm font-semibold">
                新版本 v{appUpdate.version}
              </span>
              {#if appInstalling}
                <span class="text-xs text-muted-foreground">
                  {appPercent >= 100 ? "正在安装…" : `正在下载 ${appPercent}%`}
                </span>
              {/if}
            </div>
            <pre
              class="mt-2 max-h-28 overflow-y-auto whitespace-pre-wrap break-words font-sans text-xs leading-6 text-muted-foreground">{appUpdate.body ||
                "更新说明暂无"}</pre>

            {#if appInstalling}
              <div class="mt-3 h-1.5 overflow-hidden rounded-full bg-muted">
                <div
                  class="h-full rounded-full bg-primary transition-[width]"
                  style={`width:${appPercent}%`}
                ></div>
              </div>
            {:else}
              <div class="mt-3 flex items-center gap-2">
                <Button
                  size="sm"
                  onclick={oninstall}
                  disabled={templateSyncing}
                >
                  <Download class="h-4 w-4" aria-hidden="true" />
                  安装更新
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  onclick={onskip}
                  disabled={templateSyncing}
                >
                  跳过此版本
                </Button>
              </div>
            {/if}

            {#if appError}
              <p class="mt-3 flex items-start gap-2 text-xs text-destructive">
                <TriangleAlert
                  class="mt-0.5 h-3.5 w-3.5 shrink-0"
                  aria-hidden="true"
                />
                {appError}
              </p>
            {/if}
          </div>
        {:else if appError}
          <p class="text-xs text-destructive">{appError}</p>
        {:else if !checking}
          <p class="text-xs text-muted-foreground">
            当前应用已是最新版本。
          </p>
        {/if}
      </section>

      <section class="space-y-3">
        <div class="flex items-start justify-between gap-3">
          <div>
            <h3 class="text-sm font-semibold">模板更新</h3>
            <p class="text-xs text-muted-foreground">
              已安装模板的版本更新
            </p>
          </div>
          {#if templateUpdates.length}
            <span class="rounded-full bg-primary/10 px-2 py-0.5 text-xs font-medium text-primary">
              {templateUpdates.length} 个可更新
            </span>
          {:else if !checking && !templateError}
            <span class="text-xs text-muted-foreground">已是最新</span>
          {/if}
        </div>

        {#if templateUpdates.length}
          <div class="space-y-2">
            {#each templateUpdates as item (item.id)}
              <div class="rounded-md border border-border bg-muted/30 p-3">
                <div class="flex flex-wrap items-baseline justify-between gap-2">
                  <div class="min-w-0">
                    <p class="truncate text-sm font-medium">{item.name}</p>
                    <p class="mono truncate text-xs text-muted-foreground">
                      {item.id}
                    </p>
                  </div>
                  <span class="mono shrink-0 text-xs text-muted-foreground">
                    {item.currentVersion
                      ? `v${item.currentVersion} → v${item.version}`
                      : `未安装 → v${item.version}`}
                  </span>
                </div>
                <pre
                  class="mt-2 max-h-24 overflow-y-auto whitespace-pre-wrap break-words font-sans text-xs leading-6 text-muted-foreground">{item.notes ||
                    "更新说明暂无"}</pre>
              </div>
            {/each}
          </div>

          {#if templateSyncing}
            <div class="space-y-1">
              <div class="h-1.5 overflow-hidden rounded-full bg-muted">
                <div
                  class="h-full rounded-full bg-primary transition-[width]"
                  style={`width:${templatePercent}%`}
                ></div>
              </div>
              <p class="text-xs text-muted-foreground">
                正在同步模板… {templatePercent}%
              </p>
            </div>
          {:else}
            <Button
              size="sm"
              onclick={onsync}
              disabled={appInstalling}
            >
              <Package class="h-4 w-4" aria-hidden="true" />
              全部更新
            </Button>
          {/if}

          {#if templateError}
            <p class="text-xs text-destructive">{templateError}</p>
          {/if}
        {:else if templateError}
          <p class="text-xs text-destructive">{templateError}</p>
        {:else if !checking}
          <p class="text-xs text-muted-foreground">
            已安装模板均为最新版本。
          </p>
        {/if}
      </section>

      {#if !checking && !hasUpdates && !appError && !templateError}
        <div
          class="rounded-md border border-border bg-muted/40 px-4 py-3 text-sm text-muted-foreground"
        >
          应用和模板均已是最新版本。
        </div>
      {:else if appUpdate && templateUpdates.length}
        <p class="text-xs text-muted-foreground">
          建议先更新应用，重启后再同步模板。
        </p>
      {/if}
    </div>

    <div
      class="flex items-center justify-between gap-3 border-t border-border bg-muted/40 px-6 py-4"
    >
      <Button
        type="button"
        variant="outline"
        size="sm"
        onclick={onrecheck}
        disabled={busy}
      >
        <RefreshCw class="h-4 w-4" aria-hidden="true" />
        重新检查
      </Button>
      <Button type="button" onclick={onclose} disabled={busy}>
        关闭
      </Button>
    </div>
  </div>
</GeneralDialog17>
