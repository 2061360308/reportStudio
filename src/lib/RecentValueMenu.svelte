<script lang="ts">
  import type { RecentValue } from "./types";

  let {
    open = false,
    left = 0,
    top = 0,
    width = 240,
    items = [],
    activeIndex = -1,
    onpick,
    onhover,
  }: {
    open?: boolean;
    left?: number;
    top?: number;
    width?: number;
    items?: RecentValue[];
    activeIndex?: number;
    onpick?: (value: unknown) => void;
    onhover?: (index: number) => void;
  } = $props();

  function displayValue(value: unknown): string {
    if (typeof value === "string") {
      return value.replace(/\s+/g, " ").trim();
    }
    if (value === null || value === undefined) return "";
    return String(value);
  }
</script>

{#if open && items.length}
  <div
    class="recent-menu"
    role="listbox"
    aria-label="最近使用的值"
    style:left="{left}px"
    style:top="{top}px"
    style:width="{width}px"
  >
    {#each items as item, i (i)}
      <button
        type="button"
        class="recent-item"
        class:active={i === activeIndex}
        role="option"
        aria-selected={i === activeIndex}
        title={displayValue(item.value)}
        onpointerdown={(event) => {
          event.preventDefault();
          onpick?.(item.value);
        }}
        onmouseenter={() => onhover?.(i)}
      >
        <span class="recent-value">{displayValue(item.value)}</span>
      </button>
    {/each}
  </div>
{/if}

<style>
  .recent-menu {
    position: fixed;
    z-index: 300;
    max-height: 240px;
    overflow-y: auto;
    padding: 4px;
    border-radius: var(--radius);
    background: var(--panel);
    box-shadow:
      var(--shadow-border),
      0 8px 24px rgba(0, 0, 0, 0.12);
  }
  .recent-item {
    display: block;
    width: 100%;
    padding: 7px 9px;
    border-radius: var(--radius-sm);
    background: transparent;
    box-shadow: none;
    color: var(--text);
    text-align: left;
    font-size: 13px;
    line-height: 1.4;
  }
  .recent-item:hover,
  .recent-item.active {
    background: var(--surface-100);
  }
  .recent-item:hover {
    color: var(--text);
  }
  .recent-value {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
