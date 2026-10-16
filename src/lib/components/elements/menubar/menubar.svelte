<script lang="ts">
  import { Menubar as MenubarPrimitive } from "bits-ui";
  import { cn } from "$lib/utils.js";

  let { class: className, ...restProps }: MenubarPrimitive.RootProps = $props();
</script>

<MenubarPrimitive.Root
  class={cn(
    "bg-background flex h-9 items-center gap-0.5 rounded-md border p-1 shadow-xs",
    className,
  )}
  {...restProps}
/>
