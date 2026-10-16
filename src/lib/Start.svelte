<script lang="ts">
  import type { RecentItem } from "./types";

  let {
    recent,
    onnew,
    onopen,
    oninstall,
    onopenrecent,
  }: {
    recent: RecentItem[];
    onnew: () => void;
    onopen: () => void;
    oninstall: () => void;
    onopenrecent: (path: string) => void;
  } = $props();
</script>

<div class="home">
  <div class="home-inner">
    <section class="hero">
      <span class="eyebrow mono">REPORT STUDIO</span>
      <h1>欢迎使用报告工坊</h1>
      <p class="lede">从菜单「文件 › 新建项目」开始，或打开一个已有的报告工程。</p>
    </section>

    <div class="cols">
      <section class="col">
        <div class="section-head">
          <span class="eyebrow-sm mono">START</span>
          <h2>启动</h2>
        </div>
        <ul class="list">
          <li>
            <button class="row" onclick={onnew}>
              <span class="ico">
                <svg viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M3 1.5h6l3.5 3.5v3" />
                  <path d="M3 1.5v13h6" />
                  <path d="M12 9.5v5M9.5 12h5" />
                </svg>
              </span>
              <span class="row-label">新建项目</span>
              <span class="row-hint mono">Ctrl N</span>
            </button>
          </li>
          <li>
            <button class="row" onclick={onopen}>
              <span class="ico">
                <svg viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M1.5 3.5h4.5L7.5 5h7v8.5h-13z" />
                  <path d="M1.5 6.5h13" />
                </svg>
              </span>
              <span class="row-label">打开项目</span>
              <span class="row-hint mono">Ctrl O</span>
            </button>
          </li>
          <li>
            <button class="row" onclick={oninstall}>
              <span class="ico">
                <svg viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M8 1.5v7.5" />
                  <path d="M4.5 5.5 8 9l3.5-3.5" />
                  <path d="M2.5 11.5v3h11v-3" />
                </svg>
              </span>
              <span class="row-label">载入新模板</span>
              <span class="row-hint mono">.zip</span>
            </button>
          </li>
        </ul>
      </section>

      <section class="col">
        <div class="section-head">
          <span class="eyebrow-sm mono">RECENT</span>
          <h2>最近</h2>
        </div>
        {#if recent.length === 0}
          <p class="hint">暂无最近打开的项目。</p>
        {:else}
          <ul class="list">
            {#each recent as r (r.path)}
              <li>
                <button
                  class="row recent-row"
                  onclick={() => onopenrecent(r.path)}
                  title={r.path}
                >
                  <span class="row-label name">{r.name || r.path}</span>
                  <span class="path mono">{r.path}</span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    </div>
  </div>
</div>

<style>
  .home {
    display: flex;
    height: 100%;
    overflow: auto;
  }
  .home-inner {
    width: 100%;
    max-width: 780px;
    margin: auto;
    padding: 48px 40px;
  }

  .hero {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }
  .eyebrow {
    display: inline-flex;
    align-items: center;
    padding: 3px 10px;
    border-radius: var(--radius-pill);
    background: var(--badge-bg);
    color: var(--badge-text);
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.06em;
  }
  h1 {
    margin: 16px 0 0;
    font-size: 32px;
    font-weight: 600;
    line-height: 1.2;
    letter-spacing: -0.4px;
  }
  .lede {
    margin: 12px 0 0;
    font-size: 16px;
    line-height: 1.7;
    color: var(--text-secondary);
    max-width: 46em;
  }

  .cols {
    display: flex;
    gap: 40px;
    margin-top: 44px;
    flex-wrap: wrap;
    align-items: flex-start;
  }
  .col {
    flex: 1;
    min-width: 280px;
  }
  .section-head {
    margin-bottom: 14px;
  }
  .eyebrow-sm {
    display: block;
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.08em;
    color: var(--placeholder);
  }
  h2 {
    margin: 2px 0 0;
    font-size: 16px;
    font-weight: 600;
    letter-spacing: -0.2px;
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 8px 10px;
    border: none;
    border-radius: 10px;
    background: transparent;
    box-shadow: none;
    color: var(--text);
    font-size: 14px;
    font-weight: 500;
    text-align: left;
    cursor: pointer;
    transition: background-color 0.15s ease;
  }
  .row:hover:not(:disabled) {
    background: var(--surface-50);
    box-shadow: none;
  }
  .ico {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border-radius: var(--radius);
    background: var(--panel);
    box-shadow: var(--shadow-border);
    color: var(--text);
  }
  .row-label {
    flex: none;
  }
  .row-hint {
    margin-left: auto;
    font-size: 11px;
    color: var(--placeholder);
  }

  .recent-row {
    align-items: baseline;
  }
  .recent-row .name {
    max-width: 45%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .recent-row .path {
    flex: 1;
    min-width: 0;
    font-size: 12px;
    color: var(--placeholder);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .hint {
    margin: 0;
    font-size: 13px;
    color: var(--placeholder);
  }
</style>
