<script lang="ts">
  import { cn } from "$lib/utils.js";
  import type { HTMLInputAttributes } from "svelte/elements";

  let {
    class: className,
    value = $bindable(""),
    ...restProps
  }: HTMLInputAttributes = $props();
</script>

<input
  data-slot="input"
  bind:value
  class={cn(
    "flex h-9 w-full min-w-0 rounded-md border border-input bg-background px-3 py-1 text-sm shadow-xs outline-none transition-[color,box-shadow] placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50",
    className,
  )}
  {...restProps}
/>
