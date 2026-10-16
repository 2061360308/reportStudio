<script lang="ts">
  import { Menubar as MenubarPrimitive } from "bits-ui";
  import { cn } from "$lib/utils.js";

  let {
    class: className,
    sideOffset = 4,
    ...restProps
  }: MenubarPrimitive.ContentProps = $props();
</script>

<MenubarPrimitive.Content
  {sideOffset}
  class={cn(
    "bg-popover text-popover-foreground z-50 min-w-[12rem] overflow-hidden rounded-md border p-1 shadow-md",
    className,
  )}
  {...restProps}
/>
