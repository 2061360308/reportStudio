<script lang="ts">
  import { PaneResizer, type PaneResizerProps } from "paneforge";
  import GripVertical from "@lucide/svelte/icons/grip-vertical";
  import { cn } from "$lib/utils.js";

  let {
    class: className,
    withHandle = false,
    ...restProps
  }: PaneResizerProps & { withHandle?: boolean } = $props();
</script>

<PaneResizer
  class={cn(
    "relative flex w-px items-center justify-center bg-border outline-none after:absolute after:inset-y-0 after:left-1/2 after:w-1 after:-translate-x-1/2 hover:after:bg-ring/40 focus-visible:ring-2 focus-visible:ring-ring/50 data-[panel-group-direction=vertical]:h-px data-[panel-group-direction=vertical]:w-full data-[panel-group-direction=vertical]:after:left-0 data-[panel-group-direction=vertical]:after:h-1 data-[panel-group-direction=vertical]:after:w-full data-[panel-group-direction=vertical]:after:-translate-y-1/2 data-[panel-group-direction=vertical]:after:translate-x-0 [&[data-panel-group-direction=vertical]>div]:rotate-90",
    className,
  )}
  {...restProps}
>
  {#if withHandle}
    <div
      class="z-10 flex h-4 w-3 items-center justify-center rounded-sm border bg-border text-muted-foreground"
    >
      <GripVertical class="size-2.5" />
    </div>
  {/if}
</PaneResizer>
