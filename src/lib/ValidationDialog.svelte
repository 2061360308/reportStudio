<script lang="ts">
  import CircleX from "@lucide/svelte/icons/circle-x";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import { GeneralDialog17 } from "$lib/components/elements/dialog";
  import { Button } from "$lib/components/elements/button";
  import type { ValidationIssue, VariableDef } from "./types";

  let {
    issues = [],
    variables = [],
    actionLabel = "",
    onlocate,
    onproceed,
    onclose,
  }: {
    issues?: ValidationIssue[];
    variables?: VariableDef[];
    actionLabel?: string;
    onlocate?: (key: string) => void;
    onproceed?: () => void;
    onclose?: () => void;
  } = $props();

  let open = $state(true);
  let finished = false;

  const visibleIssues = $derived(
    [...issues]
      .filter((issue) => issue.status !== "ok")
      .sort(
        (a, b) =>
          Number(b.status === "error") - Number(a.status === "error"),
      ),
  );
  const errorCount = $derived(
    visibleIssues.filter((issue) => issue.status === "error").length,
  );
  const warningCount = $derived(
    visibleIssues.filter((issue) => issue.status === "warning").length,
  );
  const hasErrors = $derived(errorCount > 0);
  const title = $derived(hasErrors ? "校验未通过" : "存在校验警告");
  const description = $derived(
    hasErrors
      ? "请先修正以下问题，再导出 PDF 或打印。"
      : "以下问题不会阻止导出，请确认是否继续。",
  );

  function fieldLabel(keys: string[]): string {
    if (!keys.length) return "表单";
    return keys
      .map((key) => {
        const variable = variables.find((item) => item.key === key);
        return variable?.label ?? variable?.alias ?? key;
      })
      .join("、");
  }

  function issueKey(issue: ValidationIssue): string | undefined {
    return issue.key ?? issue.keys[0];
  }

  function finish(
    reason: "close" | "proceed" | "locate",
    key?: string,
  ) {
    if (finished) return;
    finished = true;
    open = false;

    if (reason === "proceed") onproceed?.();
    else if (reason === "locate" && key) onlocate?.(key);
    else onclose?.();
  }

  function close() {
    finish("close");
  }

  function proceed() {
    finish("proceed");
  }

  function locate(issue: ValidationIssue) {
    finish("locate", issueKey(issue));
  }
</script>

<GeneralDialog17
  bind:open
  {title}
  {description}
  onclose={close}
  class="max-w-xl"
>
  {#snippet icon()}
    {#if hasErrors}
      <CircleX class="h-5 w-5 text-destructive" aria-hidden="true" />
    {:else}
      <TriangleAlert class="h-5 w-5 text-amber-600" aria-hidden="true" />
    {/if}
  {/snippet}

  <div class="flex min-h-0 flex-1 flex-col">
    <div
      class="flex flex-wrap items-center gap-3 border-b border-border bg-muted/30 px-6 py-3 text-xs"
    >
      {#if errorCount}
        <span class="font-semibold text-destructive">{errorCount} 个错误</span>
      {/if}
      {#if warningCount}
        <span class="font-semibold text-amber-600">{warningCount} 个警告</span>
      {/if}
    </div>

    <div class="min-h-0 flex-1 overflow-y-auto px-6 py-4">
      <div class="space-y-2">
        {#each visibleIssues as issue, i (i)}
          {@const key = issueKey(issue)}
          <button
            type="button"
            class="flex w-full items-start gap-3 rounded-md border border-border bg-background px-3 py-2.5 text-left transition-colors hover:bg-accent disabled:cursor-default disabled:hover:bg-background"
            disabled={!key}
            onclick={() => locate(issue)}
          >
            <span class="mt-0.5 shrink-0">
              {#if issue.status === "error"}
                <CircleX class="h-4 w-4 text-destructive" aria-hidden="true" />
              {:else}
                <TriangleAlert
                  class="h-4 w-4 text-amber-600"
                  aria-hidden="true"
                />
              {/if}
            </span>
            <span class="min-w-0 flex-1">
              <span
                class="block text-[11px] font-medium text-muted-foreground"
              >
                {fieldLabel(issue.keys)}
              </span>
              <span class="mt-0.5 block text-sm leading-relaxed text-foreground">
                {issue.msg}
              </span>
            </span>
          </button>
        {/each}
      </div>
    </div>

    <div
      class="flex flex-wrap items-center justify-end gap-2 border-t border-border px-6 py-4"
    >
      <Button type="button" variant="outline" onclick={close} autofocus>
        返回修改
      </Button>
      {#if !hasErrors && actionLabel}
        <Button type="button" onclick={proceed}>{actionLabel}</Button>
      {/if}
    </div>
  </div>
</GeneralDialog17>
