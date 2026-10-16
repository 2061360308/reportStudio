<script lang="ts">
  import { onMount } from "svelte";
  import { save as saveDialog } from "@tauri-apps/plugin-dialog";
  import * as api from "./api";
  import type { OpenedProject, TemplateInfo } from "./types";

  let {
    oncreated,
    oncancel,
  }: {
    oncreated: (o: OpenedProject, path: string) => void;
    oncancel: () => void;
  } = $props();

  let templates = $state<TemplateInfo[]>([]);
  let selected = $state("");
  let savePath = $state("");
  let busy = $state(false);
  let error = $state("");

  const chosen = $derived(templates.find((t) => t.id === selected) ?? null);
  const projectName = $derived(basename(savePath));

  onMount(load);

  function basename(p: string): string {
    if (!p) return "";
    const name = p.split(/[\\/]/).pop() ?? "";
    return name.replace(/\.[^.]+$/, "");
  }

  async function load() {
    error = "";
    try {
      templates = await api.listTemplates();
      if (templates.length && !selected) selected = templates[0].id;
    } catch (e) {
      error = String(e);
    }
  }

  async function pickPath() {
    const out = await saveDialog({
      defaultPath: `${chosen?.name ?? "新建报告"}.rproj`,
      filters: [{ name: "报告工程 (.rproj)", extensions: ["rproj"] }],
    });
    if (typeof out === "string") savePath = out;
  }

  async function start() {
    if (!selected || !savePath) return;
    busy = true;
    error = "";
    try {
      const o = await api.newProject(selected);
      o.project.meta.name = projectName || o.project.meta.name;
      await api.saveProject(savePath, o.project, true, o.projectId);
      oncreated(o, savePath);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") oncancel();
  }
</script>

<svelte:window onkeydown={onkeydown} />

<div
  class="overlay"
  role="presentation"
  onclick={oncancel}
  onkeydown={() => {}}
>
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-label="新建项目"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
    onkeydown={() => {}}
  >
    <h2>新建项目</h2>
    <p class="sub">选择模板，并指定工程保存位置。</p>

    <div class="body">
      {#if templates.length === 0}
        <p class="hint">暂无已安装模板，请先在菜单「文件 › 载入新模板」安装。</p>
      {:else}
        <ul class="tpl-list">
          {#each templates as t (t.id)}
            <li>
              <button
                class="tpl"
                class:selected={t.id === selected}
                onclick={() => (selected = t.id)}
                ondblclick={start}
              >
                <span class="tpl-name">{t.name}</span>
                <span class="tpl-id mono">{t.id}</span>
                <span class="pill">v{t.version}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <div class="path-row">
      <span class="label">保存位置</span>
      <button onclick={pickPath}>选择文件位置...</button>
      <span class="path mono" title={savePath}>
        {savePath || "未选择"}
      </span>
    </div>

    {#if projectName}
      <div class="name-row">
        <span class="label">工程名称</span>
        <span class="name">{projectName}</span>
        <span class="ro">（由文件名自动读取，不可编辑）</span>
      </div>
    {/if}

    {#if error}
      <pre class="error">{error}</pre>
    {/if}

    <div class="actions">
      <button onclick={oncancel} disabled={busy}>取消</button>
      <button
        class="primary"
        onclick={start}
        disabled={busy || !selected || !savePath}
      >
        开始
      </button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: rgba(0, 0, 0, 0.4);
  }
  .dialog {
    width: 620px;
    max-width: 92vw;
    max-height: 88vh;
    display: flex;
    flex-direction: column;
    padding: 24px;
    border-radius: var(--radius-lg);
    background: var(--panel);
    box-shadow: var(--shadow-elev);
  }
  h2 {
    margin: 0;
    font-size: 20px;
    font-weight: 600;
    letter-spacing: -0.3px;
  }
  .sub {
    margin: 6px 0 0;
    font-size: 14px;
    color: var(--text-secondary);
  }

  .body {
    flex: 1;
    min-height: 0;
    overflow: auto;
    margin: 18px 0 0;
  }
  .tpl-list {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 12px;
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .tpl {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    width: 100%;
    padding: 14px 16px;
    border: none;
    border-radius: var(--radius-lg);
    background: var(--panel);
    box-shadow: var(--shadow-card);
    color: var(--text);
    text-align: left;
    cursor: pointer;
    transition: box-shadow 0.15s ease;
  }
  .tpl:hover {
    background: var(--panel);
    box-shadow: var(--shadow-elev);
  }
  .tpl.selected {
    box-shadow:
      0 0 0 2px var(--develop),
      rgba(0, 0, 0, 0.04) 0px 2px 2px,
      #fafafa 0px 0px 0px 1px;
  }
  .tpl-name {
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.2px;
  }
  .tpl-id {
    font-size: 12px;
    color: var(--placeholder);
  }
  .pill {
    margin-top: 2px;
    padding: 2px 10px;
    border-radius: var(--radius-pill);
    background: var(--badge-bg);
    color: var(--badge-text);
    font-size: 12px;
    font-weight: 500;
  }

  .path-row,
  .name-row {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 16px;
  }
  .label {
    flex: none;
    width: 72px;
    color: var(--text-secondary);
    font-size: 14px;
  }
  .path {
    flex: 1;
    min-width: 0;
    font-size: 12px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .name {
    font-size: 14px;
    font-weight: 600;
  }
  .ro {
    color: var(--placeholder);
    font-size: 12px;
  }
  .hint {
    margin: 0;
    font-size: 13px;
    color: var(--placeholder);
  }
  .error {
    margin: 14px 0 0;
    color: var(--danger);
    font-size: 13px;
    white-space: pre-wrap;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 24px;
  }
</style>
