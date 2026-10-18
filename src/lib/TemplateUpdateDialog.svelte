<script lang="ts">
  import type { TemplateUpdateInfo } from "./types";

  let {
    updates,
    busy,
    progress,
    error,
    onupdate,
    onlater,
  }: {
    updates: TemplateUpdateInfo[];
    busy: boolean;
    progress: number;
    error: string;
    onupdate: () => void;
    onlater: () => void;
  } = $props();
</script>

<div
  class="overlay"
  role="presentation"
  onclick={() => !busy && onlater()}
  onkeydown={() => {}}
>
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-label="模板更新"
    tabindex="-1"
    onclick={(event) => event.stopPropagation()}
    onkeydown={() => {}}
  >
    <span class="eyebrow mono">TEMPLATES</span>
    <h2>模板更新</h2>
    <p class="sub">更新只影响新建项目；已内嵌模板的工程不会改变。</p>

    <ul class="list">
      {#each updates as item (item.id)}
        <li>
          <div class="row">
            <div class="meta">
              <span class="name">{item.name}</span>
              <span class="id mono">{item.id}</span>
            </div>
            <span class="version mono">
              {item.currentVersion ? `v${item.currentVersion} → ` : "未安装 · "}v{item.version}
            </span>
          </div>
          <pre class="notes">{item.notes || "更新说明暂无"}</pre>
        </li>
      {/each}
    </ul>

    {#if busy}
      <div class="progress">
        <div class="bar"><span style={`width:${progress}%`}></span></div>
        <div class="progress-label">正在同步模板… {progress}%</div>
      </div>
    {/if}

    {#if error}
      <pre class="error">{error}</pre>
    {/if}

    <div class="actions">
      <button onclick={onlater} disabled={busy}>稍后</button>
      <button class="primary" onclick={onupdate} disabled={busy}>
        {busy ? "更新中…" : "全部更新"}
      </button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 120;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: rgba(0, 0, 0, 0.4);
  }
  .dialog {
    width: 600px;
    max-width: 92vw;
    max-height: 86vh;
    display: flex;
    flex-direction: column;
    padding: 24px;
    border-radius: var(--radius-lg);
    background: var(--panel);
    box-shadow: var(--shadow-elev);
  }
  .eyebrow {
    align-self: flex-start;
    padding: 3px 10px;
    border-radius: var(--radius-pill);
    background: var(--badge-bg);
    color: var(--badge-text);
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.06em;
  }
  h2 {
    margin: 12px 0 0;
    font-size: 20px;
    font-weight: 600;
    letter-spacing: -0.3px;
  }
  .sub {
    margin: 6px 0 0;
    color: var(--text-secondary);
    font-size: 13px;
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 18px 0 0;
    padding: 0;
    list-style: none;
  }
  .list li {
    padding: 12px 14px;
    border-radius: var(--radius);
    background: var(--surface-50);
  }
  .row {
    display: flex;
    align-items: baseline;
    gap: 12px;
  }
  .meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .name {
    font-size: 14px;
    font-weight: 600;
  }
  .id {
    color: var(--placeholder);
    font-size: 11px;
  }
  .version {
    margin-left: auto;
    color: var(--text-secondary);
    font-size: 12px;
    white-space: nowrap;
  }
  .notes {
    margin: 8px 0 0;
    color: var(--text-secondary);
    font: inherit;
    font-size: 12px;
    line-height: 1.65;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .progress {
    margin-top: 16px;
  }
  .bar {
    height: 6px;
    overflow: hidden;
    border-radius: 999px;
    background: var(--surface-100);
  }
  .bar span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--develop, #2563eb);
    transition: width 0.15s ease;
  }
  .progress-label {
    margin-top: 8px;
    color: var(--text-secondary);
    font-size: 12px;
  }
  .error {
    margin: 14px 0 0;
    color: var(--danger);
    font-size: 13px;
    white-space: pre-wrap;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 22px;
  }
</style>
