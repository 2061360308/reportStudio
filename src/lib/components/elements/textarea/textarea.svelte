<script lang="ts">
  import { cn } from "$lib/utils.js";
  import type { HTMLTextareaAttributes } from "svelte/elements";

  let {
    class: className,
    value = $bindable(""),
    ...restProps
  }: HTMLTextareaAttributes = $props();
</script>

<textarea
  data-slot="textarea"
  bind:value
  class={cn(
    "flex min-h-16 w-full rounded-md border border-input bg-background px-3 py-2 text-sm shadow-xs outline-none transition-[color,box-shadow] placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50",
    className,
  )}
  {...restProps}
></textarea>
