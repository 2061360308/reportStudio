<script lang="ts">
  import { Dialog } from "bits-ui";
  import Trash from "@lucide/svelte/icons/trash";
  import X from "@lucide/svelte/icons/x";
  import { Button } from "$lib/components/elements/button";

  let {
    open = $bindable(false),
    src = "",
    alt = "图片",
    onclose,
    onclear,
  }: {
    open?: boolean;
    src?: string;
    alt?: string;
    onclose?: () => void;
    onclear?: () => void;
  } = $props();

  function handleOpenChange(next: boolean) {
    open = next;
    if (!next) onclose?.();
  }

  function close() {
    open = false;
    onclose?.();
  }

  function clear() {
    onclear?.();
    close();
  }
</script>

<Dialog.Root bind:open onOpenChange={handleOpenChange}>
  <Dialog.Portal>
    <Dialog.Overlay class="fixed inset-0 z-[160] bg-black/80 backdrop-blur-sm" />
    <Dialog.Content
      class="fixed left-1/2 top-1/2 z-[161] flex max-h-[92vh] max-w-[92vw] -translate-x-1/2 -translate-y-1/2 flex-col outline-none"
      tabindex={-1}
    >
      <Dialog.Title class="sr-only">{alt}</Dialog.Title>
      <Dialog.Description class="sr-only">图片预览</Dialog.Description>

      <div class="relative flex items-center justify-center">
        <img
          {src}
          {alt}
          draggable="false"
          class="max-h-[88vh] max-w-[88vw] rounded-md object-contain shadow-2xl"
        />
        <div class="absolute right-3 top-3 flex items-center gap-2">
          {#if onclear}
            <Button
              type="button"
              variant="secondary"
              size="sm"
              onclick={clear}
            >
              <Trash class="h-4 w-4" aria-hidden="true" />
              清除
            </Button>
          {/if}
          <Button
            type="button"
            variant="secondary"
            size="icon-sm"
            onclick={close}
            aria-label="关闭图片预览"
          >
            <X class="h-4 w-4" aria-hidden="true" />
          </Button>
        </div>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
