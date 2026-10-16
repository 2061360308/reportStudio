<script lang="ts">
  import { onMount } from "svelte";
  import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
  import * as api from "$lib/api";
  import * as Menubar from "$lib/components/elements/menubar";
  import Start from "$lib/Start.svelte";
  import Editor from "$lib/Editor.svelte";
  import Preview from "$lib/Preview.svelte";
  import NewProjectDialog from "$lib/NewProjectDialog.svelte";
  import type { OpenedProject, RecentItem } from "$lib/types";

  const RECENT_KEY = "report-studio:recent";

  let opened = $state<OpenedProject | null>(null);
  let projectPath = $state<string | null>(null);
  let tab = $state("vars");
  let recent = $state<RecentItem[]>([]);
  let showNew = $state(false);
  let refreshKey = $state(0);
  let toast = $state("");
  let focusReq = $state<{ key: string; n: number } | null>(null);
  let toastTimer: ReturnType<typeof setTimeout> | undefined;

  function editField(key: string) {
    focusReq = { key, n: (focusReq?.n ?? 0) + 1 };
    tab = "vars";
  }

  onMount(loadRecent);

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

  function toastMsg(msg: string) {
    toast = msg;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = ""), 3200);
  }

  /* ---------------- 文件 ---------------- */
  function openNewDialog() {
    showNew = true;
  }

  function onCreated(o: OpenedProject, path: string) {
    pushRecent({ name: o.project.meta.name, path, template: o.manifest.name });
    opened = o;
    projectPath = path;
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
      tab = "vars";
    } catch (e) {
      toastMsg(String(e));
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
      toastMsg(`已载入模板：${t.name}`);
    } catch (e) {
      toastMsg(String(e));
    }
  }

  async function saveProject() {
    if (!opened) return;
    if (!projectPath) {
      await saveProjectAs();
      return;
    }
    try {
      await api.saveProject(projectPath, opened.project, true);
      opened.project.meta.updatedAt = new Date().toISOString();
      toastMsg("已保存工程");
    } catch (e) {
      toastMsg(String(e));
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
      await api.saveProject(out, opened.project, true);
      opened.project.meta.updatedAt = new Date().toISOString();
      projectPath = out;
      pushRecent({
        name: opened.project.meta.name,
        path: out,
        template: opened.manifest.name,
      });
      toastMsg("已另存为");
    } catch (e) {
      toastMsg(String(e));
    }
  }

  function validate(o: OpenedProject): string | null {
    for (const v of o.manifest.variables) {
      if (v.required && v.type !== "computed") {
        const val = o.project.values[v.key];
        if (val === undefined || val === null || val === "") {
          return `必填项未填写：${v.label ?? v.key}`;
        }
      }
    }
    return null;
  }

  async function exportPdf() {
    if (!opened) return;
    const v = validate(opened);
    if (v) {
      toastMsg(v);
      return;
    }
    const out = await saveDialog({
      defaultPath: `${opened.manifest.name}.pdf`,
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });
    if (typeof out !== "string") return;
    try {
      await api.generatePdf(opened.templateDir, opened.project.values, out);
      toastMsg("已生成 PDF");
    } catch (e) {
      toastMsg(String(e));
    }
  }

  function backToWelcome() {
    opened = null;
    projectPath = null;
    tab = "vars";
  }

  async function printReport() {
    if (!opened) return;
    const v = validate(opened);
    if (v) {
      toastMsg(v);
      return;
    }
    try {
      await api.printReport(opened.templateDir, opened.project.values);
      toastMsg("已调用打印");
    } catch (e) {
      toastMsg(String(e));
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
          <Menubar.Item onSelect={backToWelcome} disabled={!opened}>
            返回欢迎
          </Menubar.Item>
        </Menubar.Content>
      </Menubar.Menu>

      <Menubar.Menu>
        <Menubar.Trigger>帮助</Menubar.Trigger>
        <Menubar.Content>
          <Menubar.Item onSelect={() => toastMsg("报告工坊 v0.1.0")}>
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
            {focusReq}
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

  {#if showNew}
    <NewProjectDialog oncreated={onCreated} oncancel={() => (showNew = false)} />
  {/if}

  {#if toast}
    <div class="toast">{toast}</div>
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

  /* ---------- Toast ---------- */
  .toast {
    position: fixed;
    left: 50%;
    bottom: 24px;
    transform: translateX(-50%);
    z-index: 200;
    padding: 8px 16px;
    border-radius: var(--radius);
    background: var(--text);
    color: #ffffff;
    font-size: 13px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.16);
  }
</style>
