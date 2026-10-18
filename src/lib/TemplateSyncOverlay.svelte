<script lang="ts">
  let {
    percent,
    message,
    error,
    onretry,
    onclose,
  }: {
    percent: number;
    message: string;
    error: string;
    onretry: () => void;
    onclose: () => void;
  } = $props();
</script>

<div class="overlay">
  <div class="panel">
    <span class="eyebrow mono">TEMPLATES</span>
    <h2>{error ? "模板获取失败" : "正在准备模板"}</h2>
    <p class="sub">
      {#if error}
        {error}
      {:else}
        首次启动需要从更新源下载内置模板，请保持网络连接。
      {/if}
    </p>

    {#if !error}
      <div class="bar"><span style={`width:${percent}%`}></span></div>
      <div class="label">{message || "正在同步…"} {percent}%</div>
    {/if}

    <div class="actions">
      <button class="ghost" onclick={onclose}>稍后</button>
      {#if error}
        <button class="primary" onclick={onretry}>重试</button>
      {/if}
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 130;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: rgba(255, 255, 255, 0.72);
    backdrop-filter: blur(10px);
  }
  .panel {
    width: 440px;
    max-width: 90vw;
    padding: 28px;
    border-radius: var(--radius-lg);
    background: var(--panel);
    box-shadow: var(--shadow-elev);
  }
  .eyebrow {
    display: inline-flex;
    padding: 3px 10px;
    border-radius: var(--radius-pill);
    background: var(--badge-bg);
    color: var(--badge-text);
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.06em;
  }
  h2 {
    margin: 14px 0 0;
    font-size: 20px;
    font-weight: 600;
    letter-spacing: -0.3px;
  }
  .sub {
    margin: 8px 0 0;
    color: var(--text-secondary);
    font-size: 13px;
    line-height: 1.7;
  }
  .bar {
    height: 6px;
    overflow: hidden;
    margin-top: 18px;
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
  .label {
    margin-top: 8px;
    color: var(--placeholder);
    font-size: 12px;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 20px;
  }
  .ghost {
    color: var(--muted);
  }
</style>
