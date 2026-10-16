<script lang="ts">
  import { PaneGroup, type PaneGroupProps } from "paneforge";
  import { cn } from "$lib/utils.js";

  let { class: className, ...restProps }: PaneGroupProps = $props();
</script>

<PaneGroup
  class={cn(
    "flex h-full w-full data-[panel-group-direction=vertical]:flex-col",
    className,
  )}
  {...restProps}
/>
