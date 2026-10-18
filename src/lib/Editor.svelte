<script lang="ts">
  import { onMount } from "svelte";
  import type { Project, TemplateManifest } from "./types";
  import TableEditor from "./TableEditor.svelte";
  import ImageEditor from "./ImageEditor.svelte";
  import * as api from "./api";

  let {
    manifest,
    project,
    templateDir,
    projectId,
    focusReq,
    onchange,
  }: {
    manifest: TemplateManifest;
    project: Project;
    templateDir: string;
    projectId: string;
    focusReq?: { key: string; n: number } | null;
    onchange?: () => void;
  } = $props();

  let root = $state<HTMLDivElement | undefined>();
  let flashKey = $state("");

  let resolved = $state<Record<string, unknown>>({});
  let timer: ReturnType<typeof setTimeout> | undefined;

  function refresh() {
    clearTimeout(timer);
    timer = setTimeout(async () => {
      try {
        resolved = await api.resolveValues(templateDir, project.values);
      } catch {
        /* 忽略：依赖未填时计算结果为空 */
      }
    }, 120);
  }

  onMount(refresh);

  $effect(() => {
    if (!focusReq || !root) return;
    const key = focusReq.key;
    const el = root.querySelector<HTMLElement>(`[data-field="${key}"]`);
    if (!el) return;
    el.scrollIntoView({ block: "center", behavior: "smooth" });
    el.querySelector<HTMLElement>("input, textarea, select")?.focus();
    flashKey = key;
    window.setTimeout(() => {
      if (flashKey === key) flashKey = "";
    }, 1600);
  });

  function get(key: string): unknown {
    return project.values[key];
  }

  function set(key: string, val: unknown) {
    project.values[key] = val;
    onchange?.();
    refresh();
  }

  function toNum(e: Event): number | null {
    const v = (e.target as HTMLInputElement).value;
    return v === "" ? null : Number(v);
  }

  function computedText(key: string, unit?: string): string {
    const v = resolved[key];
    if (v === undefined || v === null || v === "") return "—";
    return unit ? `${v} ${unit}` : String(v);
  }
</script>

<div class="editor" bind:this={root}>
  {#each manifest.variables as v (v.key)}
    <div
      class="field"
      class:wide={v.type === "text" ||
        v.type === "table" ||
        v.type === "images"}
      class:flash={flashKey === v.key}
      data-field={v.key}
    >
      <div class="label">
        <span>{v.label ?? v.alias ?? v.key}</span>
        {#if v.required}<span class="req">*</span>{/if}
      </div>

      {#if v.type === "computed"}
        <div class="computed">{computedText(v.key, v.unit)}</div>
      {:else if v.type === "table"}
        <TableEditor variable={v} {project} {onchange} />
      {:else if v.type === "image" || v.type === "images"}
        <ImageEditor variable={v} {project} {projectId} {onchange} />
      {:else if v.type === "text"}
        <textarea
          rows="3"
          placeholder={v.placeholder ?? ""}
          value={String(get(v.key) ?? "")}
          oninput={(e) => set(v.key, (e.target as HTMLTextAreaElement).value)}
        ></textarea>
      {:else if v.type === "bool"}
        <label class="check">
          <input
            type="checkbox"
            checked={Boolean(get(v.key))}
            onchange={(e) => set(v.key, (e.target as HTMLInputElement).checked)}
          />
          <span>{Boolean(get(v.key)) ? "是" : "否"}</span>
        </label>
      {:else if v.type === "select"}
        <select
          value={String(get(v.key) ?? "")}
          onchange={(e) => set(v.key, (e.target as HTMLSelectElement).value)}
        >
          <option value="">--</option>
          {#each v.options ?? [] as o (o.value)}
            <option value={o.value}>{o.label}</option>
          {/each}
        </select>
      {:else if v.type === "multiselect"}
        <select
          multiple
          onchange={(e) =>
            set(
              v.key,
              Array.from((e.target as HTMLSelectElement).selectedOptions).map(
                (o) => o.value,
              ),
            )}
        >
          {#each v.options ?? [] as o (o.value)}
            <option
              value={o.value}
              selected={(get(v.key) as string[] ?? []).includes(o.value)}
              >{o.label}</option
            >
          {/each}
        </select>
      {:else if v.type === "date"}
        <input
          type="date"
          value={String(get(v.key) ?? "")}
          onchange={(e) => set(v.key, (e.target as HTMLInputElement).value)}
        />
      {:else if v.type === "datetime"}
        <input
          type="datetime-local"
          value={String(get(v.key) ?? "")}
          onchange={(e) => set(v.key, (e.target as HTMLInputElement).value)}
        />
      {:else if ["number", "integer", "money", "percent"].includes(v.type)}
        <div class="num">
          <input
            type="number"
            step={v.type === "integer" ? 1 : v.type === "percent" ? 0.01 : "any"}
            placeholder={v.placeholder ?? ""}
            value={get(v.key) === null || get(v.key) === undefined
              ? ""
              : String(get(v.key))}
            oninput={(e) => set(v.key, toNum(e))}
          />
          {#if v.unit}<span class="unit">{v.unit}</span>{/if}
        </div>
      {:else}
        <input
          type="text"
          placeholder={v.placeholder ?? ""}
          value={String(get(v.key) ?? "")}
          oninput={(e) => set(v.key, (e.target as HTMLInputElement).value)}
        />
      {/if}

      {#if v.help}<span class="help">{v.help}</span>{/if}
    </div>
  {/each}
</div>

<style>
  .editor {
    padding: 24px;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 18px 28px;
    align-content: start;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  .field.wide {
    grid-column: 1 / -1;
  }
  .field.flash {
    border-radius: 8px;
    background: rgba(255, 229, 143, 0.35);
    box-shadow: 0 0 0 2px var(--focus, #0072f5);
  }
  .label {
    display: flex;
    align-items: center;
    gap: 3px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
  }
  .req {
    color: var(--danger);
  }

  .field input[type="text"],
  .field input[type="number"],
  .field input[type="date"],
  .field input[type="datetime-local"],
  .field textarea,
  .field select {
    width: 100%;
  }
  .field textarea {
    resize: vertical;
    min-height: 68px;
    line-height: 1.6;
  }
  .field select[multiple] {
    min-height: 84px;
  }

  .num {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .num input {
    flex: 1;
    min-width: 0;
  }
  .unit {
    flex: none;
    font-size: 12px;
    color: var(--muted);
  }

  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    color: var(--text);
    padding: 5px 0;
  }
  .check input {
    width: 16px;
    height: 16px;
    padding: 0;
    box-shadow: none;
    flex: none;
    accent-color: var(--text);
  }

  .computed {
    display: inline-flex;
    align-items: center;
    width: fit-content;
    height: 33px;
    padding: 0 12px;
    border-radius: var(--radius-sm);
    background: var(--surface-50);
    box-shadow: var(--shadow-border);
    color: var(--placeholder);
    font-size: 13px;
  }

  .help {
    font-size: 12px;
    color: var(--placeholder);
  }
</style>
