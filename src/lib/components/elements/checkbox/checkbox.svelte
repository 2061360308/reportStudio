<script lang="ts">
  import { Checkbox as CheckboxPrimitive } from "bits-ui";
  import Check from "@lucide/svelte/icons/check";
  import { cn } from "$lib/utils.js";

  let {
    class: className,
    checked = $bindable(false),
    ...restProps
  }: CheckboxPrimitive.RootProps = $props();
</script>

<CheckboxPrimitive.Root
  bind:checked
  class={cn(
    "inline-flex size-4 shrink-0 items-center justify-center rounded-[4px] border border-input bg-background p-0 text-primary-foreground shadow-xs outline-none transition-shadow focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50 data-[state=checked]:border-primary data-[state=checked]:bg-primary",
    className,
  )}
  {...restProps}
>
  {#if checked}
    <Check class="size-3.5" aria-hidden="true" />
  {/if}
</CheckboxPrimitive.Root>
