<script lang="ts">
  import { cn } from "$lib/utils.js";
  import type { HTMLAttributes } from "svelte/elements";

  let {
    class: className,
    children,
    ...restProps
  }: HTMLAttributes<HTMLSpanElement> = $props();
</script>

<span
  class={cn("text-muted-foreground ml-auto text-xs tracking-widest", className)}
  {...restProps}
>
  {@render children?.()}
</span>
