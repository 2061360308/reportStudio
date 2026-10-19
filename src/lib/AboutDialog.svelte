<script lang="ts">
  import { onMount } from "svelte";
  import { getIdentifier, getTauriVersion } from "@tauri-apps/api/app";
  import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
  import BookOpen from "@lucide/svelte/icons/book-open";
  import Bug from "@lucide/svelte/icons/bug";
  import Check from "@lucide/svelte/icons/check";
  import Copy from "@lucide/svelte/icons/copy";
  import Download from "@lucide/svelte/icons/download";
  import FolderOpen from "@lucide/svelte/icons/folder-open";
  import GitBranch from "@lucide/svelte/icons/git-branch";
  import Globe from "@lucide/svelte/icons/globe";
  import Info from "@lucide/svelte/icons/info";
  import Package from "@lucide/svelte/icons/package";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import ShieldCheck from "@lucide/svelte/icons/shield-check";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import appIcon from "../../src-tauri/icons/icon.png";
  import * as api from "./api";
  import type { AppInfo, TemplateInfo } from "./types";
  import { Button } from "$lib/components/elements/button";
  import { GeneralDialog17 } from "$lib/components/elements/dialog";

  const REPO = "https://github.com/2061360308/reportStudio";
  const DOCS = "https://oilu.cn/reportStudio/";
  const RELEASES = `${REPO}/releases`;
  const ISSUES = `${REPO}/issues`;

  let {
    open = $bindable(false),
    version = "",
    hasUpdates = false,
    oncheckupdates,
  }: {
    open?: boolean;
    version?: string;
    hasUpdates?: boolean;
    oncheckupdates?: () => void;
  } = $props();

  let info = $state<AppInfo | null>(null);
  let templates = $state<TemplateInfo[]>([]);
  let tauriVersion = $state("");
  let identifier = $state("");
  let userAgent = $state("");
  let loading = $state(true);
  let error = $state("");
  let actionError = $state("");
  let copied = $state("");
  let copyTimer: ReturnType<typeof setTimeout> | undefined;

  const platform = $derived(
    info
      ? `${osLabel(info.os)} · ${archLabel(info.arch)}`
      : "—",
  );
  const mode = $derived(import.meta.env.PROD ? "正式版" : "开发模式");

  const links = [
    { label: "GitHub 仓库", url: REPO, icon: GitBranch },
    { label: "使用文档", url: DOCS, icon: BookOpen },
    { label: "最新版本", url: RELEASES, icon: Download },
    { label: "问题反馈", url: ISSUES, icon: Bug },
  ];

  const diagnosticText = $derived.by(() => {
    const lines = [
      `报告工坊${version ? ` v${version}` : ""}`,
      `Tauri ${tauriVersion || "—"}`,
      `系统 ${platform}`,
      `应用标识 ${identifier || "—"}`,
      `已安装模板 ${templates.length} 个`,
      ...templates.map(
        (item) => `- ${item.name} v${item.version} (${item.id})`,
      ),
      `模板目录 ${info?.templatesDir || "—"}`,
      `渲染目录 ${info?.rendersDir || "—"}`,
      `内嵌模板目录 ${info?.embeddedTemplatesDir || "—"}`,
      `User-Agent ${userAgent || "—"}`,
    ];
    return lines.join("\n");
  });

  onMount(load);

  async function load() {
    loading = true;
    error = "";
    userAgent = typeof navigator !== "undefined" ? navigator.userAgent : "";
    try {
      const [appInfo, list, tauri, id] = await Promise.all([
        api.getAppInfo(),
        api.listTemplates(),
        getTauriVersion(),
        getIdentifier(),
      ]);
      info = appInfo;
      templates = list;
      tauriVersion = tauri;
      identifier = id;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function osLabel(os: string): string {
    switch (os.toLowerCase()) {
      case "windows":
        return "Windows";
      case "linux":
        return "Linux";
      case "macos":
        return "macOS";
      default:
        return os || "未知系统";
    }
  }

  function archLabel(arch: string): string {
    switch (arch.toLowerCase()) {
      case "x86_64":
        return "x64";
      case "aarch64":
        return "ARM64";
      case "x86":
        return "x86";
      default:
        return arch || "未知架构";
    }
  }

  async function copyText(text: string, key: string) {
    actionError = "";
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      const area = document.createElement("textarea");
      area.value = text;
      area.style.position = "fixed";
      area.style.opacity = "0";
      document.body.appendChild(area);
      area.select();
      document.execCommand("copy");
      area.remove();
    }
    copied = key;
    clearTimeout(copyTimer);
    copyTimer = setTimeout(() => (copied = ""), 1600);
  }

  async function openLink(url: string) {
    actionError = "";
    try {
      await openUrl(url);
    } catch (e) {
      actionError = `打开链接失败：${e}`;
    }
  }

  async function openFolder(path: string) {
    if (!path) return;
    actionError = "";
    try {
      await revealItemInDir(path);
    } catch (e) {
      actionError = `打开目录失败：${e}`;
    }
  }

  function checkUpdates() {
    oncheckupdates?.();
    open = false;
  }
</script>

<GeneralDialog17
  bind:open
  title="关于报告工坊"
  description="工程报告生成工具"
  onclose={() => (open = false)}
  class="h-[min(90vh,720px)] max-w-2xl"
>
  {#snippet icon()}
    <img src={appIcon} alt="" class="h-9 w-9 rounded-md object-contain" />
  {/snippet}

  <div class="flex min-h-0 flex-1 flex-col">
    <div class="min-h-0 flex-1 space-y-5 overflow-y-auto px-6 py-5">
      <div class="flex items-start gap-3">
        <div class="min-w-0 flex-1">
          <div class="flex flex-wrap items-center gap-2">
            <h3 class="text-base font-semibold">报告工坊</h3>
            <span
              class="rounded-full border border-border px-2 py-0.5 text-xs text-muted-foreground"
            >
              {version ? `v${version}` : "版本未知"}
            </span>
            <span
              class="rounded-full bg-muted px-2 py-0.5 text-xs text-muted-foreground"
            >
              {mode}
            </span>
            {#if hasUpdates}
              <span
                class="rounded-full bg-primary/10 px-2 py-0.5 text-xs font-medium text-primary"
              >
                有可用更新
              </span>
            {/if}
          </div>
          <p class="mt-1 text-xs text-muted-foreground">
            模板化工程报告编辑与输出工具
          </p>
        </div>
      </div>

      {#if loading}
        <p class="text-xs text-muted-foreground">正在读取运行信息…</p>
      {:else if error}
        <p class="flex items-start gap-2 rounded-md border border-destructive/30 bg-destructive/5 p-3 text-xs text-destructive">
          <TriangleAlert
            class="mt-0.5 h-3.5 w-3.5 shrink-0"
            aria-hidden="true"
          />
          {error}
        </p>
      {:else}
        <div class="grid grid-cols-2 gap-3">
          <div class="rounded-md border border-border bg-muted/30 p-3">
            <p class="text-xs text-muted-foreground">应用版本</p>
            <p class="mono mt-1 truncate text-sm font-medium">
              {version ? `v${version}` : "—"}
            </p>
          </div>
          <div class="rounded-md border border-border bg-muted/30 p-3">
            <p class="text-xs text-muted-foreground">Tauri</p>
            <p class="mono mt-1 truncate text-sm font-medium">
              {tauriVersion || "—"}
            </p>
          </div>
          <div class="rounded-md border border-border bg-muted/30 p-3">
            <p class="text-xs text-muted-foreground">运行环境</p>
            <p class="mt-1 truncate text-sm font-medium">{platform}</p>
          </div>
          <div class="rounded-md border border-border bg-muted/30 p-3">
            <p class="text-xs text-muted-foreground">已安装模板</p>
            <p class="mt-1 truncate text-sm font-medium">
              {templates.length} 个
            </p>
          </div>
        </div>
      {/if}

      <div
        class="flex items-center justify-between gap-3 rounded-md border border-border bg-muted/30 p-3"
      >
        <div class="flex min-w-0 items-center gap-2">
          <RefreshCw
            class="h-4 w-4 shrink-0 text-muted-foreground"
            aria-hidden="true"
          />
          <div class="min-w-0">
            <p class="text-sm font-medium">更新</p>
            <p class="truncate text-xs text-muted-foreground">
              {hasUpdates
                ? "发现可用的应用或模板更新"
                : "检查应用与模板的新版本"}
            </p>
          </div>
        </div>
        <Button
          type="button"
          variant={hasUpdates ? "default" : "outline"}
          size="sm"
          class="shrink-0"
          onclick={checkUpdates}
        >
          {hasUpdates ? "查看更新" : "检查更新"}
        </Button>
      </div>

      <section class="space-y-2">
        <div class="flex items-center justify-between gap-2">
          <div class="flex items-center gap-2">
            <Package
              class="h-4 w-4 text-muted-foreground"
              aria-hidden="true"
            />
            <h3 class="text-sm font-semibold">已安装模板</h3>
          </div>
          <span class="text-xs text-muted-foreground">
            {templates.length} 个
          </span>
        </div>
        {#if templates.length}
          <ul
            class="max-h-36 divide-y divide-border overflow-y-auto rounded-md border border-border"
          >
            {#each templates as item (item.id)}
              <li class="flex items-center justify-between gap-3 px-3 py-2">
                <div class="min-w-0">
                  <p class="truncate text-sm font-medium">{item.name}</p>
                  <p class="mono truncate text-xs text-muted-foreground">
                    {item.id}
                  </p>
                </div>
                <span class="mono shrink-0 text-xs text-muted-foreground">
                  v{item.version}
                </span>
              </li>
            {/each}
          </ul>
        {:else if !loading}
          <p class="text-xs text-muted-foreground">暂无已安装模板。</p>
        {/if}
      </section>

      <section class="space-y-2">
        <div class="flex items-center gap-2">
          <Globe class="h-4 w-4 text-muted-foreground" aria-hidden="true" />
          <h3 class="text-sm font-semibold">链接</h3>
        </div>
        <div class="grid grid-cols-2 gap-2">
          {#each links as link (link.url)}
            {@const Icon = link.icon}
            <Button
              type="button"
              variant="outline"
              size="sm"
              class="justify-start"
              onclick={() => openLink(link.url)}
            >
              <Icon class="h-4 w-4" aria-hidden="true" />
              {link.label}
            </Button>
          {/each}
        </div>
      </section>

      <details class="rounded-md border border-border">
        <summary
          class="flex cursor-pointer list-none items-center gap-2 px-3 py-2 text-sm font-medium"
        >
          <Info class="h-4 w-4 text-muted-foreground" aria-hidden="true" />
          诊断信息
        </summary>
        <div class="space-y-2 border-t border-border px-3 py-3">
          <div class="space-y-1">
            <p class="text-xs text-muted-foreground">模板目录</p>
            <div class="flex items-center gap-2">
              <span
                class="mono min-w-0 flex-1 truncate text-xs text-foreground"
                title={info?.templatesDir}
              >
                {info?.templatesDir || "—"}
              </span>
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                onclick={() =>
                  copyText(info?.templatesDir ?? "", "templatesDir")}
                aria-label="复制模板目录"
              >
                {#if copied === "templatesDir"}
                  <Check class="h-3.5 w-3.5" aria-hidden="true" />
                {:else}
                  <Copy class="h-3.5 w-3.5" aria-hidden="true" />
                {/if}
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                onclick={() => openFolder(info?.templatesDir ?? "")}
                aria-label="打开模板目录"
              >
                <FolderOpen class="h-3.5 w-3.5" aria-hidden="true" />
              </Button>
            </div>
          </div>

          <div class="space-y-1">
            <p class="text-xs text-muted-foreground">渲染目录</p>
            <div class="flex items-center gap-2">
              <span
                class="mono min-w-0 flex-1 truncate text-xs text-foreground"
                title={info?.rendersDir}
              >
                {info?.rendersDir || "—"}
              </span>
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                onclick={() => copyText(info?.rendersDir ?? "", "rendersDir")}
                aria-label="复制渲染目录"
              >
                {#if copied === "rendersDir"}
                  <Check class="h-3.5 w-3.5" aria-hidden="true" />
                {:else}
                  <Copy class="h-3.5 w-3.5" aria-hidden="true" />
                {/if}
              </Button>
            </div>
          </div>

          <div class="space-y-1">
            <p class="text-xs text-muted-foreground">应用标识</p>
            <p class="mono break-all text-xs text-foreground">
              {identifier || "—"}
            </p>
          </div>

          <div class="space-y-1">
            <p class="text-xs text-muted-foreground">User-Agent</p>
            <p class="break-all text-xs text-foreground">
              {userAgent || "—"}
            </p>
          </div>

          <div class="flex items-center gap-2 pt-1">
            <Button
              type="button"
              variant="outline"
              size="sm"
              onclick={() => copyText(diagnosticText, "diagnostics")}
            >
              {#if copied === "diagnostics"}
                <Check class="h-4 w-4" aria-hidden="true" />
                已复制
              {:else}
                <Copy class="h-4 w-4" aria-hidden="true" />
                复制诊断信息
              {/if}
            </Button>
          </div>
        </div>
      </details>

      {#if actionError}
        <p class="text-xs text-destructive">{actionError}</p>
      {/if}
    </div>

    <div
      class="flex flex-wrap items-center justify-between gap-3 border-t border-border bg-muted/40 px-6 py-4"
    >
      <p class="flex items-center gap-1.5 text-xs text-muted-foreground">
        <ShieldCheck class="h-3.5 w-3.5" aria-hidden="true" />
        MIT License · © 2026 报告工坊
      </p>
      <div class="flex items-center gap-2">
        <Button
          type="button"
          variant="outline"
          size="sm"
          onclick={() => copyText(diagnosticText, "diagnostics")}
        >
          {#if copied === "diagnostics"}
            <Check class="h-4 w-4" aria-hidden="true" />
            已复制
          {:else}
            <Copy class="h-4 w-4" aria-hidden="true" />
            复制诊断信息
          {/if}
        </Button>
        <Button type="button" size="sm" onclick={() => (open = false)}>
          关闭
        </Button>
      </div>
    </div>
  </div>
</GeneralDialog17>
