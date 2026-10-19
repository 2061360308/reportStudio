<script lang="ts">
  import { onMount } from "svelte";
  import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
  import { getVersion } from "@tauri-apps/api/app";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import {
    openPath as openWithSystem,
    revealItemInDir,
  } from "@tauri-apps/plugin-opener";
  import type { Update } from "@tauri-apps/plugin-updater";
  import * as api from "$lib/api";
  import * as Menubar from "$lib/components/elements/menubar";
  import { GeneralNotification06 } from "$lib/components/elements/notification";
  import Start from "$lib/Start.svelte";
  import Editor from "$lib/Editor.svelte";
  import Preview from "$lib/Preview.svelte";
  import NewProjectDialog from "$lib/NewProjectDialog.svelte";
  import AboutDialog from "$lib/AboutDialog.svelte";
  import UpdatesDialog from "$lib/UpdatesDialog.svelte";
  import ValidationDialog from "$lib/ValidationDialog.svelte";
  import TemplateSyncOverlay from "$lib/TemplateSyncOverlay.svelte";
  import {
    checkAppUpdate,
    installAppUpdate,
    isTauriRuntime,
    skipVersion,
  } from "$lib/updater";
  import { findTemplateUpdates, syncAllTemplates } from "$lib/templateSync";
  import { TemplateValidation } from "$lib/templateHooks";
  import type {
    OpenedProject,
    RecentItem,
    TemplateInfo,
    TemplateSyncEvent,
    TemplateUpdateInfo,
    ValidationIssue,
  } from "$lib/types";

  const RECENT_KEY = "report-studio:recent";
  const BASE_TITLE = "报告工坊";
  const MAX_TITLE_NAME = 30;
  const DEFAULT_NOTIFICATION_MS = 3500;
  const DEFAULT_ERROR_MS = 6000;
  const RECENT_VALUE_TYPES = new Set([
    "string",
    "text",
    "number",
    "integer",
    "money",
    "percent",
  ]);

  type NotificationTone = "success" | "error" | "warning" | "info";

  interface AppNotification {
    title: string;
    description?: string;
    tone?: NotificationTone;
    primaryLabel?: string;
    secondaryLabel?: string;
    onprimary?: () => void;
    onsecondary?: () => void;
    autoDismissMs?: number;
  }

  let opened = $state<OpenedProject | null>(null);
  let projectPath = $state<string | null>(null);
  let dirty = $state(false);
  let tab = $state("vars");
  let recent = $state<RecentItem[]>([]);
  let showNew = $state(false);
  let refreshKey = $state(0);
  let templatesRefreshKey = $state(0);
  let notification = $state<AppNotification | null>(null);
  let notificationTimer: ReturnType<typeof setTimeout> | undefined;
  let focusReq = $state<{ key: string; n: number } | null>(null);
  let validation = $state<TemplateValidation | null>(null);
  let localValidationIssues = $state<ValidationIssue[]>([]);
  let usageVersion = $state(0);
  let validationDialog = $state<{
    issues: ValidationIssue[];
    actionLabel?: string;
  } | null>(null);
  let validationDialogResolve: ((proceed: boolean) => void) | null = null;

  let showAbout = $state(false);
  let updatesOpen = $state(false);
  let updatesChecking = $state(false);

  let appVersion = $state("");
  let appUpdate = $state<Update | null>(null);
  let appInstalling = $state(false);
  let appPercent = $state(0);
  let appError = $state("");

  let templateUpdates = $state<TemplateUpdateInfo[]>([]);
  let templateSyncing = $state(false);
  let templatePercent = $state(0);
  let templateError = $state("");
  let templateMessage = $state("");
  let showFirstRunSync = $state(false);

  const windowTitle = $derived.by(() => {
    const name = opened?.project.meta.name?.trim();
    const suffix = dirty ? "*" : "";
    if (!name) return `${BASE_TITLE}${suffix}`;
    const chars = Array.from(name);
    const clipped =
      chars.length > MAX_TITLE_NAME
        ? `${chars.slice(0, MAX_TITLE_NAME).join("")}…`
        : name;
    return `${BASE_TITLE} - ${clipped}${suffix}`;
  });

  $effect(() => {
    const title = windowTitle;
    if (typeof document !== "undefined") document.title = title;
    if (!isTauriRuntime()) return;
    void getCurrentWindow().setTitle(title).catch(() => {});
  });

  $effect(() => {
    const current = opened;
    localValidationIssues = [];
    const hookPath =
      current && typeof current.manifest.hooks?.validate === "string"
        ? current.manifest.hooks.validate
        : "";
    const next =
      current && hookPath
        ? new TemplateValidation({
            templateDir: current.templateDir,
            hookPath,
            variables: current.manifest.variables,
            readText: api.readTemplateText,
          })
        : null;
    validation = next;
    return () => next?.dispose();
  });

  function editField(key: string) {
    focusReq = { key, n: (focusReq?.n ?? 0) + 1 };
    tab = "vars";
  }

  function markDirty() {
    dirty = true;
  }

  onMount(() => {
    loadRecent();
    let unlisten: (() => void) | undefined;
    let disposed = false;

    if (isTauriRuntime()) {
      getVersion()
        .then((version) => (appVersion = version))
        .catch(() => {});

      void (async () => {
        try {
          const off = await listen<string>("open-project", (event) => {
            if (typeof event.payload === "string" && event.payload) {
              void openPath(event.payload);
            }
          });
          if (disposed) {
            off();
            return;
          }
          unlisten = off;

          const pending = await api.takeLaunchProject();
          if (!disposed && pending) await openPath(pending);
        } catch {
          // 文件关联不可用时忽略
        }
      })();
    }

    const timer = setTimeout(runStartupChecks, 3000);
    return () => {
      disposed = true;
      unlisten?.();
      clearTimeout(timer);
    };
  });

  function loadRecent() {
    try {
      recent = JSON.parse(localStorage.getItem(RECENT_KEY) ?? "[]");
    } catch {
      recent = [];
    }
  }

  function saveRecent() {
    localStorage.setItem(RECENT_KEY, JSON.stringify(recent));
  }

  function pushRecent(item: RecentItem) {
    recent = [item, ...recent.filter((r) => r.path !== item.path)].slice(0, 8);
    saveRecent();
  }

  function showNotification(next: AppNotification) {
    clearTimeout(notificationTimer);
    notification = next;
    const delay =
      next.autoDismissMs === undefined
        ? next.tone === "error"
          ? DEFAULT_ERROR_MS
          : DEFAULT_NOTIFICATION_MS
        : next.autoDismissMs;
    if (delay <= 0) return;
    notificationTimer = setTimeout(() => {
      notification = null;
    }, delay);
  }

  function notify(
    title: string,
    tone: NotificationTone = "success",
    description?: string,
  ) {
    showNotification({ title, description, tone });
  }

  function notifyError(title: string, error: unknown) {
    showNotification({
      title,
      description: String(error),
      tone: "error",
      autoDismissMs: DEFAULT_ERROR_MS,
    });
  }

  function dismissNotification() {
    clearTimeout(notificationTimer);
    notification = null;
  }

  function fileName(path: string): string {
    return path.split(/[\\/]/).pop() || path;
  }

  async function rememberTemplateValues() {
    const current = opened;
    if (!current) return;

    const values: Record<string, unknown> = {};
    for (const variable of current.manifest.variables) {
      if (!RECENT_VALUE_TYPES.has(variable.type)) continue;
      const value = current.project.values[variable.key];
      if (value === undefined || value === null) continue;
      if (typeof value === "string" && !value.trim()) continue;
      if (typeof value === "number" && !Number.isFinite(value)) continue;
      values[variable.key] = value;
    }
    if (!Object.keys(values).length) return;

    try {
      await api.recordTemplateValues(current.manifest.id, values);
      usageVersion += 1;
    } catch {
      /* 最近值记录失败不影响主流程 */
    }
  }

  async function openGeneratedFile(path: string) {
    try {
      await openWithSystem(path);
    } catch (e) {
      notifyError("打开文件失败", e);
    }
  }

  async function revealGeneratedFile(path: string) {
    try {
      await revealItemInDir(path);
    } catch (e) {
      notifyError("打开文件夹失败", e);
    }
  }

  /* ---------------- 更新 ---------------- */
  async function runStartupChecks() {
    if (!import.meta.env.PROD || !isTauriRuntime()) return;

    let templates: TemplateInfo[] = [];
    try {
      templates = await api.listTemplates();
    } catch {
      templates = [];
    }

    if (templates.length === 0) {
      showFirstRunSync = true;
      await syncTemplatesNow(true);
    }

    if (!showFirstRunSync) {
      await checkAllUpdates(false);
    }
  }

  async function syncTemplatesNow(firstRun: boolean) {
    if (templateSyncing) return;
    templateSyncing = true;
    templatePercent = 0;
    templateError = "";
    templateMessage = "正在同步模板…";

    try {
      await syncAllTemplates((event: TemplateSyncEvent) => {
        const unit = event.total ? (event.current + (event.percent ?? 0) / 100) / event.total : 0;
        templatePercent = Math.max(0, Math.min(100, Math.round(unit * 100)));
        if (event.message) templateMessage = event.message;
        if (event.phase === "installed" && event.id) {
          templateMessage = `已安装 ${event.id}`;
        }
      });
      templatesRefreshKey += 1;
      showFirstRunSync = false;
      templateUpdates = [];
      notify(firstRun ? "模板准备完成" : "模板已更新", "success");
    } catch (e) {
      templateError = String(e);
      if (!firstRun) notifyError("模板更新失败", e);
    } finally {
      templateSyncing = false;
    }
  }

  async function checkAllUpdates(manual = true) {
    if (updatesChecking || appInstalling || templateSyncing) return;
    updatesChecking = true;
    appError = "";
    templateError = "";
    appUpdate = null;
    templateUpdates = [];
    if (manual) updatesOpen = true;

    try {
      const [appResult, templateResult] = await Promise.allSettled([
        checkAppUpdate(manual),
        findTemplateUpdates(),
      ]);

      if (appResult.status === "fulfilled") {
        appUpdate = appResult.value;
      } else {
        appError = String(appResult.reason);
      }

      if (templateResult.status === "fulfilled") {
        templateUpdates = templateResult.value;
      } else {
        templateError = String(templateResult.reason);
      }

      if (!manual && (appUpdate || templateUpdates.length > 0)) {
        updatesOpen = true;
      }
    } finally {
      updatesChecking = false;
    }
  }

  function openUpdates() {
    if (appUpdate || templateUpdates.length > 0) {
      updatesOpen = true;
    } else {
      void checkAllUpdates(true);
    }
  }

  async function installUpdate() {
    if (!appUpdate || appInstalling) return;
    appInstalling = true;
    appError = "";
    appPercent = 0;
    try {
      await installAppUpdate(appUpdate, (progress) => {
        appPercent = progress.percent;
      });
    } catch (e) {
      appError = String(e);
      appInstalling = false;
    }
  }

  function skipAppUpdate() {
    if (!appUpdate) return;
    skipVersion(appUpdate.version);
    appUpdate = null;
    appInstalling = false;
    appError = "";
  }

  /* ---------------- 文件 ---------------- */
  function openNewDialog() {
    showNew = true;
  }

  function onCreated(o: OpenedProject, path: string) {
    pushRecent({ name: o.project.meta.name, path, template: o.manifest.name });
    opened = o;
    projectPath = path;
    dirty = false;
    focusReq = null;
    showNew = false;
    tab = "vars";
  }

  async function openExisting() {
    const path = await openDialog({
      multiple: false,
      filters: [{ name: "报告工程 (.rproj)", extensions: ["rproj", "zip"] }],
    });
    if (typeof path !== "string") return;
    await openPath(path);
  }

  async function openPath(path: string) {
    try {
      const o = await api.openProject(path);
      pushRecent({ name: o.project.meta.name, path, template: o.manifest.name });
      opened = o;
      projectPath = path;
      dirty = false;
      focusReq = null;
      tab = "vars";
    } catch (e) {
      notifyError("打开工程失败", e);
      recent = recent.filter((r) => r.path !== path);
      saveRecent();
    }
  }

  async function installTemplate() {
    const path = await openDialog({
      multiple: false,
      filters: [{ name: "模板包 (.zip)", extensions: ["zip"] }],
    });
    if (typeof path !== "string") return;
    try {
      const t = await api.installTemplate(path);
      templatesRefreshKey += 1;
      notify("模板已载入", "success", t.name);
    } catch (e) {
      notifyError("模板载入失败", e);
    }
  }

  async function saveProject() {
    if (!opened) return;
    if (!projectPath) {
      await saveProjectAs();
      return;
    }
    try {
      await api.saveProject(projectPath, opened.project, true, opened.projectId);
      opened.project.meta.updatedAt = new Date().toISOString();
      dirty = false;
      await rememberTemplateValues();
      notify("工程已保存", "success");
    } catch (e) {
      notifyError("保存失败", e);
    }
  }

  async function saveProjectAs() {
    if (!opened) return;
    const out = await saveDialog({
      defaultPath: `${opened.project.meta.name}.rproj`,
      filters: [{ name: "报告工程 (.rproj)", extensions: ["rproj"] }],
    });
    if (typeof out !== "string") return;
    try {
      await api.saveProject(out, opened.project, true, opened.projectId);
      opened.project.meta.updatedAt = new Date().toISOString();
      dirty = false;
      projectPath = out;
      pushRecent({
        name: opened.project.meta.name,
        path: out,
        template: opened.manifest.name,
      });
      await rememberTemplateValues();
      notify("工程已另存为", "success");
    } catch (e) {
      notifyError("另存为失败", e);
    }
  }

  function requiredIssues(o: OpenedProject): ValidationIssue[] {
    const issues: ValidationIssue[] = [];
    for (const v of o.manifest.variables) {
      if (v.required && v.type !== "computed") {
        const val = o.project.values[v.key];
        if (val === undefined || val === null || val === "") {
          issues.push({
            status: "error",
            keys: [v.key],
            msg: `必填项未填写：${v.label ?? v.alias ?? v.key}`,
          });
        }
      }
    }
    return issues;
  }

  function dedupeIssues(issues: ValidationIssue[]): ValidationIssue[] {
    const seen = new Set<string>();
    return issues.filter((issue) => {
      const key = `${issue.status}|${[...issue.keys].sort().join(",")}|${issue.msg}`;
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
  }

  async function collectValidationIssues(): Promise<ValidationIssue[]> {
    if (!opened) return [];
    const issues = [...localValidationIssues, ...requiredIssues(opened)];
    if (!validation) return dedupeIssues(issues);

    let resolved: Record<string, unknown>;
    try {
      resolved = await api.resolveValues(
        opened.templateDir,
        opened.project.values,
      );
    } catch (e) {
      issues.push({
        status: "error",
        keys: [],
        msg: `校验失败：${String(e)}`,
      });
      return dedupeIssues(issues);
    }

    const hookIssues = await validation.run(
      "beforeExport",
      resolved,
      opened.project.values,
    );
    issues.push(...hookIssues.filter((issue) => issue.status !== "ok"));
    return dedupeIssues(issues);
  }

  function askValidation(
    issues: ValidationIssue[],
    actionLabel?: string,
  ): Promise<boolean> {
    const visible = issues.filter((issue) => issue.status !== "ok");
    if (!visible.length) return Promise.resolve(true);
    if (validationDialogResolve) return Promise.resolve(false);

    return new Promise<boolean>((resolve) => {
      validationDialogResolve = resolve;
      validationDialog = { issues: visible, actionLabel };
    });
  }

  function finishValidationDialog(proceed: boolean) {
    const resolve = validationDialogResolve;
    validationDialogResolve = null;
    validationDialog = null;
    resolve?.(proceed);
  }

  function closeValidationDialog() {
    const first =
      validationDialog?.issues.find((issue) => issue.status === "error") ??
      validationDialog?.issues[0];
    finishValidationDialog(false);
    const key = first?.key ?? first?.keys[0];
    if (key) editField(key);
    else tab = "vars";
  }

  function locateValidationIssue(key: string) {
    finishValidationDialog(false);
    editField(key);
  }

  async function exportPdf() {
    if (!opened || validationDialog) return;

    const issues = await collectValidationIssues();
    const errors = issues.filter((issue) => issue.status === "error");
    if (errors.length) {
      await askValidation(issues);
      return;
    }
    if (issues.length && !(await askValidation(issues, "仍然导出 PDF"))) return;

    const out = await saveDialog({
      defaultPath: `${opened.manifest.name}.pdf`,
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });
    if (typeof out !== "string") return;
    try {
      const path = await api.generatePdf(
        opened.templateDir,
        opened.project.values,
        opened.projectId,
        out,
      );
      await rememberTemplateValues();
      showNotification({
        tone: "success",
        title: "PDF 已生成",
        description: fileName(path),
        primaryLabel: "打开文件",
        secondaryLabel: "打开文件夹",
        onprimary: () => void openGeneratedFile(path),
        onsecondary: () => void revealGeneratedFile(path),
        autoDismissMs: 0,
      });
    } catch (e) {
      notifyError("PDF 生成失败", e);
    }
  }

  function closeProject() {
    if (validationDialogResolve) finishValidationDialog(false);
    opened = null;
    projectPath = null;
    dirty = false;
    focusReq = null;
    tab = "vars";
  }

  async function printReport() {
    if (!opened || validationDialog) return;

    const issues = await collectValidationIssues();
    const errors = issues.filter((issue) => issue.status === "error");
    if (errors.length) {
      await askValidation(issues);
      return;
    }
    if (issues.length && !(await askValidation(issues, "仍然打印"))) return;

    try {
      await api.printReport(opened.templateDir, opened.project.values, opened.projectId);
      await rememberTemplateValues();
      notify("已调用打印", "success");
    } catch (e) {
      notifyError("打印失败", e);
    }
  }

  /* ---------------- 视图 ---------------- */
  function refreshPreview() {
    refreshKey += 1;
  }

  function handleKey(e: KeyboardEvent) {
    if (!(e.ctrlKey || e.metaKey)) return;
    switch (e.key.toLowerCase()) {
      case "n":
        e.preventDefault();
        openNewDialog();
        break;
      case "o":
        e.preventDefault();
        openExisting();
        break;
      case "s":
        if (opened) {
          e.preventDefault();
          if (e.shiftKey) saveProjectAs();
          else saveProject();
        }
        break;
      case "p":
        if (opened) {
          e.preventDefault();
          if (e.shiftKey) exportPdf();
          else printReport();
        }
        break;
      case "r":
        if (opened && tab === "preview") {
          e.preventDefault();
          refreshPreview();
        }
        break;
    }
  }
</script>

<svelte:window onkeydown={handleKey} />

<div class="app">
  <header class="menubar">
    <Menubar.Root class="w-full">
      <Menubar.Menu>
        <Menubar.Trigger>文件</Menubar.Trigger>
        <Menubar.Content>
          <Menubar.Item onSelect={openNewDialog}>
            新建项目 <Menubar.Shortcut>Ctrl N</Menubar.Shortcut>
          </Menubar.Item>
          <Menubar.Item onSelect={openExisting}>
            打开项目 <Menubar.Shortcut>Ctrl O</Menubar.Shortcut>
          </Menubar.Item>
          <Menubar.Item onSelect={closeProject} disabled={!opened}>
            关闭项目
          </Menubar.Item>
          <Menubar.Item onSelect={installTemplate}>
            载入新模板
          </Menubar.Item>
          <Menubar.Separator />
          <Menubar.Item onSelect={saveProject} disabled={!opened}>
            保存 <Menubar.Shortcut>Ctrl S</Menubar.Shortcut>
          </Menubar.Item>
          <Menubar.Item onSelect={saveProjectAs} disabled={!opened}>
            另存为… <Menubar.Shortcut>Ctrl Shift S</Menubar.Shortcut>
          </Menubar.Item>
          <Menubar.Item onSelect={printReport} disabled={!opened}>
            打印… <Menubar.Shortcut>Ctrl P</Menubar.Shortcut>
          </Menubar.Item>
          <Menubar.Item onSelect={exportPdf} disabled={!opened}>
            输出为 PDF <Menubar.Shortcut>Ctrl Shift P</Menubar.Shortcut>
          </Menubar.Item>
        </Menubar.Content>
      </Menubar.Menu>

      <Menubar.Menu>
        <Menubar.Trigger>视图</Menubar.Trigger>
        <Menubar.Content>
          <Menubar.RadioGroup bind:value={tab}>
            <Menubar.GroupHeading class="text-xs font-medium text-muted-foreground">
              切换到
            </Menubar.GroupHeading>
            <Menubar.RadioItem value="vars">编辑变量</Menubar.RadioItem>
            <Menubar.RadioItem value="preview">预览</Menubar.RadioItem>
          </Menubar.RadioGroup>
          <Menubar.Separator />
          <Menubar.Item onSelect={refreshPreview} disabled={!opened || tab !== "preview"}>
            刷新预览 <Menubar.Shortcut>Ctrl R</Menubar.Shortcut>
          </Menubar.Item>
        </Menubar.Content>
      </Menubar.Menu>

      <Menubar.Menu>
        <Menubar.Trigger>帮助</Menubar.Trigger>
        <Menubar.Content>
          <Menubar.Item onSelect={() => checkAllUpdates(true)}>
            检查更新
          </Menubar.Item>
          <Menubar.Separator />
          <Menubar.Item onSelect={() => (showAbout = true)}>
            关于报告工坊
          </Menubar.Item>
        </Menubar.Content>
      </Menubar.Menu>
    </Menubar.Root>
  </header>

  <main class="main">
    {#if opened}
      <div class="work">
        {#if tab === "vars"}
          <Editor
            manifest={opened.manifest}
            project={opened.project}
            templateDir={opened.templateDir}
            projectId={opened.projectId}
            {focusReq}
            {validation}
            localIssues={localValidationIssues}
            onlocalissues={(issues) => (localValidationIssues = issues)}
            usageVersion={usageVersion}
            onchange={markDirty}
          />
        {:else}
          {#key refreshKey}
            <Preview {opened} onedit={editField} />
          {/key}
        {/if}
      </div>
    {:else}
      <Start
        {recent}
        onnew={openNewDialog}
        onopen={openExisting}
        oninstall={installTemplate}
        onopenrecent={openPath}
      />
    {/if}
  </main>

  {#if opened}
    <div class="pill">
      <button class:active={tab === "vars"} onclick={() => (tab = "vars")}>
        变量
      </button>
      <button class:active={tab === "preview"} onclick={() => (tab = "preview")}>
        预览
      </button>
    </div>
  {/if}

  {#if validationDialog}
    <ValidationDialog
      issues={validationDialog.issues}
      variables={opened?.manifest.variables ?? []}
      actionLabel={validationDialog.actionLabel}
      onlocate={locateValidationIssue}
      onproceed={() => finishValidationDialog(true)}
      onclose={closeValidationDialog}
    />
  {/if}

  {#if showNew}
    {#key templatesRefreshKey}
      <NewProjectDialog
        oncreated={onCreated}
        oncancel={() => (showNew = false)}
        onsync={() => syncTemplatesNow(false)}
      />
    {/key}
  {/if}

  {#if updatesOpen}
    <UpdatesDialog
      bind:open={updatesOpen}
      checking={updatesChecking}
      appUpdate={appUpdate}
      appInstalling={appInstalling}
      appPercent={appPercent}
      appError={appError}
      templateUpdates={templateUpdates}
      templateSyncing={templateSyncing}
      templatePercent={templatePercent}
      templateError={templateError}
      oninstall={installUpdate}
      onskip={skipAppUpdate}
      onsync={() => syncTemplatesNow(false)}
      onrecheck={() => checkAllUpdates(true)}
      onclose={() => (updatesOpen = false)}
    />
  {/if}

  {#if showAbout}
    <AboutDialog
      bind:open={showAbout}
      version={appVersion}
      hasUpdates={!!appUpdate || templateUpdates.length > 0}
      oncheckupdates={openUpdates}
    />
  {/if}

  {#if showFirstRunSync}
    <TemplateSyncOverlay
      percent={templatePercent}
      message={templateMessage}
      error={templateError}
      onretry={() => syncTemplatesNow(true)}
      onclose={() => (showFirstRunSync = false)}
    />
  {/if}

  {#if notification}
    <div class="notification-host">
      <GeneralNotification06
        visible={true}
        title={notification.title}
        description={notification.description}
        tone={notification.tone}
        primaryLabel={notification.primaryLabel}
        secondaryLabel={notification.secondaryLabel}
        onprimary={notification.onprimary}
        onsecondary={notification.onsecondary}
        ondismiss={dismissNotification}
      />
    </div>
  {/if}
</div>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100%;
    position: relative;
  }

  /* ---------- Menubar ---------- */
  .menubar {
    display: flex;
    align-items: center;
    z-index: 30;
  }
  /* ---------- Main ---------- */
  .main {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
  .work {
    height: 100%;
    overflow: auto;
    padding: 0;
  }

  /* ---------- Floating pill ---------- */
  .pill {
    position: absolute;
    top: 7px;
    right: 10px;
    z-index: 40;
    display: flex;
    gap: 1px;
    padding: 2px;
    border-radius: var(--radius-pill);
    background: rgba(255, 255, 255, 0.5);
    backdrop-filter: blur(8px);
    box-shadow: var(--shadow-border);
    opacity: 0.3;
    transition:
      opacity 0.18s ease,
      background-color 0.18s ease;
  }
  .pill:hover {
    opacity: 1;
    background: #ffffff;
  }
  .pill button {
    padding: 2px 10px;
    border: none;
    border-radius: var(--radius-pill);
    background: transparent;
    box-shadow: none;
    color: var(--muted);
    font-size: 11px;
    font-weight: 500;
  }
  .pill button:hover {
    background: var(--surface-50);
    box-shadow: none;
    color: var(--text);
  }
  .pill button.active {
    background: var(--text);
    color: #ffffff;
  }
  .pill button.active:hover {
    color: #ffffff;
  }

  /* ---------- Notification ---------- */
  .notification-host {
    position: fixed;
    right: 24px;
    bottom: 24px;
    z-index: 200;
    width: min(28rem, calc(100vw - 3rem));
  }
</style>
