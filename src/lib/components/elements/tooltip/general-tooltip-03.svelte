<script lang="ts">
  import { Tooltip } from "bits-ui";
  import CircleHelp from "@lucide/svelte/icons/circle-help";
  import type { Snippet } from "svelte";
  import { Button } from "$lib/components/elements/button";
  import { cn } from "$lib/utils.js";

  let {
    open = $bindable(false),
    content = "",
    label = "查看提示",
    side = "top",
    icon,
    class: className,
  }: {
    open?: boolean;
    content?: string;
    label?: string;
    side?: "top" | "right" | "bottom" | "left";
    icon?: Snippet;
    class?: string;
  } = $props();
</script>

<Tooltip.Provider delayDuration={150}>
  <Tooltip.Root bind:open>
    <Tooltip.Trigger>
      {#snippet child({ props })}
        <Button
          {...props}
          type="button"
          variant="ghost"
          size="icon-sm"
          class={cn("text-muted-foreground hover:text-foreground", className)}
        >
          {#if icon}
            {@render icon()}
          {:else}
            <CircleHelp class="h-4 w-4" aria-hidden="true" />
          {/if}
          <span class="sr-only">{label}</span>
        </Button>
      {/snippet}
    </Tooltip.Trigger>

    <Tooltip.Portal>
      <Tooltip.Content
        {side}
        sideOffset={6}
        class="z-[200] max-w-64 whitespace-pre-line rounded-md bg-foreground px-3 py-2 text-xs leading-relaxed text-background shadow-md"
      >
        {content}
        <Tooltip.Arrow class="text-foreground" />
      </Tooltip.Content>
    </Tooltip.Portal>
  </Tooltip.Root>
</Tooltip.Provider>
