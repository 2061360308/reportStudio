<script lang="ts">
  import { Menubar as MenubarPrimitive } from "bits-ui";
  import { cn } from "$lib/utils.js";

  let {
    class: className,
    sideOffset = 0,
    alignOffset = -4,
    ...restProps
  }: MenubarPrimitive.SubContentProps = $props();
</script>

<MenubarPrimitive.SubContent
  {sideOffset}
  {alignOffset}
  class={cn(
    "bg-popover text-popover-foreground z-50 min-w-[8rem] overflow-hidden rounded-md border p-1 shadow-lg",
    className,
  )}
  {...restProps}
/>
