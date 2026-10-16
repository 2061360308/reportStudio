<script lang="ts">
  import { cn } from "$lib/utils.js";
  import type { HTMLAttributes } from "svelte/elements";

  let {
    class: className,
    children,
    ...restProps
  }: HTMLAttributes<HTMLParagraphElement> = $props();
</script>

<p
  data-slot="empty-description"
  class={cn("text-muted-foreground text-sm", className)}
  {...restProps}
>
  {@render children?.()}
</p>
