<script lang="ts">
  import X from "@lucide/svelte/icons/x";
  import CircleCheck from "@lucide/svelte/icons/circle-check";
  import CircleX from "@lucide/svelte/icons/circle-x";
  import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
  import Info from "@lucide/svelte/icons/info";
  import type { Snippet } from "svelte";
  import { Button } from "$lib/components/elements/button";
  import { cn } from "$lib/utils.js";

  type Tone = "success" | "error" | "warning" | "info";

  let {
    visible = $bindable(true),
    title,
    description = "",
    tone = "success",
    primaryLabel = "",
    secondaryLabel = "",
    onprimary,
    onsecondary,
    ondismiss,
    icon,
    class: className,
  }: {
    visible?: boolean;
    title: string;
    description?: string;
    tone?: Tone;
    primaryLabel?: string;
    secondaryLabel?: string;
    onprimary?: () => void;
    onsecondary?: () => void;
    ondismiss?: () => void;
    icon?: Snippet;
    class?: string;
  } = $props();

  const iconClass = $derived(
    tone === "error"
      ? "text-destructive"
      : tone === "warning"
        ? "text-amber-600 dark:text-amber-400"
        : tone === "info"
          ? "text-sky-600 dark:text-sky-400"
          : "text-green-600 dark:text-green-400",
  );

  function dismiss() {
    visible = false;
    ondismiss?.();
  }
</script>

{#if visible}
  <div
    role={tone === "error" ? "alert" : "status"}
    aria-live="polite"
    class={cn(
      "w-full rounded-lg border border-border bg-card p-4 text-card-foreground shadow-sm",
      className,
    )}
  >
    <div class="flex items-start gap-3">
      <span class="mt-0.5 shrink-0 {iconClass}">
        {#if icon}
          {@render icon()}
        {:else if tone === "error"}
          <CircleX class="h-5 w-5" aria-hidden="true" />
        {:else if tone === "warning"}
          <TriangleAlert class="h-5 w-5" aria-hidden="true" />
        {:else if tone === "info"}
          <Info class="h-5 w-5" aria-hidden="true" />
        {:else}
          <CircleCheck class="h-5 w-5" aria-hidden="true" />
        {/if}
      </span>

      <div class="min-w-0 flex-1">
        <p class="text-sm font-semibold leading-none">{title}</p>
        {#if description}
          <p
            class="mt-1.5 whitespace-pre-line text-sm leading-relaxed text-muted-foreground"
          >
            {description}
          </p>
        {/if}

        {#if primaryLabel || secondaryLabel}
          <div class="mt-3 flex flex-wrap items-center gap-2">
            {#if primaryLabel}
              <Button type="button" size="sm" onclick={onprimary}>
                {primaryLabel}
              </Button>
            {/if}
            {#if secondaryLabel}
              <Button
                type="button"
                variant="outline"
                size="sm"
                onclick={onsecondary}
              >
                {secondaryLabel}
              </Button>
            {/if}
          </div>
        {/if}
      </div>

      <Button
        variant="ghost"
        size="icon-sm"
        class="-mr-1 -mt-1 shrink-0 text-muted-foreground hover:text-foreground"
        onclick={dismiss}
      >
        <X aria-hidden="true" />
        <span class="sr-only">关闭通知</span>
      </Button>
    </div>
  </div>
{/if}
