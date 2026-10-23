<script lang="ts">
  import type { Project, VariableDef } from "./types";
  import * as api from "./api";
  import ImageIcon from "@lucide/svelte/icons/image";
  import Images from "@lucide/svelte/icons/images";
  import Plus from "@lucide/svelte/icons/plus";
  import RotateCcw from "@lucide/svelte/icons/rotate-ccw";
  import { Button } from "$lib/components/elements/button";
  import * as ButtonGroup from "$lib/components/elements/button-group";
  import ImageGalleryDialog from "./ImageGalleryDialog.svelte";
  import ImageLightbox from "./ImageLightbox.svelte";

  let {
    variable,
    project,
    projectId,
    onchange,
  }: {
    variable: VariableDef;
    project: Project;
    projectId: string;
    onchange?: () => void;
  } = $props();

  let busy = $state(false);
  let err = $state("");
  let dragActive = $state(false);
  let fileInput = $state<HTMLInputElement | undefined>();
  let galleryOpen = $state(false);
  let lightboxOpen = $state(false);
  let lightboxSrc = $state("");
  let lightboxAlt = $state("图片");
  let lightboxCanClear = $state(false);

  const DEFAULT_EXT = ["png", "jpg", "jpeg", "bmp", "webp", "gif"];
  const multi = $derived(variable.type === "images");
  const max = $derived(variable.max ?? 0);
  const allowedExts = $derived(
    (variable.accept?.length ? variable.accept : DEFAULT_EXT)
      .map((ext) => ext.replace(/^\./, "").toLowerCase())
      .filter((ext) => ext !== "pdf"),
  );
  const accept = $derived(allowedExts.map((ext) => `.${ext}`).join(","));
  const imageList = $derived(getList());
  const singleValue = $derived(getSingle());
  const hasDefault = $derived(
    variable.default !== undefined && variable.default !== null,
  );
  const reachedMax = $derived(
    multi && max > 0 && imageList.length >= max,
  );

  function getList(): string[] {
    const value = project.values[variable.key];
    return Array.isArray(value) ? (value as string[]) : [];
  }

  function getSingle(): string {
    const value = project.values[variable.key];
    return typeof value === "string" ? value : "";
  }

  function url(name: string): string {
    return `http://report.localhost/m/${projectId}/${name}`;
  }

  function allowed(name: string): boolean {
    const ext = name.split(".").pop()?.toLowerCase() ?? "";
    return allowedExts.includes(ext);
  }

  function readAsDataURL(file: File): Promise<string> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result));
      reader.onerror = () => reject(reader.error);
      reader.readAsDataURL(file);
    });
  }

  function openPicker() {
    if (busy || reachedMax) return;
    fileInput?.click();
  }

  async function onPick(event: Event) {
    const input = event.target as HTMLInputElement;
    const files = Array.from(input.files ?? []);
    input.value = "";
    if (!files.length) return;
    await uploadFiles(files);
  }

  async function uploadFiles(files: File[]) {
    if (busy || !files.length) return;
    if (reachedMax) {
      err = max > 0 ? `最多 ${max} 张图片` : err;
      return;
    }

    busy = true;
    err = "";
    let changed = false;
    let nextList = multi ? [...imageList] : null;

    try {
      for (const file of files) {
        if (!allowed(file.name)) {
          err = `不支持的文件类型：${file.name}`;
          continue;
        }
        if (multi && max > 0 && (nextList?.length ?? 0) >= max) {
          err = `最多 ${max} 张图片`;
          break;
        }

        const name = await api.addResource(
          projectId,
          file.name,
          await readAsDataURL(file),
        );

        if (multi && nextList) {
          if (!nextList.includes(name)) {
            nextList.push(name);
            changed = true;
          }
        } else {
          project.values[variable.key] = name;
          changed = true;
          break;
        }
      }

      if (multi && nextList && changed) {
        project.values[variable.key] = nextList;
      }
    } catch (e) {
      err = String(e);
    } finally {
      busy = false;
      if (changed) onchange?.();
    }
  }

  function hasFileDrag(event: DragEvent): boolean {
    return Array.from(event.dataTransfer?.types ?? []).includes("Files");
  }

  function onDragEnter(event: DragEvent) {
    if (!hasFileDrag(event)) return;
    event.preventDefault();
    dragActive = true;
  }

  function onDragOver(event: DragEvent) {
    if (!hasFileDrag(event)) return;
    event.preventDefault();
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = busy || reachedMax ? "none" : "copy";
    }
  }

  function onDragLeave(event: DragEvent) {
    const target = event.currentTarget as HTMLElement;
    const related = event.relatedTarget;
    if (related instanceof Node && target.contains(related)) return;
    dragActive = false;
  }

  function onDrop(event: DragEvent) {
    if (!hasFileDrag(event)) return;
    event.preventDefault();
    dragActive = false;
    const files = Array.from(event.dataTransfer?.files ?? []);
    if (files.length) void uploadFiles(files);
  }

  function preventWindowDrop(event: DragEvent) {
    if (hasFileDrag(event)) event.preventDefault();
  }

  function removeAt(index: number) {
    const name = imageList[index];
    const next = imageList.filter((_, i) => i !== index);
    project.values[variable.key] = next;
    onchange?.();

    if (!next.length) {
      galleryOpen = false;
      lightboxOpen = false;
    } else if (lightboxOpen && lightboxSrc === url(name)) {
      lightboxOpen = false;
    }
  }

  function reorder(from: number, to: number) {
    if (from === to || from < 0 || from >= imageList.length) return;
    const target = Math.max(0, Math.min(to, imageList.length - 1));
    const next = [...imageList];
    const [item] = next.splice(from, 1);
    if (item === undefined) return;
    next.splice(target, 0, item);
    project.values[variable.key] = next;
    onchange?.();
  }

  function resetToDefault() {
    const value = variable.default;
    if (multi) {
      const next = Array.isArray(value)
        ? value.filter((item): item is string => typeof item === "string")
        : typeof value === "string" && value
          ? [value]
          : [];
      project.values[variable.key] = next;
    } else {
      project.values[variable.key] =
        typeof value === "string" ? value : "";
    }
    galleryOpen = false;
    lightboxOpen = false;
    onchange?.();
  }

  function clearSingle() {
    project.values[variable.key] = "";
    lightboxOpen = false;
    onchange?.();
  }

  function openSingle() {
    if (!singleValue) return;
    lightboxSrc = url(singleValue);
    lightboxAlt = variable.alias ?? "图片";
    lightboxCanClear = true;
    lightboxOpen = true;
  }

  function openGallery() {
    if (!imageList.length) {
      openPicker();
      return;
    }
    galleryOpen = true;
  }

  function previewImage(name: string) {
    lightboxSrc = url(name);
    lightboxAlt = variable.alias ?? "图片";
    lightboxCanClear = false;
    lightboxOpen = true;
  }
</script>

<svelte:window ondragover={preventWindowDrop} ondrop={preventWindowDrop} />

<div
  class="wrap"
  class:drag-active={dragActive}
  role="group"
  aria-label={variable.alias ?? "图片"}
  ondragenter={onDragEnter}
  ondragover={onDragOver}
  ondragleave={onDragLeave}
  ondrop={onDrop}
>
  {#if multi}
    {#if imageList.length}
      <button
        type="button"
        class="stack"
        onclick={openGallery}
        title={`查看全部 ${imageList.length} 张图片`}
        aria-label={`查看全部 ${imageList.length} 张图片`}
      >
        {#each imageList.slice(0, 3) as name (name)}
          <span class="stack-card">
            <img src={url(name)} alt="" />
          </span>
        {/each}
        <span class="count">{imageList.length} 张</span>
      </button>
    {:else}
      <button
        type="button"
        class="empty"
        onclick={openPicker}
        title="添加图片（可拖拽文件）"
        aria-label="添加图片"
      >
        <Images class="h-5 w-5" aria-hidden="true" />
      </button>
    {/if}
  {:else if singleValue}
    <button
      type="button"
      class="single-thumb"
      onclick={openSingle}
      title="点击放大查看"
      aria-label="点击放大查看"
    >
      <img src={url(singleValue)} alt="" />
    </button>
  {:else}
    <button
      type="button"
      class="empty"
      onclick={openPicker}
      title="添加图片（可拖拽文件）"
      aria-label="添加图片"
    >
      <ImageIcon class="h-5 w-5" aria-hidden="true" />
    </button>
  {/if}

  <ButtonGroup.Root class="shrink-0">
    <Button
      type="button"
      variant="outline"
      size="sm"
      onclick={openPicker}
      disabled={busy || reachedMax}
    >
      <Plus class="h-4 w-4" aria-hidden="true" />
      添加
    </Button>
    {#if hasDefault}
      <Button
        type="button"
        variant="outline"
        size="sm"
        onclick={resetToDefault}
        disabled={busy}
      >
        <RotateCcw class="h-4 w-4" aria-hidden="true" />
        重置
      </Button>
    {/if}
  </ButtonGroup.Root>
</div>

{#if dragActive}<span class="hint">松开以添加图片</span>{/if}
{#if busy}<span class="hint">上传中…</span>{/if}
{#if reachedMax && max > 0}<span class="hint">最多 {max} 张</span>{/if}
{#if err}<span class="err">{err}</span>{/if}

<input
  bind:this={fileInput}
  type="file"
  {accept}
  multiple={multi}
  onchange={onPick}
  class="hidden"
/>

<ImageGalleryDialog
  bind:open={galleryOpen}
  title={variable.alias ?? "图片"}
  images={imageList}
  urlFor={url}
  onclose={() => (galleryOpen = false)}
  onremove={removeAt}
  onreorder={reorder}
  onpreview={previewImage}
/>

<ImageLightbox
  bind:open={lightboxOpen}
  src={lightboxSrc}
  alt={lightboxAlt}
  onclear={lightboxCanClear ? clearSingle : undefined}
/>

<style>
  .wrap {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    border-radius: 10px;
    outline: 2px dashed transparent;
    outline-offset: 4px;
    transition: outline-color 0.12s ease;
  }
  .wrap.drag-active {
    outline-color: var(--focus, #0072f5);
  }
  .wrap.drag-active .empty {
    background: color-mix(in srgb, var(--focus, #0072f5) 12%, transparent);
    color: var(--focus, #0072f5);
  }
  .single-thumb,
  .empty {
    flex: none;
    width: 64px;
    height: 64px;
    padding: 0;
    border: 1px solid var(--border, #e5e5e5);
    border-radius: var(--radius, 8px);
    background: var(--surface-50, #fafafa);
    box-shadow: none;
    overflow: hidden;
    cursor: pointer;
  }
  .single-thumb img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .single-thumb:hover {
    box-shadow: 0 0 0 2px var(--focus, #0072f5);
  }
  .empty {
    display: flex;
    align-items: center;
    justify-content: center;
    border-style: dashed;
    color: var(--placeholder, #808080);
  }
  .empty:hover {
    background: var(--surface-100, #f2f2f2);
  }
  .stack {
    position: relative;
    flex: none;
    width: 72px;
    height: 64px;
    padding: 0;
    border: 0;
    border-radius: 0;
    background: transparent;
    box-shadow: none;
    cursor: pointer;
  }
  .stack:hover {
    background: transparent;
  }
  .stack-card {
    position: absolute;
    left: 4px;
    top: 6px;
    width: 52px;
    height: 52px;
    overflow: hidden;
    border-radius: 6px;
    background: var(--surface-50, #fafafa);
    box-shadow:
      0 0 0 1px rgba(0, 0, 0, 0.14),
      0 2px 4px rgba(0, 0, 0, 0.12);
  }
  .stack-card img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .stack-card:nth-child(1) {
    z-index: 3;
    transform: rotate(-4deg);
  }
  .stack-card:nth-child(2) {
    z-index: 2;
    transform: translate(6px, 2px) rotate(2deg);
  }
  .stack-card:nth-child(3) {
    z-index: 1;
    transform: translate(12px, 4px) rotate(7deg);
  }
  .count {
    position: absolute;
    right: 0;
    bottom: 0;
    z-index: 4;
    padding: 1px 6px;
    border-radius: 9999px;
    background: var(--text, #171717);
    color: #fff;
    font-size: 10px;
    line-height: 1.6;
    white-space: nowrap;
  }
  .hint {
    margin-left: 8px;
    font-size: 12px;
    color: var(--muted, #666);
  }
  .err {
    display: block;
    margin-top: 6px;
    font-size: 12px;
    color: var(--danger, #dc2626);
  }
</style>
