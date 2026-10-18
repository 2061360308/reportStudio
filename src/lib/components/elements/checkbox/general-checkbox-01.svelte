<script lang="ts">
  import { Checkbox } from "$lib/components/elements/checkbox";
  import { Label } from "$lib/components/elements/label";
  import { cn } from "$lib/utils.js";

  const fallbackId = $props.id();

  let {
    id = fallbackId,
    checked = $bindable(false),
    label,
    disabled = false,
    class: className,
  }: {
    id?: string;
    checked?: boolean;
    label: string;
    disabled?: boolean;
    class?: string;
  } = $props();
</script>

<div class={cn("flex items-center gap-2", className)}>
  <Checkbox {id} bind:checked {disabled} />
  <Label for={id} class="cursor-pointer text-xs font-normal text-foreground">
    {label}
  </Label>
</div>
