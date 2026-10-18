<script lang="ts">
  import { cn } from "$lib/utils.js";
  import type { HTMLLabelAttributes } from "svelte/elements";
  import type { Snippet } from "svelte";

  let {
    class: className,
    children,
    ...restProps
  }: HTMLLabelAttributes & { children?: Snippet } = $props();
</script>

<label
  data-slot="label"
  class={cn(
    "flex items-center gap-2 text-sm font-medium leading-none select-none",
    className,
  )}
  {...restProps}
>
  {@render children?.()}
</label>
