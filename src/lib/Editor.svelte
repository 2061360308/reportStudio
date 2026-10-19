<script lang="ts">
  import { onDestroy } from "svelte";
  import CircleX from "@lucide/svelte/icons/circle-x";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import type {
    Project,
    RecentValue,
    TemplateManifest,
    ValidationIssue,
  } from "./types";
  import type { TemplateValidation } from "./templateHooks";
  import { GeneralTooltip03 } from "$lib/components/elements/tooltip";
  import RecentValueMenu from "./RecentValueMenu.svelte";
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
    validation,
    localIssues,
    onlocalissues,
    usageVersion,
  }: {
    manifest: TemplateManifest;
    project: Project;
    templateDir: string;
    projectId: string;
    focusReq?: { key: string; n: number } | null;
    onchange?: (key: string, value: unknown) => void;
    validation?: TemplateValidation | null;
    localIssues?: ValidationIssue[];
    onlocalissues?: (issues: ValidationIssue[]) => void;
    usageVersion?: number;
  } = $props();

  let root = $state<HTMLDivElement | undefined>();
  let flashKey = $state("");

  let resolved = $state<Record<string, unknown>>({});
  let issues = $state<ValidationIssue[]>([]);
  let localIssueList = $state<ValidationIssue[]>([]);
  let recentValues = $state<Record<string, RecentValue[]>>({});
  let recentMenu = $state<{
    key: string;
    items: RecentValue[];
    activeIndex: number;
    left: number;
    top: number;
    width: number;
  } | null>(null);
  let recentBlurTimer: ReturnType<typeof setTimeout> | undefined;
  let lastChanged = $state<{ key: string; value: unknown } | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let validationSeq = 0;

  function refresh(event: "change" | "preview" = "change") {
    clearTimeout(timer);
    const seq = ++validationSeq;
    timer = setTimeout(async () => {
      let resolvedValues: Record<string, unknown>;
      try {
        resolvedValues = await api.resolveValues(templateDir, project.values);
      } catch {
        /* 忽略：依赖未填时计算结果为空 */
        return;
      }
      if (seq !== validationSeq) return;
      resolved = resolvedValues;

      if (!validation) {
        issues = [];
        return;
      }

      const change = event === "change" ? lastChanged : null;
      try {
        const result = await validation.run(
          event,
          resolvedValues,
          project.values,
          change?.key,
          change?.value,
        );
        if (seq !== validationSeq) return;
        issues = result;
      } catch (error) {
        if (seq !== validationSeq) return;
        issues = [
          {
            status: "error",
            keys: [],
            msg: `校验失败：${String(error)}`,
          },
        ];
      }
    }, 120);
  }

  $effect(() => {
    localIssueList = localIssues ?? [];
  });

  $effect(() => {
    const templateId = manifest.id;
    void usageVersion;
    let cancelled = false;
    recentMenu = null;
    recentValues = {};

    api
      .getTemplateUsage(templateId)
      .then((usage) => {
        if (!cancelled) recentValues = usage.variables ?? {};
      })
      .catch(() => {
        if (!cancelled) recentValues = {};
      });

    return () => {
      cancelled = true;
    };
  });

  $effect(() => {
    const close = () => {
      recentMenu = null;
    };
    window.addEventListener("scroll", close, true);
    window.addEventListener("resize", close);
    return () => {
      window.removeEventListener("scroll", close, true);
      window.removeEventListener("resize", close);
    };
  });

  $effect(() => {
    // project/校验器切换时重新做一次完整预览校验
    void validation;
    lastChanged = null;
    issues = [];
    refresh("preview");
  });

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

  const allIssues = $derived([...localIssueList, ...issues]);

  const fieldIssues = $derived.by(() => {
    const map = new Map<string, ValidationIssue[]>();
    for (const issue of allIssues) {
      if (issue.status === "ok") continue;
      for (const key of issue.keys) {
        const list = map.get(key);
        if (list) list.push(issue);
        else map.set(key, [issue]);
      }
    }
    return map;
  });

  const globalIssues = $derived(
    allIssues.filter((issue) => {
      if (issue.status === "ok") return false;
      if (issue.keys.length === 0) return true;
      return !issue.keys.some((key) =>
        manifest.variables.some((variable) => variable.key === key),
      );
    }),
  );
  function fieldState(key: string): "error" | "warning" | "" {
    const list = fieldIssues.get(key);
    if (!list?.length) return "";
    return list.some((issue) => issue.status === "error") ? "error" : "warning";
  }

  function fieldMessagesFor(key: string): string {
    const list = fieldIssues.get(key);
    if (!list?.length) return "";
    return [
      ...new Set(
        list
          .map((issue) => issue.msg.trim())
          .filter((message) => message.length > 0),
      ),
    ].join("\n");
  }

  const AUTOCOMPLETE_TYPES = new Set([
    "string",
    "text",
    "number",
    "integer",
    "money",
    "percent",
  ]);

  function isAutofillable(type: string): boolean {
    return AUTOCOMPLETE_TYPES.has(type);
  }

  function recentText(value: unknown): string {
    if (typeof value === "string") return value.replace(/\s+/g, " ").trim();
    if (value === null || value === undefined) return "";
    return String(value);
  }

  function recentSuggestions(key: string, query: string): RecentValue[] {
    const list = recentValues[key] ?? [];
    const needle = query.trim().toLowerCase();
    if (!needle) return list.slice(0, 5);
    return list
      .filter((item) => recentText(item.value).toLowerCase().includes(needle))
      .slice(0, 5);
  }

  function recentPosition(
    input: HTMLInputElement | HTMLTextAreaElement,
    count: number,
  ) {
    const rect = input.getBoundingClientRect();
    const width = Math.max(
      200,
      Math.min(Math.min(rect.width, 360), window.innerWidth - 16),
    );
    const left = Math.max(
      8,
      Math.min(rect.left, window.innerWidth - width - 8),
    );
    const estimatedHeight = Math.min(count, 5) * 34 + 8;
    const below = rect.bottom + 6;
    const top =
      below + estimatedHeight <= window.innerHeight - 8
        ? below
        : Math.max(8, rect.top - estimatedHeight - 6);
    return { left, top, width };
  }

  function openRecentMenu(
    key: string,
    input: HTMLInputElement | HTMLTextAreaElement,
  ) {
    clearTimeout(recentBlurTimer);
    const variable = manifest.variables.find((item) => item.key === key);
    if (!variable || !isAutofillable(variable.type)) {
      recentMenu = null;
      return;
    }

    const items = recentSuggestions(key, input.value);
    if (!items.length) {
      recentMenu = null;
      return;
    }

    recentMenu = {
      key,
      items,
      activeIndex: 0,
      ...recentPosition(input, items.length),
    };
  }

  function updateRecentMenu(
    key: string,
    input: HTMLInputElement | HTMLTextAreaElement,
  ) {
    if (recentMenu?.key !== key) {
      openRecentMenu(key, input);
      return;
    }

    const items = recentSuggestions(key, input.value);
    if (!items.length) {
      recentMenu = null;
      return;
    }

    recentMenu = {
      ...recentMenu,
      items,
      activeIndex: Math.min(recentMenu.activeIndex, items.length - 1),
      ...recentPosition(input, items.length),
    };
  }

  function chooseRecent(value: unknown) {
    const key = recentMenu?.key;
    if (!key) return;
    recentMenu = null;
    set(key, value);
  }

  function handleRecentKeydown(event: KeyboardEvent) {
    const menu = recentMenu;
    if (!menu) return;

    if (event.key === "ArrowDown") {
      event.preventDefault();
      recentMenu = {
        ...menu,
        activeIndex: (menu.activeIndex + 1) % menu.items.length,
      };
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      recentMenu = {
        ...menu,
        activeIndex:
          (menu.activeIndex - 1 + menu.items.length) % menu.items.length,
      };
      return;
    }
    if (event.key === "Enter") {
      if (menu.items[menu.activeIndex]) {
        event.preventDefault();
        chooseRecent(menu.items[menu.activeIndex].value);
      }
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      recentMenu = null;
      return;
    }
    if (event.key === "Tab") recentMenu = null;
  }

  function scheduleCloseRecentMenu() {
    clearTimeout(recentBlurTimer);
    recentBlurTimer = setTimeout(() => {
      recentMenu = null;
    }, 120);
  }

  function get(key: string): unknown {
    return project.values[key];
  }

  function set(key: string, val: unknown) {
    project.values[key] = val;
    lastChanged = { key, value: val };
    onchange?.(key, val);
    refresh("change");
  }

  function syncComplex(key: string) {
    lastChanged = { key, value: project.values[key] };
    onchange?.(key, project.values[key]);
    refresh("change");
  }

  function labelFor(key: string): string {
    const variable = manifest.variables.find((item) => item.key === key);
    return variable?.label ?? variable?.alias ?? key;
  }

  function setLocalDateIssue(key: string, invalid: boolean) {
    const exists = localIssueList.some((issue) => issue.keys.includes(key));
    if (exists === invalid) return;

    const next = invalid
      ? [
          ...localIssueList.filter((issue) => !issue.keys.includes(key)),
          {
            status: "error" as const,
            keys: [key],
            key,
            msg: `${labelFor(key)}不是有效日期`,
          },
        ]
      : localIssueList.filter((issue) => !issue.keys.includes(key));

    localIssueList = next;
    onlocalissues?.(next);
  }

  function setDate(key: string, event: Event) {
    const input = event.target as HTMLInputElement;

    // 原生 date input 在输入 2026-02-31 这类不存在的日期时，
    // value 会变成空字符串，但 validity.badInput 为 true。
    if (input.validity.badInput) {
      setLocalDateIssue(key, true);
      return;
    }

    setLocalDateIssue(key, false);
    set(key, input.value);
  }

  function checkDateOnBlur(key: string, event: Event) {
    const input = event.target as HTMLInputElement;
    if (input.validity.badInput) setLocalDateIssue(key, true);
  }

  const dateTimers = new Map<string, ReturnType<typeof setTimeout>>();

  function scheduleDateCheck(key: string, event: Event) {
    const input = event.target as HTMLInputElement;
    const existing = dateTimers.get(key);
    if (existing) clearTimeout(existing);

    dateTimers.set(
      key,
      setTimeout(() => {
        dateTimers.delete(key);
        if (input.validity.badInput) {
          setLocalDateIssue(key, true);
        } else if (
          localIssueList.some((issue) => issue.keys.includes(key))
        ) {
          setLocalDateIssue(key, false);
          set(key, input.value);
        }
      }, 400),
    );
  }

  onDestroy(() => {
    for (const timer of dateTimers.values()) clearTimeout(timer);
    dateTimers.clear();
    clearTimeout(recentBlurTimer);
  });

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
  {#if globalIssues.length}
    <div class="validation-global">
      {#each globalIssues as issue, i (i)}
        <div class:global-warn={issue.status === "warning"}>{issue.msg}</div>
      {/each}
    </div>
  {/if}

  {#each manifest.variables as v (v.key)}
    <div
      class="field"
      class:wide={v.type === "text" ||
        v.type === "table" ||
        v.type === "images"}
      class:flash={flashKey === v.key}
      class:invalid={fieldState(v.key) === "error"}
      class:warn={fieldState(v.key) === "warning"}
      data-field={v.key}
    >
      <div class="label">
        <span>{v.label ?? v.alias ?? v.key}</span>
        {#if v.required}<span class="req">*</span>{/if}
        {#if fieldState(v.key)}
          <span class="sr-only">{fieldMessagesFor(v.key)}</span>
          <div class="field-status">
            <GeneralTooltip03
              content={fieldMessagesFor(v.key)}
              label={fieldState(v.key) === "error" ? "查看错误" : "查看警告"}
            >
              {#snippet icon()}
                {#if fieldState(v.key) === "error"}
                  <CircleX class="h-4 w-4 text-destructive" aria-hidden="true" />
                {:else}
                  <TriangleAlert
                    class="h-4 w-4 text-amber-600"
                    aria-hidden="true"
                  />
                {/if}
              {/snippet}
            </GeneralTooltip03>
          </div>
        {/if}
      </div>

      {#if v.type === "computed"}
        <div class="computed">{computedText(v.key, v.unit)}</div>
      {:else if v.type === "table"}
        <TableEditor
          variable={v}
          {project}
          onchange={() => syncComplex(v.key)}
        />
      {:else if v.type === "image" || v.type === "images"}
        <ImageEditor
          variable={v}
          {project}
          {projectId}
          onchange={() => syncComplex(v.key)}
        />
      {:else if v.type === "text"}
        <textarea
          rows="3"
          placeholder={v.placeholder ?? ""}
          value={String(get(v.key) ?? "")}
          onfocus={(e) => openRecentMenu(v.key, e.currentTarget)}
          oninput={(e) => {
            set(v.key, e.currentTarget.value);
            updateRecentMenu(v.key, e.currentTarget);
          }}
          onkeydown={handleRecentKeydown}
          onblur={scheduleCloseRecentMenu}
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
          oninput={(e) => scheduleDateCheck(v.key, e)}
          onchange={(e) => setDate(v.key, e)}
          onblur={(e) => checkDateOnBlur(v.key, e)}
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
            onfocus={(e) => openRecentMenu(v.key, e.currentTarget)}
            oninput={(e) => {
              set(v.key, toNum(e));
              updateRecentMenu(v.key, e.currentTarget);
            }}
            onkeydown={handleRecentKeydown}
            onblur={scheduleCloseRecentMenu}
          />
          {#if v.unit}<span class="unit">{v.unit}</span>{/if}
        </div>
      {:else}
        <input
          type="text"
          placeholder={v.placeholder ?? ""}
          value={String(get(v.key) ?? "")}
          onfocus={(e) => openRecentMenu(v.key, e.currentTarget)}
          oninput={(e) => {
            set(v.key, e.currentTarget.value);
            updateRecentMenu(v.key, e.currentTarget);
          }}
          onkeydown={handleRecentKeydown}
          onblur={scheduleCloseRecentMenu}
        />
      {/if}

      {#if v.help}<span class="help">{v.help}</span>{/if}
    </div>
  {/each}

  <RecentValueMenu
    open={!!recentMenu}
    left={recentMenu?.left ?? 0}
    top={recentMenu?.top ?? 0}
    width={recentMenu?.width ?? 240}
    items={recentMenu?.items ?? []}
    activeIndex={recentMenu?.activeIndex ?? -1}
    onpick={chooseRecent}
    onhover={(index) => {
      if (recentMenu) recentMenu = { ...recentMenu, activeIndex: index };
    }}
  />
</div>

<style>
  .editor {
    padding: 24px;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 18px 28px;
    align-content: start;
  }
  .validation-global {
    grid-column: 1 / -1;
    padding: 10px 12px;
    border: 1px solid rgba(220, 38, 38, 0.25);
    border-radius: 8px;
    background: rgba(220, 38, 38, 0.06);
    color: var(--danger, #dc2626);
    font-size: 12px;
    line-height: 1.6;
  }
  .validation-global .global-warn {
    color: #b45309;
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
  .field.invalid input,
  .field.invalid textarea,
  .field.invalid select {
    box-shadow: 0 0 0 1px var(--danger, #dc2626);
  }
  .field.warn input,
  .field.warn textarea,
  .field.warn select {
    box-shadow: 0 0 0 1px #d97706;
  }
  .label {
    display: flex;
    align-items: center;
    gap: 3px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
  }
  .field-status {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
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
