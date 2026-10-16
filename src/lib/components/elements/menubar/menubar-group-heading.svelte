<script lang="ts">
  import { Menubar as MenubarPrimitive } from "bits-ui";
  import { cn } from "$lib/utils.js";

  let {
    class: className,
    ...restProps
  }: MenubarPrimitive.GroupHeadingProps = $props();
</script>

<MenubarPrimitive.GroupHeading
  class={cn("text-foreground px-2 py-1.5 text-sm font-semibold", className)}
  {...restProps}
/>
