<script lang="ts">
  import { Dialog } from "bits-ui";
  import Trash from "@lucide/svelte/icons/trash";
  import X from "@lucide/svelte/icons/x";
  import ZoomIn from "@lucide/svelte/icons/zoom-in";
  import ZoomOut from "@lucide/svelte/icons/zoom-out";
  import Maximize2 from "@lucide/svelte/icons/maximize-2";
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

  const MIN_ZOOM = 1;
  const MAX_ZOOM = 8;

  let stage = $state<HTMLDivElement | undefined>();
  let img = $state<HTMLImageElement | undefined>();
  let zoom = $state(1);
  let tx = $state(0);
  let ty = $state(0);
  let dragging = $state(false);
  let lastX = 0;
  let lastY = 0;

  function clamp(value: number, min: number, max: number) {
    return Math.min(max, Math.max(min, value));
  }

  function clampPan() {
    if (!stage || !img) {
      tx = 0;
      ty = 0;
      return;
    }
    const maxX = Math.max(
      0,
      (img.offsetWidth * zoom - stage.clientWidth) / 2,
    );
    const maxY = Math.max(
      0,
      (img.offsetHeight * zoom - stage.clientHeight) / 2,
    );
    tx = clamp(tx, -maxX, maxX);
    ty = clamp(ty, -maxY, maxY);
  }

  function resetZoom() {
    zoom = 1;
    tx = 0;
    ty = 0;
  }

  function applyZoom(next: number, clientX?: number, clientY?: number) {
    if (!stage) return;
    const value = clamp(next, MIN_ZOOM, MAX_ZOOM);
    if (value === zoom) return;

    const rect = stage.getBoundingClientRect();
    const px =
      (clientX ?? rect.left + rect.width / 2) - rect.left - rect.width / 2;
    const py =
      (clientY ?? rect.top + rect.height / 2) - rect.top - rect.height / 2;
    const ratio = value / zoom;

    tx = px - (px - tx) * ratio;
    ty = py - (py - ty) * ratio;
    zoom = value;
    clampPan();
  }

  function onWheel(event: WheelEvent) {
    event.preventDefault();
    if (!event.deltaY) return;
    const delta =
      event.deltaMode === 1
        ? event.deltaY * 16
        : event.deltaMode === 2
          ? event.deltaY * 100
          : event.deltaY;
    applyZoom(zoom * Math.exp(-delta * 0.0015), event.clientX, event.clientY);
  }

  $effect(() => {
    const el = stage;
    if (!el) return;
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  });

  function onPointerDown(event: PointerEvent) {
    if (event.button !== 0 || zoom <= MIN_ZOOM || !stage) return;
    const target = event.target as HTMLElement | null;
    if (target?.closest("button, a, input")) return;
    dragging = true;
    lastX = event.clientX;
    lastY = event.clientY;
    stage.setPointerCapture(event.pointerId);
    event.preventDefault();
  }

  function onPointerMove(event: PointerEvent) {
    if (!dragging) return;
    tx += event.clientX - lastX;
    ty += event.clientY - lastY;
    lastX = event.clientX;
    lastY = event.clientY;
    clampPan();
  }

  function onPointerUp(event: PointerEvent) {
    if (!dragging) return;
    dragging = false;
    if (stage?.hasPointerCapture(event.pointerId)) {
      stage.releasePointerCapture(event.pointerId);
    }
  }

  $effect(() => {
    if (open) {
      void src;
      resetZoom();
    }
  });

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

<svelte:window onresize={clampPan} />

<Dialog.Root bind:open onOpenChange={handleOpenChange}>
  <Dialog.Portal>
    <Dialog.Overlay class="fixed inset-0 z-[160] bg-black/80 backdrop-blur-sm" />
    <Dialog.Content
      class="fixed left-1/2 top-1/2 z-[161] flex -translate-x-1/2 -translate-y-1/2 flex-col outline-none"
      tabindex={-1}
    >
      <Dialog.Title class="sr-only">{alt}</Dialog.Title>
      <Dialog.Description class="sr-only">
        图片预览，可使用滚轮缩放，放大后拖拽平移
      </Dialog.Description>

      <div
        bind:this={stage}
        class="relative flex h-[88vh] w-[92vw] touch-none select-none items-center justify-center overflow-hidden"
        class:cursor-grab={zoom > MIN_ZOOM && !dragging}
        class:cursor-grabbing={dragging}
        role="presentation"
        onpointerdown={onPointerDown}
        onpointermove={onPointerMove}
        onpointerup={onPointerUp}
        onpointercancel={onPointerUp}
      >
        <img
          bind:this={img}
          {src}
          {alt}
          draggable="false"
          class="max-h-full max-w-full rounded-md object-contain shadow-2xl will-change-transform"
          style:transform={`translate(${tx}px, ${ty}px) scale(${zoom})`}
          onload={clampPan}
        />
        <div class="absolute right-3 top-3 z-10 flex items-center gap-2">
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

        <div
          class="absolute bottom-4 left-1/2 z-10 flex -translate-x-1/2 items-center gap-1 rounded-full bg-black/60 px-2 py-1 text-white shadow-lg backdrop-blur-sm"
        >
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            class="bg-transparent text-white hover:bg-white/15 hover:text-white"
            onclick={() => applyZoom(zoom / 1.25)}
            disabled={zoom <= MIN_ZOOM}
            aria-label="缩小"
          >
            <ZoomOut class="h-4 w-4" aria-hidden="true" />
          </Button>
          <span class="min-w-11 text-center text-xs tabular-nums text-white/90">
            {Math.round(zoom * 100)}%
          </span>
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            class="bg-transparent text-white hover:bg-white/15 hover:text-white"
            onclick={() => applyZoom(zoom * 1.25)}
            disabled={zoom >= MAX_ZOOM}
            aria-label="放大"
          >
            <ZoomIn class="h-4 w-4" aria-hidden="true" />
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            class="bg-transparent text-white hover:bg-white/15 hover:text-white"
            onclick={resetZoom}
            aria-label="适应窗口"
          >
            <Maximize2 class="h-4 w-4" aria-hidden="true" />
          </Button>
        </div>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
