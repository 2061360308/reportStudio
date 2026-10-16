<script lang="ts">
  import { Menubar as MenubarPrimitive } from "bits-ui";
  import { cn } from "$lib/utils.js";

  let { class: className, ...restProps }: MenubarPrimitive.GroupProps = $props();
</script>

<MenubarPrimitive.Group class={cn(className)} {...restProps} />
