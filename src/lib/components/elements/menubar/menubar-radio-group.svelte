<script lang="ts">
  import { Menubar as MenubarPrimitive } from "bits-ui";
  import { cn } from "$lib/utils.js";

  let {
    class: className,
    value = $bindable(""),
    ...restProps
  }: MenubarPrimitive.RadioGroupProps = $props();
</script>

<MenubarPrimitive.RadioGroup bind:value class={cn(className)} {...restProps} />
