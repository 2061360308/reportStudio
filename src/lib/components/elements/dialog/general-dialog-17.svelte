<script lang="ts">
  import { Dialog } from "bits-ui";
  import X from "@lucide/svelte/icons/x";
  import type { Snippet } from "svelte";
  import { Button } from "$lib/components/elements/button";
  import { cn } from "$lib/utils.js";

  let {
    open = $bindable(false),
    title,
    description,
    onclose,
    dismissible = true,
    icon,
    children,
    class: className,
  }: {
    open?: boolean;
    title: string;
    description?: string;
    onclose?: () => void;
    dismissible?: boolean;
    icon?: Snippet;
    children?: Snippet;
    class?: string;
  } = $props();

  let contentRef = $state<HTMLDivElement | null>(null);

  function handleOpenChange(next: boolean) {
    if (!next && !dismissible) return;
    open = next;
    if (!next) onclose?.();
  }

  function handleOpenAutoFocus(event: Event) {
    event.preventDefault();
    requestAnimationFrame(() => {
      const target = contentRef?.querySelector<HTMLElement>("[autofocus]");
      (target ?? contentRef)?.focus();
    });
  }
</script>

<Dialog.Root {open} onOpenChange={handleOpenChange}>
  <Dialog.Portal>
    <Dialog.Overlay class="fixed inset-0 z-[129] bg-black/50" />
    <Dialog.Content
      bind:ref={contentRef}
      class={cn(
        "fixed left-1/2 top-1/2 z-[130] flex max-h-[90vh] w-[calc(100%-2rem)] max-w-lg -translate-x-1/2 -translate-y-1/2 flex-col overflow-hidden rounded-lg border border-border bg-background shadow-lg outline-none",
        className,
      )}
      tabindex={-1}
      onOpenAutoFocus={handleOpenAutoFocus}
      onEscapeKeydown={(event) => {
        if (!dismissible) event.preventDefault();
      }}
      onInteractOutside={(event) => {
        if (!dismissible) event.preventDefault();
      }}
    >
      <div class="flex items-start gap-3 border-b border-border px-6 py-5 pr-14">
        {#if icon}
          <div
            class="flex size-9 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground"
          >
            {@render icon()}
          </div>
        {/if}
        <div class="flex min-w-0 flex-col gap-1">
          <Dialog.Title
            class="text-lg font-semibold leading-none tracking-tight"
          >
            {title}
          </Dialog.Title>
          {#if description}
            <Dialog.Description class="text-sm text-muted-foreground">
              {description}
            </Dialog.Description>
          {/if}
        </div>
      </div>

      <Dialog.Close disabled={!dismissible}>
        {#snippet child({ props })}
          <Button
            {...props}
            type="button"
            variant="ghost"
            size="icon-sm"
            class="absolute right-4 top-4 text-muted-foreground"
            disabled={!dismissible}
          >
            <X class="h-4 w-4" aria-hidden="true" />
            <span class="sr-only">关闭弹窗</span>
          </Button>
        {/snippet}
      </Dialog.Close>

      {@render children?.()}
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
