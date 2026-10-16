<script lang="ts">
  import { Menubar as MenubarPrimitive } from "bits-ui";
  import { cn } from "$lib/utils.js";

  let { class: className, ...restProps }: MenubarPrimitive.TriggerProps =
    $props();
</script>

<MenubarPrimitive.Trigger
  class={cn(
    "hover:bg-accent hover:text-accent-foreground focus-visible:bg-accent focus-visible:text-accent-foreground data-[state=open]:bg-accent data-[state=open]:text-accent-foreground flex cursor-default items-center rounded-sm px-2 py-1 text-sm font-medium outline-hidden select-none disabled:pointer-events-none disabled:opacity-50",
    className,
  )}
  {...restProps}
/>
