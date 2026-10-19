<script lang="ts">
  import { cn } from "$lib/utils.js";
  import type { HTMLAttributes } from "svelte/elements";

  let {
    class: className,
    children,
    ...restProps
  }: HTMLAttributes<HTMLDivElement> = $props();
</script>

<div
  data-slot="button-group-text"
  class={cn(
    "inline-flex items-center px-2 text-xs text-muted-foreground",
    className,
  )}
  {...restProps}
>
  {@render children?.()}
</div>
