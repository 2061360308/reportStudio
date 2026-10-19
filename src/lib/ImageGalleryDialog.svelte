<script lang="ts">
  import GripVertical from "@lucide/svelte/icons/grip-vertical";
  import Images from "@lucide/svelte/icons/images";
  import Maximize2 from "@lucide/svelte/icons/maximize-2";
  import Trash from "@lucide/svelte/icons/trash";
  import { GeneralDialog17 } from "$lib/components/elements/dialog";
  import { cn } from "$lib/utils.js";

  let {
    open = $bindable(false),
    title = "图片",
    images = [],
    urlFor,
    onclose,
    onremove,
    onreorder,
    onpreview,
  }: {
    open?: boolean;
    title?: string;
    images?: string[];
    urlFor: (name: string) => string;
    onclose?: () => void;
    onremove: (index: number) => void;
    onreorder: (from: number, to: number) => void;
    onpreview: (name: string) => void;
  } = $props();

  let dragFrom = $state<number | null>(null);
  let dragOver = $state<number | null>(null);

  function onDragStart(event: DragEvent, index: number) {
    dragFrom = index;
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = "move";
      event.dataTransfer.setData("text/plain", String(index));
    }
  }

  function onDragOver(event: DragEvent, index: number) {
    event.preventDefault();
    dragOver = index;
    if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
  }

  function onDrop(event: DragEvent, index: number) {
    event.preventDefault();
    const raw = dragFrom ?? Number(event.dataTransfer?.getData("text/plain"));
    const from = Number.isInteger(raw) ? raw : -1;
    if (from >= 0 && from !== index) onreorder(from, index);
    dragFrom = null;
    dragOver = null;
  }

  function onDragEnd() {
    dragFrom = null;
    dragOver = null;
  }
</script>

<GeneralDialog17
  bind:open
  {title}
  description="拖拽排序，点击缩略图放大，支持删除。"
  {onclose}
  class="h-[min(90vh,720px)] max-w-4xl"
>
  <div class="flex min-h-0 flex-1 flex-col">
    <div class="min-h-0 flex-1 overflow-y-auto px-6 py-5">
      {#if images.length === 0}
        <div
          class="flex h-full min-h-40 flex-col items-center justify-center gap-2 text-muted-foreground"
        >
          <Images class="h-8 w-8" aria-hidden="true" />
          <p class="text-sm">暂无图片</p>
        </div>
      {:else}
        <div
          class="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4"
          role="list"
        >
          {#each images as name, i (name)}
            <div
              class={cn(
                "group relative aspect-square cursor-grab overflow-hidden rounded-md border border-border bg-muted/40 transition-shadow active:cursor-grabbing",
                dragOver === i && "ring-2 ring-primary",
              )}
              role="listitem"
              draggable="true"
              ondragstart={(event) => onDragStart(event, i)}
              ondragover={(event) => onDragOver(event, i)}
              ondrop={(event) => onDrop(event, i)}
              ondragend={onDragEnd}
            >
              <button
                type="button"
                class="absolute inset-0 block overflow-hidden bg-transparent p-0 shadow-none"
                onclick={() => onpreview(name)}
                aria-label={`放大查看第 ${i + 1} 张图片`}
              >
                <img
                  src={urlFor(name)}
                  alt=""
                  draggable="false"
                  class="h-full w-full object-cover"
                />
                <span
                  class="absolute inset-0 flex items-center justify-center bg-black/0 opacity-0 transition group-hover:bg-black/30 group-hover:opacity-100"
                >
                  <Maximize2 class="h-5 w-5 text-white" aria-hidden="true" />
                </span>
              </button>

              <span
                class="pointer-events-none absolute left-1.5 top-1.5 rounded bg-black/55 p-1 text-white"
              >
                <GripVertical class="h-3.5 w-3.5" aria-hidden="true" />
              </span>
              <span
                class="pointer-events-none absolute bottom-1.5 left-1.5 rounded bg-black/55 px-1.5 py-0.5 text-[10px] text-white"
              >
                {i + 1}
              </span>
              <button
                type="button"
                class="absolute right-1.5 top-1.5 inline-flex size-7 items-center justify-center rounded border-0 bg-black/55 p-0 text-white shadow-none transition-colors hover:bg-destructive"
                onclick={(event) => {
                  event.stopPropagation();
                  onremove(i);
                }}
                aria-label={`删除第 ${i + 1} 张图片`}
              >
                <Trash class="h-3.5 w-3.5" aria-hidden="true" />
              </button>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</GeneralDialog17>
