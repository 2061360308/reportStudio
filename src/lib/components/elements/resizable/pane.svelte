<script lang="ts">
  import { Pane, type PaneProps } from "paneforge";
  import { cn } from "$lib/utils.js";

  let { class: className, ...restProps }: PaneProps = $props();
</script>

<Pane class={cn("relative overflow-hidden", className)} {...restProps} />
