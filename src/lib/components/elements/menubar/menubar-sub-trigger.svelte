<script lang="ts">
  import { Menubar as MenubarPrimitive } from "bits-ui";
  import ChevronRightIcon from "@lucide/svelte/icons/chevron-right";
  import { cn } from "$lib/utils.js";

  let {
    class: className,
    inset,
    children: content,
    ...restProps
  }: MenubarPrimitive.SubTriggerProps & { inset?: boolean } = $props();
</script>

<MenubarPrimitive.SubTrigger
  class={cn(
    "data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground data-[state=open]:bg-accent data-[state=open]:text-accent-foreground flex cursor-default items-center rounded-sm px-2 py-1.5 text-sm outline-hidden select-none data-disabled:pointer-events-none data-disabled:opacity-50",
    inset && "pl-8",
    className,
  )}
  {...restProps}
>
  {@render content?.()}
  <ChevronRightIcon class="ml-auto size-4" />
</MenubarPrimitive.SubTrigger>
