<script lang="ts">
  import { Menubar as MenubarPrimitive } from "bits-ui";
  import CheckIcon from "@lucide/svelte/icons/check";
  import { cn } from "$lib/utils.js";
  import type { Snippet } from "svelte";

  let {
    class: className,
    inset,
    checked = $bindable(false),
    children: content,
    ...restProps
  }: Omit<MenubarPrimitive.CheckboxItemProps, "children"> & {
    inset?: boolean;
    children?: Snippet;
  } = $props();
</script>

<MenubarPrimitive.CheckboxItem
  bind:checked
  class={cn(
    "data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground relative flex cursor-default items-center rounded-sm py-1.5 pr-2 pl-8 text-sm outline-hidden select-none data-disabled:pointer-events-none data-disabled:opacity-50",
    inset && "pl-8",
    className,
  )}
  {...restProps}
>
  {#snippet children({ checked })}
    <span
      class="pointer-events-none absolute left-2 flex size-3.5 items-center justify-center"
    >
      {#if checked}<CheckIcon class="size-4" />{/if}
    </span>
    {@render content?.()}
  {/snippet}
</MenubarPrimitive.CheckboxItem>
