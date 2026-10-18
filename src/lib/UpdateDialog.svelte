<script lang="ts">
  let {
    version,
    notes,
    installing,
    percent,
    error,
    oninstall,
    onlater,
    onskip,
  }: {
    version: string;
    notes: string;
    installing: boolean;
    percent: number;
    error: string;
    oninstall: () => void;
    onlater: () => void;
    onskip: () => void;
  } = $props();
</script>

<div
  class="overlay"
  role="presentation"
  onclick={() => !installing && onlater()}
  onkeydown={() => {}}
>
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-label="发现新版本"
    tabindex="-1"
    onclick={(event) => event.stopPropagation()}
    onkeydown={() => {}}
  >
    <span class="eyebrow mono">UPDATE</span>
    <h2>发现新版本 v{version}</h2>

    <div class="notes">
      {#if notes}
        <pre>{notes}</pre>
      {:else}
        <p class="muted">更新说明暂无</p>
      {/if}
    </div>

    {#if installing}
      <div class="progress">
        <div class="bar"><span style={`width:${percent}%`}></span></div>
        <div class="progress-label">
          {#if percent >= 100}
            正在安装，应用将自动重启…
          {:else}
            正在下载… {percent}%
          {/if}
        </div>
      </div>
    {/if}

    {#if error}
      <pre class="error">{error}</pre>
    {/if}

    <div class="actions">
      {#if !installing}
        <button class="ghost" onclick={onskip}>跳过此版本</button>
        <button onclick={onlater}>稍后</button>
        <button class="primary" onclick={oninstall}>立即更新</button>
      {:else}
        <button disabled>更新中…</button>
      {/if}
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
    width: 560px;
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
  .notes {
    flex: 1;
    min-height: 80px;
    max-height: 42vh;
    overflow: auto;
    margin: 16px 0 0;
    padding: 12px 14px;
    border-radius: var(--radius);
    background: var(--surface-50);
  }
  .notes pre {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    font: inherit;
    font-size: 13px;
    line-height: 1.7;
  }
  .muted {
    margin: 0;
    color: var(--placeholder);
    font-size: 13px;
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
  .ghost {
    margin-right: auto;
    color: var(--muted);
  }
</style>
