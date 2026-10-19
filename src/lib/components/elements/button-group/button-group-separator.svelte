<script lang="ts">
  import { cn } from "$lib/utils.js";
  import type { HTMLAttributes } from "svelte/elements";

  let {
    class: className,
    orientation = "vertical",
    ...restProps
  }: HTMLAttributes<HTMLDivElement> & {
    orientation?: "horizontal" | "vertical";
  } = $props();
</script>

<div
  data-slot="button-group-separator"
  role="separator"
  aria-orientation={orientation}
  class={cn(
    "bg-border",
    orientation === "vertical" ? "w-px self-stretch" : "h-px w-full",
    className,
  )}
  {...restProps}
></div>
