<script module lang="ts">
  type Mem = {
    zoom: number;
    current: number;
    mode: string;
    scrollTop: number;
    scrollLeft: number;
    sidebarOpen: boolean;
    highlightAll: boolean;
    panelScroll: Record<string, number>;
    collapsed: Record<string, boolean>;
  };
  // 单次运行内、跨"变量/预览"切换保留预览状态（按模板 id 记忆）
  const previewMemory = new Map<string, Mem>();
</script>

<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import PanelLeft from "@lucide/svelte/icons/panel-left";
  import ChevronsLeft from "@lucide/svelte/icons/chevrons-left";
  import ChevronLeft from "@lucide/svelte/icons/chevron-left";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import ChevronsRight from "@lucide/svelte/icons/chevrons-right";
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
  import Proportions from "@lucide/svelte/icons/proportions";
  import ScanSquare from "@lucide/svelte/icons/scan-square";
  import SquareDashedBottomCode from "@lucide/svelte/icons/square-dashed-bottom-code";
  import Minus from "@lucide/svelte/icons/minus";
  import Plus from "@lucide/svelte/icons/plus";
  import TableOfContents from "@lucide/svelte/icons/table-of-contents";
  import GalleryThumbnails from "@lucide/svelte/icons/gallery-thumbnails";
  import Bookmark from "@lucide/svelte/icons/bookmark";
  import ListChevronsUpDown from "@lucide/svelte/icons/list-chevrons-up-down";
  import ListChevronsDownUp from "@lucide/svelte/icons/list-chevrons-down-up";
  import Siren from "@lucide/svelte/icons/siren";
  import * as Empty from "$lib/components/elements/empty";
  import * as Resizable from "$lib/components/elements/resizable";
  import * as api from "./api";
  import type { OpenedProject, PreviewPage } from "./types";

  let { opened, onedit }: { opened: OpenedProject; onedit?: (key: string) => void } =
    $props();

  const MM = 96 / 25.4; // 1mm ≈ 3.7795px
  const A4_W = 210 * MM;
  const A4_H = 297 * MM;
  const THUMB_W = 132;
  const MIN_ZOOM = 0.25;
  const MAX_ZOOM = 4;

  let pages = $state<PreviewPage[]>([]);
  let error = $state("");
  let busy = $state(false);
  let zoom = $state(1);
  let current = $state(0);
  let dragging = $state(false);
  let dragRect = false;
  let viewRect = $state<{
    left: number;
    top: number;
    width: number;
    height: number;
  } | null>(null);

  let stage = $state<HTMLDivElement | undefined>();
  let sidebar = $state<HTMLElement | undefined>();
  let frames = $state<(HTMLIFrameElement | undefined)[]>([]);
  let thumbEls = $state<(HTMLButtonElement | undefined)[]>([]);
  let lastX = 0;
  let lastY = 0;
  let raf = 0;

  let sidebarOpen = $state(true);
  let showZoom = $state(false);
  let mode = $state<"toc" | "thumbs" | "bookmarks">("toc");
  let highlightAll = $state(false);
  let zoomInput = $state("100");
  let pageText = $state("");
  let pageFocused = $state(false);
  let ready = $state(false);

  const toc = $derived(opened.manifest.toc ?? []);

  interface Occ {
    page: string;
    id: string;
    field: string;
    text: string;
    before: string;
    after: string;
    x: number;
    y: number;
    w: number;
    h: number;
  }
  let fields = $state<Occ[]>([]);
  let collapsed = $state<Record<string, boolean>>({});
  let hover: { page: string; id: string } | null = null;

  const fieldGroups = $derived.by(() => {
    const map = new Map<string, Occ[]>();
    for (const o of fields) {
      if (!map.has(o.field)) map.set(o.field, []);
      map.get(o.field)!.push(o);
    }
    return [...map.entries()];
  });
  const allCollapsed = $derived(
    fieldGroups.length > 0 && fieldGroups.every(([k]) => collapsed[k]),
  );

  function aliasOf(key: string): string {
    return opened.manifest.variables.find((v) => v.key === key)?.alias ?? key;
  }
  function toggleGroup(key: string) {
    collapsed = { ...collapsed, [key]: !collapsed[key] };
  }
  function toggleAll() {
    if (allCollapsed) {
      collapsed = {};
    } else {
      const c: Record<string, boolean> = {};
      for (const [k] of fieldGroups) c[k] = true;
      collapsed = c;
    }
  }

  function selectMode(m: "toc" | "thumbs" | "bookmarks") {
    mode = m;
    tick().then(restorePanelScroll);
  }
  function restorePanelScroll() {
    const mem = previewMemory.get(opened.manifest.id);
    if (sidebar) sidebar.scrollTop = mem?.panelScroll?.[mode] ?? 0;
  }

  function applyHighlightAll() {
    for (const f of frames) {
      f?.contentWindow?.postMessage(
        { type: "rs-highlight-all", on: highlightAll },
        "*",
      );
    }
  }
  function toggleHighlightAll() {
    highlightAll = !highlightAll;
    applyHighlightAll();
  }

  function onMainLoad(i: number) {
    updateView();
    if (highlightAll) {
      frames[i]?.contentWindow?.postMessage(
        { type: "rs-highlight-all", on: true },
        "*",
      );
    }
  }

  function revealItem(o: Occ) {
    const idx = pages.findIndex((p) => p.file === o.page);
    if (idx < 0) return;
    scrollToPage(idx);
    tick().then(() => {
      frames[idx]?.contentWindow?.postMessage({ type: "rs-locate", id: o.id }, "*");
    });
  }

  function onMessage(ev: MessageEvent) {
    const d = ev.data as {
      type?: string;
      page?: string;
      id?: string;
      field?: string;
      top?: number;
      items?: Occ[];
    };
    if (!d || typeof d !== "object") return;
    // 只接受主视图 iframe 的消息（缩略图带 ?thumb=1，不发送）
    if (!frames.some((f) => f && f.contentWindow === ev.source)) return;
    if (d.type === "rs-fields" && Array.isArray(d.items)) {
      fields = [
        ...fields.filter((o) => o.page !== d.page),
        ...d.items.map((it) => ({ ...it, page: d.page as string })),
      ];
    } else if (d.type === "rs-located") {
      const idx = pages.findIndex((p) => p.file === d.page);
      const f = idx >= 0 ? frames[idx] : undefined;
      if (!f || !stage) return;
      const st = stage.getBoundingClientRect();
      const fr = f.getBoundingClientRect();
      const frameTop = fr.top - st.top + stage.scrollTop;
      stage.scrollTop =
        frameTop + (Number(d.top) || 0) * zoom - stage.clientHeight / 2;
      updateView();
    } else if (d.type === "rs-edit" && d.field) {
      onedit?.(d.field);
    }
  }

  let moveRaf = 0;

  function hitTest(clientX: number, clientY: number): Occ | null {
    for (let i = 0; i < frames.length; i++) {
      const f = frames[i];
      if (!f) continue;
      const r = f.getBoundingClientRect();
      if (
        clientX < r.left ||
        clientX > r.right ||
        clientY < r.top ||
        clientY > r.bottom
      ) {
        continue;
      }
      const page = pages[i];
      if (!page) return null;
      const px = (clientX - r.left) / zoom;
      const py = (clientY - r.top) / zoom;
      for (const o of fields) {
        if (o.page !== page.file) continue;
        if (px >= o.x && px <= o.x + o.w && py >= o.y && py <= o.y + o.h) {
          return o;
        }
      }
      return null;
    }
    return null;
  }

  function postHover(page: string, id: string) {
    const idx = pages.findIndex((p) => p.file === page);
    frames[idx]?.contentWindow?.postMessage({ type: "rs-hover", id }, "*");
  }

  function clearHover() {
    if (!hover) return;
    postHover(hover.page, "");
    hover = null;
  }

  function setHover(o: Occ | null) {
    if (o) {
      if (hover && hover.page === o.page && hover.id === o.id) return;
      clearHover();
      postHover(o.page, o.id);
      hover = { page: o.page, id: o.id };
    } else {
      clearHover();
    }
  }

  function onStageMove(ev: MouseEvent) {
    if (dragging) return;
    const cx = ev.clientX;
    const cy = ev.clientY;
    if (moveRaf) return;
    moveRaf = requestAnimationFrame(() => {
      moveRaf = 0;
      setHover(hitTest(cx, cy));
    });
  }

  function onStageDblclick(ev: MouseEvent) {
    const o = hitTest(ev.clientX, ev.clientY);
    if (o) onedit?.(o.field);
  }

  function nativeSize(p: PreviewPage | null) {
    if (p && p.orientation === "landscape") return { w: A4_H, h: A4_W };
    return { w: A4_W, h: A4_H };
  }

  const thumbs = $derived(
    pages.map((p) => {
      const n = nativeSize(p);
      const scale = THUMB_W / n.w;
      return { p, nw: n.w, nh: n.h, scale, height: n.h * scale };
    }),
  );

  function trackFrame(node: HTMLIFrameElement, i: number) {
    frames[i] = node;
    return {};
  }

  function trackThumb(node: HTMLButtonElement, i: number) {
    thumbEls[i] = node;
    return {};
  }

  function ensureThumbVisible(i: number) {
    const el = thumbEls[i];
    if (!el || !el.isConnected || !sidebar) return;
    const st = sidebar.getBoundingClientRect();
    const r = el.getBoundingClientRect();
    if (r.top < st.top) sidebar.scrollTop -= st.top - r.top + 8;
    else if (r.bottom > st.bottom) sidebar.scrollTop += r.bottom - st.bottom + 8;
  }

  async function render() {
    busy = true;
    error = "";
    try {
      pages = await api.renderPreview(opened.templateDir, opened.project.values);
      frames = [];
      thumbEls = [];
      const mem = previewMemory.get(opened.manifest.id);
      if (mem) {
        zoom = mem.zoom;
        current = mem.current;
        mode = mem.mode as "toc" | "thumbs" | "bookmarks";
        sidebarOpen = mem.sidebarOpen ?? true;
        highlightAll = mem.highlightAll ?? false;
        collapsed = mem.collapsed ?? {};
      } else {
        zoom = 1;
        current = 0;
        mode = "toc";
        sidebarOpen = true;
        highlightAll = false;
        collapsed = {};
      }
      await tick();
      if (mem && stage) {
        stage.scrollTop = mem.scrollTop;
        stage.scrollLeft = mem.scrollLeft;
      }
      if (sidebar) sidebar.scrollTop = mem?.panelScroll?.[mode] ?? 0;
      updateView();
      // 首次打开：默认"适合页面"
      if (!mem) fitPage();
      applyHighlightAll();
      ready = true;
      saveMemory();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  onMount(render);
  onMount(() => {
    window.addEventListener("message", onMessage);
    return () => window.removeEventListener("message", onMessage);
  });
  onDestroy(saveMemory);

  function saveMemory() {
    if (!stage || !ready) return;
    const id = opened.manifest.id;
    const prev = previewMemory.get(id);
    previewMemory.set(id, {
      zoom,
      current,
      mode,
      scrollTop: stage.scrollTop,
      scrollLeft: stage.scrollLeft,
      sidebarOpen,
      highlightAll,
      panelScroll: {
        ...(prev?.panelScroll ?? {}),
        [mode]: sidebar?.scrollTop ?? 0,
      },
      collapsed: { ...collapsed },
    });
  }

  function clamp(z: number) {
    return Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, z));
  }

  function onScroll() {
    if (raf) return;
    raf = requestAnimationFrame(() => {
      raf = 0;
      clearHover();
      updateView();
      saveMemory();
    });
  }

  function updateView() {
    if (!stage || pages.length === 0) {
      viewRect = null;
      return;
    }
    const st = stage.getBoundingClientRect();

    // 当前页 = 视口内可见面积最大的那一页
    let best = current;
    let bestArea = -1;
    for (let i = 0; i < frames.length; i++) {
      const el = frames[i];
      if (!el) continue;
      const r = el.getBoundingClientRect();
      const w = Math.max(0, Math.min(r.right, st.right) - Math.max(r.left, st.left));
      const h = Math.max(0, Math.min(r.bottom, st.bottom) - Math.max(r.top, st.top));
      const area = w * h;
      if (area > bestArea) {
        bestArea = area;
        best = i;
      }
    }
    if (best !== current) {
      current = best;
      ensureThumbVisible(best);
    }

    const el = frames[current];
    const page = pages[current];
    if (!el || !page) {
      viewRect = null;
      return;
    }
    const r = el.getBoundingClientRect();
    // 页面完整可见则不画矩形
    if (r.width <= st.width + 0.5 && r.height <= st.height + 0.5) {
      viewRect = null;
      return;
    }
    const left = Math.max(st.left, r.left);
    const top = Math.max(st.top, r.top);
    const right = Math.min(st.right, r.right);
    const bottom = Math.min(st.bottom, r.bottom);
    const dw = right - left;
    const dh = bottom - top;
    if (dw <= 0 || dh <= 0) {
      viewRect = null;
      return;
    }
    const s = THUMB_W / nativeSize(page).w;
    viewRect = {
      left: ((left - r.left) / zoom) * s,
      top: ((top - r.top) / zoom) * s,
      width: (dw / zoom) * s,
      height: (dh / zoom) * s,
    };
  }

  function scrollToPage(i: number) {
    const el = frames[i];
    if (!el || !stage) return;
    const st = stage.getBoundingClientRect();
    const r = el.getBoundingClientRect();
    stage.scrollTop += r.top - st.top - 12;
    current = i;
    ensureThumbVisible(i);
    updateView();
    saveMemory();
  }

  function applyZoom(nz: number) {
    const z = clamp(nz);
    if (!stage || z === zoom) {
      zoom = z;
      return;
    }
    const f = z / zoom;
    zoom = z;
    tick().then(() => {
      if (stage) {
        stage.scrollLeft *= f;
        stage.scrollTop *= f;
      }
      updateView();
      saveMemory();
    });
  }

  function fitWidth() {
    if (!stage || !pages[current]) return;
    const n = nativeSize(pages[current]);
    applyZoom((stage.clientWidth - 24) / n.w);
  }

  function fitPage() {
    if (!stage || !pages[current]) return;
    const n = nativeSize(pages[current]);
    applyZoom(
      Math.min((stage.clientWidth - 24) / n.w, (stage.clientHeight - 24) / n.h),
    );
  }

  function openZoom() {
    zoomInput = String(Math.round(zoom * 100));
    showZoom = true;
  }

  function applyCustom() {
    const v = parseFloat(zoomInput);
    if (!isNaN(v)) applyZoom(v / 100);
    showZoom = false;
  }

  function setIndex(i: number) {
    if (i < 0 || i >= pages.length) return;
    scrollToPage(i);
  }

  function gotoPage(n: number) {
    setIndex(n - 1);
  }

  function onPageFocus(e: FocusEvent) {
    pageFocused = true;
    pageText = String(current + 1);
    requestAnimationFrame(() => (e.target as HTMLInputElement).select());
  }

  function commitPage() {
    const n = parseInt(pageText, 10);
    if (!isNaN(n)) setIndex(Math.min(pages.length, Math.max(1, n)) - 1);
  }

  function onPageBlur() {
    pageFocused = false;
    commitPage();
  }

  function onPageKey(e: KeyboardEvent) {
    if (e.key === "Enter") (e.target as HTMLInputElement).blur();
    if (e.key === "Escape") (e.target as HTMLInputElement).blur();
  }

  function onWheel(e: WheelEvent) {
    if (!e.ctrlKey) return; // 普通滚轮 → 连续翻页（自然滚动）
    e.preventDefault();
    applyZoom(zoom * (e.deltaY < 0 ? 1.1 : 1 / 1.1));
  }

  /* ---------- 拖动页面（平移滚动） ---------- */
  function onPointerDown(e: PointerEvent) {
    if (!stage || e.button !== 0) return;
    if (stage.scrollWidth <= stage.clientWidth && stage.scrollHeight <= stage.clientHeight)
      return;
    dragging = true;
    lastX = e.clientX;
    lastY = e.clientY;
    stage.setPointerCapture(e.pointerId);
    e.preventDefault();
  }
  function onPointerMove(e: PointerEvent) {
    if (!dragging || !stage) return;
    stage.scrollLeft -= e.clientX - lastX;
    stage.scrollTop -= e.clientY - lastY;
    lastX = e.clientX;
    lastY = e.clientY;
  }
  function onPointerUp() {
    dragging = false;
  }

  /* ---------- 拖动缩略图上的视口矩形 ---------- */
  function onRectDown(e: PointerEvent) {
    e.stopPropagation();
    dragRect = true;
    lastX = e.clientX;
    lastY = e.clientY;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    e.preventDefault();
  }
  function onRectMove(e: PointerEvent) {
    if (!dragRect || !stage || !pages[current]) return;
    const dx = e.clientX - lastX;
    const dy = e.clientY - lastY;
    lastX = e.clientX;
    lastY = e.clientY;
    const s = THUMB_W / nativeSize(pages[current]).w;
    stage.scrollLeft += (dx / s) * zoom;
    stage.scrollTop += (dy / s) * zoom;
  }
  function onRectUp() {
    dragRect = false;
  }

  $effect(() => {
    void pages.length;
    void zoom;
    tick().then(updateView);
  });

  $effect(() => {
    if (!pageFocused) {
      void current;
      void pages.length;
      pageText = pages.length ? `${current + 1} / ${pages.length}` : "-";
    }
  });

  $effect(() => {
    void ready;
    void zoom;
    void current;
    void mode;
    void sidebarOpen;
    void highlightAll;
    void collapsed;
    saveMemory();
  });
</script>

<div class="preview">
  {#if error}
    <pre class="error">{error}</pre>
  {/if}

  <div class="body">
    <Resizable.PaneGroup
      direction="horizontal"
      autoSaveId="rs-preview"
      class="h-full w-full"
    >
      {#if sidebarOpen}
        <Resizable.Pane defaultSize={20} minSize={8} maxSize={55} class="min-w-0">
          <div class="left">
        <nav class="rail">
          <button
            class="railbtn"
            class:active={mode === "toc"}
            title="目录"
            onclick={() => selectMode("toc")}
          >
            <TableOfContents size={18} />
          </button>
          <button
            class="railbtn"
            class:active={mode === "thumbs"}
            title="缩略图"
            onclick={() => selectMode("thumbs")}
          >
            <GalleryThumbnails size={18} />
          </button>
          <button
            class="railbtn"
            class:active={mode === "bookmarks"}
            title="书签"
            onclick={() => selectMode("bookmarks")}
          >
            <Bookmark size={18} />
          </button>
        </nav>

        <aside class="panel" bind:this={sidebar} onscroll={saveMemory}>
          {#if mode === "thumbs"}
            <div class="thumbs">
              {#each thumbs as t, i (t.p.file)}
                <button
                  class="thumb"
                  class:active={i === current}
                  onclick={() => scrollToPage(i)}
                  title={`第 ${i + 1} 页`}
                  use:trackThumb={i}
                >
                  <div
                    class="thumb-frame"
                    style="width:{THUMB_W}px;height:{t.height}px;"
                  >
                    <iframe
                      class="thumb-iframe"
                      src={`${t.p.url}?thumb=1`}
                      scrolling="no"
                      tabindex="-1"
                      title={`缩略图 ${i + 1}`}
                      style="width:{t.nw}px;height:{t.nh}px;transform:scale({t.scale});"
                    ></iframe>
                    {#if i === current && viewRect}
                      <div
                        class="vrect"
                        role="presentation"
                        style="left:{viewRect.left}px;top:{viewRect.top}px;width:{viewRect.width}px;height:{viewRect.height}px;"
                        onpointerdown={onRectDown}
                        onpointermove={onRectMove}
                        onpointerup={onRectUp}
                        onpointercancel={onRectUp}
                      ></div>
                    {/if}
                  </div>
                  <span class="num">{i + 1}</span>
                </button>
              {/each}
            </div>
          {:else if mode === "toc"}
            {#if toc.length}
              <ul class="toc">
                {#each toc as item (item.title)}
                  <li>
                    <button
                      class="tocitem"
                      class:active={item.page === current + 1}
                      onclick={() => gotoPage(item.page)}
                      title={item.title}
                    >
                      <span class="toctitle">{item.title}</span>
                      <span class="tocpage">{item.page}</span>
                    </button>
                  </li>
                {/each}
              </ul>
            {:else}
              <Empty.Root class="gap-3 border-0 p-6 md:p-6">
                <Empty.Media variant="icon">
                  <TableOfContents aria-hidden="true" />
                </Empty.Media>
                <Empty.Header>
                  <Empty.Title>暂无目录</Empty.Title>
                  <Empty.Description>该模板未定义目录。</Empty.Description>
                </Empty.Header>
              </Empty.Root>
            {/if}
          {:else}
            {#if fieldGroups.length}
              <div class="bm-bar">
                <button
                  class="bm-tool"
                  title={allCollapsed ? "全部展开" : "全部折叠"}
                  onclick={toggleAll}
                >
                  {#if allCollapsed}
                    <ListChevronsUpDown size={15} />
                  {:else}
                    <ListChevronsDownUp size={15} />
                  {/if}
                </button>
                <div class="spacer"></div>
                <button
                  class="bm-tool"
                  class:on={highlightAll}
                  title="全部高亮"
                  onclick={toggleHighlightAll}
                >
                  <Siren size={15} />
                </button>
              </div>
              <div class="bm-list">
                {#each fieldGroups as [key, occs] (key)}
                  <div class="bm-group">
                    <button class="bm-head" onclick={() => toggleGroup(key)}>
                      <span class="bm-caret" class:open={!collapsed[key]}>▶</span>
                      <span class="bm-name">{aliasOf(key)}</span>
                      <span class="bm-count">{occs.length}</span>
                    </button>
                    {#if !collapsed[key]}
                      <ul class="bm-items">
                        {#each occs as o (`${o.page}#${o.id}`)}
                          <li>
                            <button
                              class="bm-item"
                              onclick={() => revealItem(o)}
                              ondblclick={() => onedit?.(o.field)}
                              title="单击定位高亮，双击编辑该字段"
                            >
                              <span class="ctx"
                                >…{o.before}<em>{o.text}</em>{o.after}…</span
                              >
                            </button>
                          </li>
                        {/each}
                      </ul>
                    {/if}
                  </div>
                {/each}
              </div>
            {:else}
              <Empty.Root class="gap-3 border-0 p-6 md:p-6">
                <Empty.Media variant="icon">
                  <Bookmark aria-hidden="true" />
                </Empty.Media>
                <Empty.Header>
                  <Empty.Title>暂无字段标记</Empty.Title>
                  <Empty.Description>预览页面中未发现模板字段。</Empty.Description>
                </Empty.Header>
              </Empty.Root>
            {/if}
          {/if}
            </aside>
          </div>
        </Resizable.Pane>
        <Resizable.Handle withHandle />
      {/if}
      <Resizable.Pane class="min-w-0">
        <div
          class="stage"
      class:dragging
      role="presentation"
      bind:this={stage}
      onscroll={onScroll}
      onwheel={onWheel}
      onpointerdown={onPointerDown}
      onpointermove={onPointerMove}
      onpointerup={onPointerUp}
      onpointercancel={onPointerUp}
      onmousemove={onStageMove}
      onmouseleave={clearHover}
      ondblclick={onStageDblclick}
    >
      {#if pages.length === 0 && busy}
        <div class="empty">渲染中…</div>
      {/if}
      {#each thumbs as t, i (t.p.file)}
        <div class="frame" style="width:{t.nw * zoom}px;height:{t.nh * zoom}px;">
          <iframe
            class="page-iframe"
            src={t.p.url}
            scrolling="no"
            title={`第 ${i + 1} 页`}
            use:trackFrame={i}
            onload={() => onMainLoad(i)}
            style="width:{t.nw}px;height:{t.nh}px;transform:scale({zoom});"
          ></iframe>
        </div>
      {/each}
        </div>
      </Resizable.Pane>
    </Resizable.PaneGroup>
  </div>

  <div class="statusbar">
    <div class="group">
      <button
        class="ibtn"
        onclick={() => (sidebarOpen = !sidebarOpen)}
        title={sidebarOpen ? "折叠缩略图" : "展开缩略图"}
      >
        <PanelLeft size={16} />
      </button>
      <span class="divider"></span>
      <button class="ibtn" onclick={() => setIndex(0)} disabled={current <= 0} title="首页">
        <ChevronsLeft size={16} />
      </button>
      <button class="ibtn" onclick={() => setIndex(current - 1)} disabled={current <= 0} title="上一页">
        <ChevronLeft size={16} />
      </button>
      <input
        class="pagebox"
        value={pageText}
        onfocus={onPageFocus}
        onblur={onPageBlur}
        onkeydown={onPageKey}
        oninput={(e) => pageFocused && (pageText = (e.target as HTMLInputElement).value)}
        title="输入页码后回车跳转"
      />
      <button class="ibtn" onclick={() => setIndex(current + 1)} disabled={current >= pages.length - 1} title="下一页">
        <ChevronRight size={16} />
      </button>
      <button class="ibtn" onclick={() => setIndex(pages.length - 1)} disabled={current >= pages.length - 1} title="尾页">
        <ChevronsRight size={16} />
      </button>
    </div>

    <div class="spacer"></div>

    <div class="group">
      <button class="lbtn" onclick={() => applyZoom(1)}>
        <Proportions size={14} />实际大小
      </button>
      <button class="lbtn" onclick={fitPage}>
        <ScanSquare size={14} />适合页面
      </button>
      <button class="lbtn" onclick={fitWidth}>
        <SquareDashedBottomCode size={14} />适合宽度
      </button>

      <div class="zoomwrap">
        <button class="lbtn pctbtn" onclick={openZoom}>
          {Math.round(zoom * 100)}%
          <ChevronDown size={13} />
        </button>
        {#if showZoom}
          <div class="mask" role="presentation" onclick={() => (showZoom = false)}></div>
          <div class="zoompanel">
            <div class="zp-title">显示比例</div>
            {#each [3, 1, 0.75, 0.5, 0.37, 0.01] as r}
              <button class="zp-item" onclick={() => { applyZoom(r); showZoom = false; }}>
                <span class="radio" class:on={Math.abs(zoom - r) < 0.001}></span>
                {Math.round(r * 100)}%
              </button>
            {/each}
            <div class="zp-title">百分比</div>
            <div class="zp-input">
              <input
                type="number"
                bind:value={zoomInput}
                onkeydown={(e) => e.key === "Enter" && applyCustom()}
              />
              <span class="pctsign">%</span>
            </div>
          </div>
        {/if}
      </div>

      <button class="ibtn" onclick={() => applyZoom(zoom / 1.1)} title="缩小">
        <Minus size={16} />
      </button>
      <input
        class="slider"
        type="range"
        min="25"
        max="400"
        step="1"
        value={Math.round(zoom * 100)}
        oninput={(e) => applyZoom(Number((e.target as HTMLInputElement).value) / 100)}
        title="缩放"
      />
      <button class="ibtn" onclick={() => applyZoom(zoom * 1.1)} title="放大">
        <Plus size={16} />
      </button>
    </div>
  </div>
</div>

<style>
  .preview {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .body {
    flex: 1;
    min-height: 0;
  }

  /* ---------- 左侧导航（竖排窗格 + 面板，无外框） ---------- */
  .left {
    display: flex;
    height: 100%;
    min-height: 0;
  }
  .rail {
    flex: none;
    width: 40px;
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 8px 0;
    border-right: 1px solid var(--border);
  }
  .railbtn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    padding: 0;
    border: none;
    border-radius: 8px;
    background: transparent;
    box-shadow: none;
    color: var(--muted);
  }
  .railbtn:hover {
    background: var(--surface-50);
    box-shadow: none;
    color: var(--text);
  }
  .railbtn.active {
    background: var(--surface-100);
    color: var(--text);
  }
  .railbtn :global(svg) {
    display: block;
  }

  .panel {
    flex: 1;
    min-width: 0;
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
    background: transparent;
  }
  .thumbs {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 8px 4px;
  }
  .toc {
    list-style: none;
    margin: 0;
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .tocitem {
    display: flex;
    align-items: baseline;
    gap: 8px;
    width: 100%;
    padding: 5px 8px;
    border: none;
    border-radius: 6px;
    background: transparent;
    box-shadow: none;
    color: var(--text);
    font-size: 13px;
    text-align: left;
  }
  .tocitem:hover {
    background: var(--surface-50);
    box-shadow: none;
  }
  .tocitem.active {
    background: rgba(37, 99, 235, 0.12);
  }
  .toctitle {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tocpage {
    flex: none;
    color: var(--placeholder);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }

  /* ---------- 书签（字段标记）面板 ---------- */
  .bm-bar {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 6px 6px 2px;
    background: var(--bg);
  }
  .bm-tool {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 24px;
    padding: 0;
    border: none;
    border-radius: 6px;
    background: transparent;
    box-shadow: none;
    color: var(--muted);
  }
  .bm-tool:hover {
    background: var(--surface-50);
    box-shadow: none;
    color: var(--text);
  }
  .bm-tool.on {
    background: rgba(37, 99, 235, 0.14);
    color: var(--develop);
  }
  .bm-tool :global(svg) {
    display: block;
  }
  .bm-group {
    padding: 0 4px;
  }
  .bm-head {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 5px 6px;
    border: none;
    border-radius: 6px;
    background: transparent;
    box-shadow: none;
    color: var(--text);
    font-size: 13px;
    font-weight: 600;
    text-align: left;
  }
  .bm-head:hover {
    background: var(--surface-50);
    box-shadow: none;
  }
  .bm-caret {
    flex: none;
    display: inline-block;
    font-size: 9px;
    color: var(--placeholder);
    transition: transform 0.15s ease;
  }
  .bm-caret.open {
    transform: rotate(90deg);
  }
  .bm-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .bm-count {
    flex: none;
    font-weight: 400;
    color: var(--placeholder);
    font-size: 11px;
  }
  .bm-items {
    list-style: none;
    margin: 0 0 4px;
    padding: 0 0 0 4px;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .bm-item {
    display: block;
    width: 100%;
    padding: 4px 8px;
    border: none;
    border-radius: 6px;
    background: transparent;
    box-shadow: none;
    color: var(--text-secondary);
    font-size: 12px;
    text-align: left;
  }
  .bm-item:hover {
    background: var(--surface-50);
    box-shadow: none;
  }
  .bm-item .ctx {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .bm-item em {
    font-style: normal;
    background-image: linear-gradient(180deg, transparent 56%, #ffe58f 56%);
    border-radius: 2px;
  }

  .thumb {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: 3px;
    border: none;
    border-radius: 6px;
    background: transparent;
    box-shadow: none;
    flex: none;
  }
  .thumb:hover {
    background: rgba(0, 0, 0, 0.05);
    box-shadow: none;
  }
  .thumb.active {
    background: rgba(37, 99, 235, 0.12);
  }
  .thumb-frame {
    position: relative;
    overflow: hidden;
    background: #fff;
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.15);
  }
  .thumb-iframe,
  .page-iframe {
    position: absolute;
    top: 0;
    left: 0;
    border: 0;
    display: block;
    transform-origin: 0 0;
    pointer-events: none;
    background: #fff;
  }
  .num {
    font-size: 11px;
    color: var(--muted);
  }
  .thumb.active .num {
    color: var(--text);
    font-weight: 600;
  }
  .vrect {
    position: absolute;
    border: 1.5px solid var(--develop);
    background: rgba(10, 114, 239, 0.12);
    box-sizing: border-box;
    cursor: move;
    pointer-events: auto;
  }

  /* ---------- 右侧连续页面（无外框） ---------- */
  .stage {
    position: relative;
    width: 100%;
    height: 100%;
    min-width: 0;
    overflow: auto;
    padding: 12px;
    background: #eceef1;
  }
  .stage.dragging {
    cursor: grabbing;
    user-select: none;
  }
  .frame {
    position: relative;
    margin: 0 auto 14px;
    overflow: hidden;
    background: #fff;
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.15);
  }
  .empty {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--placeholder);
    font-size: 13px;
  }

  /* ---------- 底部状态栏 ---------- */
  .statusbar {
    position: relative;
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 10px;
    background: var(--panel);
    box-shadow: 0 -1px 0 0 var(--ring);
  }
  .group {
    display: flex;
    align-items: center;
    gap: 2px;
    min-width: 0;
  }
  .spacer {
    flex: 1;
  }
  .divider {
    width: 1px;
    height: 18px;
    margin: 0 6px;
    background: var(--border);
  }

  .ibtn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 26px;
    padding: 0;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    box-shadow: none;
    color: var(--text);
  }
  .ibtn:hover:not(:disabled) {
    background: var(--surface-50);
    box-shadow: none;
  }
  .ibtn :global(svg),
  .lbtn :global(svg),
  .pctbtn :global(svg) {
    display: block;
  }
  .ibtn:disabled {
    background: transparent;
    box-shadow: none;
    color: var(--placeholder);
  }

  .lbtn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 26px;
    padding: 0 10px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    box-shadow: none;
    color: var(--text);
    font-size: 12px;
    white-space: nowrap;
  }
  .lbtn:hover {
    background: var(--surface-50);
    box-shadow: none;
  }

  .pagebox {
    width: 72px;
    height: 26px;
    padding: 0 6px;
    text-align: center;
    font-size: 12px;
    background: var(--panel);
  }

  .pctbtn {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    min-width: 58px;
  }
  .zoomwrap {
    position: relative;
  }
  .mask {
    position: fixed;
    inset: 0;
    z-index: 40;
  }
  .zoompanel {
    position: absolute;
    right: 0;
    bottom: calc(100% + 6px);
    z-index: 41;
    width: 180px;
    padding: 10px 10px 12px;
    border-radius: var(--radius);
    background: var(--panel);
    box-shadow:
      var(--shadow-border),
      0 8px 24px rgba(0, 0, 0, 0.12);
  }
  .zp-title {
    padding: 4px 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text);
  }
  .zp-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 4px 6px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    box-shadow: none;
    color: var(--text);
    font-size: 13px;
    text-align: left;
  }
  .zp-item:hover {
    background: var(--surface-50);
    box-shadow: none;
  }
  .radio {
    width: 13px;
    height: 13px;
    flex: none;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1.5px var(--placeholder);
  }
  .radio.on {
    box-shadow:
      inset 0 0 0 1.5px var(--develop),
      inset 0 0 0 4px var(--panel);
    background: var(--develop);
  }
  .zp-input {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 2px 0 0 6px;
  }
  .zp-input input {
    flex: 1;
    min-width: 0;
    height: 28px;
  }
  .pctsign {
    font-size: 13px;
    color: var(--muted);
  }

  .slider {
    width: 120px;
    max-width: 30vw;
    accent-color: var(--text);
    box-shadow: none;
    padding: 0;
    background: transparent;
  }

  .error {
    color: var(--danger);
    white-space: pre-wrap;
    font-size: 12px;
    margin: 0;
    padding: 6px 12px;
  }
</style>
