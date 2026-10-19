<script lang="ts">
  import { cn } from "$lib/utils.js";
  import type { HTMLAttributes } from "svelte/elements";

  let {
    class: className,
    orientation = "horizontal",
    children,
    ...restProps
  }: HTMLAttributes<HTMLDivElement> & {
    orientation?: "horizontal" | "vertical";
  } = $props();
</script>

<div
  data-slot="button-group"
  data-orientation={orientation}
  role="group"
  class={cn(
    "inline-flex items-stretch",
    orientation === "vertical"
      ? "flex-col [&>[data-slot=button]]:rounded-none [&>[data-slot=button]:first-child]:rounded-t-md [&>[data-slot=button]:last-child]:rounded-b-md [&>[data-slot=button]+[data-slot=button]]:-mt-px"
      : "flex-row [&>[data-slot=button]]:rounded-none [&>[data-slot=button]:first-child]:rounded-l-md [&>[data-slot=button]:last-child]:rounded-r-md [&>[data-slot=button]+[data-slot=button]]:-ml-px",
    className,
  )}
  {...restProps}
>
  {@render children?.()}
</div>
