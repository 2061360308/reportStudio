<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import Handsontable from "handsontable";
  import "handsontable/dist/handsontable.full.css";
  import type { ColumnDef, Project, VariableDef } from "./types";

  let {
    variable,
    project,
    onchange,
  }: {
    variable: VariableDef;
    project: Project;
    onchange?: () => void;
  } = $props();

  let container: HTMLDivElement;
  let hot: Handsontable | null = null;

  const cols: ColumnDef[] = $derived(variable.columns ?? []);

  function hotType(t: ColumnDef["type"]): string {
    switch (t) {
      case "number":
      case "integer":
      case "money":
      case "percent":
        return "numeric";
      case "date":
        return "date";
      case "bool":
        return "checkbox";
      case "select":
        return "dropdown";
      default:
        return "text";
    }
  }

  function sync() {
    if (!hot) return;
    project.values[variable.key] = hot.getSourceData();
    onchange?.();
  }

  onMount(() => {
    if (!Array.isArray(project.values[variable.key])) {
      project.values[variable.key] = [];
    }
    hot = new Handsontable(container, {
      data: project.values[variable.key] as Record<string, unknown>[],
      colHeaders: cols.map((c) => c.label ?? c.alias ?? c.key),
      columns: cols.map((c) => ({
        data: c.key,
        type: hotType(c.type),
        readOnly: c.type === "computed",
        source: c.options ? c.options.map((o) => o.value) : undefined,
        numericFormat:
          c.type === "money" ? { pattern: "0,0.00" } : undefined,
        className: c.type === "computed" ? "htComputed" : undefined,
      })),
      rowHeaders: true,
      height: 220,
      width: "100%",
      stretchH: "all",
      licenseKey: "non-commercial-and-evaluation",
      afterChange: sync,
      afterCreateRow: sync,
      afterRemoveRow: sync,
    });
  });

  onDestroy(() => hot?.destroy());

  function addRow() {
    if (!hot) return;
    const data = hot.getSourceData() as Record<string, unknown>[];
    data.push({});
    hot.loadData(data);
    sync();
  }

  function removeRow() {
    if (!hot) return;
    const sel = hot.getSelectedLast();
    if (!sel) return;
    hot.alter("remove_row", sel[0], 1);
    sync();
  }
</script>

<div class="tbl">
  <div class="bar">
    <button type="button" onclick={addRow}>+ 添加行</button>
    <button type="button" onclick={removeRow}>删除选中行</button>
  </div>
  <div bind:this={container} class="hot"></div>
</div>

<style>
  .tbl {
    margin: 6px 0 8px;
  }
  .bar {
    display: flex;
    gap: 8px;
    margin-bottom: 10px;
  }
  .bar button {
    padding: 5px 12px;
    font-size: 12px;
  }
  .hot {
    max-width: 100%;
  }
  :global(.htComputed) {
    background: var(--surface-100) !important;
  }
</style>
