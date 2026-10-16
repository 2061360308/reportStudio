<script lang="ts">
  import { cn } from "$lib/utils.js";
  import type { HTMLAttributes } from "svelte/elements";

  let {
    class: className,
    variant = "default",
    children,
    ...restProps
  }: HTMLAttributes<HTMLDivElement> & {
    variant?: "default" | "icon";
  } = $props();
</script>

<div
  data-slot="empty-media"
  data-variant={variant}
  class={cn(
    "flex shrink-0 items-center justify-center",
    variant === "icon" &&
      "size-10 rounded-lg border bg-muted/50 [&_svg]:size-5 [&_svg]:text-muted-foreground",
    className,
  )}
  {...restProps}
>
  {@render children?.()}
</div>
