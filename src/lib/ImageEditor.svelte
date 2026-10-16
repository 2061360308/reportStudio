<script lang="ts">
  import type { Project, VariableDef } from "./types";
  import * as api from "./api";

  let {
    variable,
    project,
    projectId,
  }: { variable: VariableDef; project: Project; projectId: string } = $props();

  let busy = $state(false);
  let err = $state("");

  const multi = $derived(variable.type === "images");
  const max = $derived(variable.max ?? 0);
  const accept = $derived(
    variable.accept && variable.accept.length
      ? variable.accept.map((e) => "." + e.replace(/^\./, "")).join(",")
      : "image/*,application/pdf",
  );

  function list(): string[] {
    const v = project.values[variable.key];
    return Array.isArray(v) ? (v as string[]) : [];
  }
  function single(): string {
    const v = project.values[variable.key];
    return typeof v === "string" ? v : "";
  }
  function url(name: string): string {
    // 值自带路由前缀（assets/… 或 resources/…），统一走挂载根
    return `http://report.localhost/m/${projectId}/${name}`;
  }
  function isPdf(name: string): boolean {
    return name.toLowerCase().endsWith(".pdf");
  }

  const DEFAULT_EXT = ["png", "jpg", "jpeg", "bmp", "webp", "gif", "pdf"];
  function allowed(name: string): boolean {
    const ext = name.split(".").pop()?.toLowerCase() ?? "";
    const allow = variable.accept?.length
      ? variable.accept.map((a) => a.replace(/^\./, "").toLowerCase())
      : DEFAULT_EXT;
    return allow.includes(ext);
  }

  function readAsDataURL(f: File): Promise<string> {
    return new Promise((res, rej) => {
      const r = new FileReader();
      r.onload = () => res(String(r.result));
      r.onerror = () => rej(r.error);
      r.readAsDataURL(f);
    });
  }

  async function onPick(e: Event) {
    const input = e.target as HTMLInputElement;
    const files = Array.from(input.files ?? []);
    input.value = "";
    if (!files.length) return;
    busy = true;
    err = "";
    try {
      for (const f of files) {
        if (!allowed(f.name)) {
          err = `不支持的文件类型：${f.name}`;
          continue;
        }
        if (multi && max > 0 && list().length >= max) {
          err = `最多 ${max} 个文件`;
          break;
        }
        const name = await api.addResource(projectId, f.name, await readAsDataURL(f));
        if (multi) {
          const next = [...list()];
          if (!next.includes(name)) next.push(name);
          project.values[variable.key] = next;
        } else {
          project.values[variable.key] = name;
          break;
        }
      }
    } catch (e) {
      err = String(e);
    } finally {
      busy = false;
    }
  }

  function remove(name: string) {
    if (multi) {
      project.values[variable.key] = list().filter((x) => x !== name);
    } else {
      // 单张清除：恢复变量默认值（模板默认图），无默认则清空
      project.values[variable.key] =
        typeof variable.default === "string" ? variable.default : "";
    }
  }
  function move(i: number, d: number) {
    const arr = [...list()];
    const j = i + d;
    if (j < 0 || j >= arr.length) return;
    [arr[i], arr[j]] = [arr[j], arr[i]];
    project.values[variable.key] = arr;
  }
</script>

{#if multi}
  <div class="grid">
    {#each list() as name, i (name)}
      <div class="cell">
        {#if isPdf(name)}
          <div class="pdf">PDF</div>
        {:else}
          <img src={url(name)} alt="" />
        {/if}
        <button class="x" onclick={() => remove(name)} title="移除">×</button>
        <div class="mv">
          <button onclick={() => move(i, -1)} disabled={i === 0} title="上移">←</button>
          <button
            onclick={() => move(i, 1)}
            disabled={i === list().length - 1}
            title="下移">→</button
          >
        </div>
      </div>
    {/each}
    {#if max === 0 || list().length < max}
      <label class="add" class:busy>
        <input type="file" {accept} multiple onchange={onPick} />
        +
      </label>
    {/if}
  </div>
{:else if single()}
  <div class="single">
    {#if isPdf(single())}
      <div class="pdf">PDF</div>
    {:else}
      <img src={url(single())} alt="" />
    {/if}
    <div class="ops">
      <label class="btn">
        <input type="file" {accept} onchange={onPick} />替换
      </label>
      <button class="btn" onclick={() => remove(single())}>清除</button>
    </div>
  </div>
{:else}
  <label class="btn pick">
    <input type="file" {accept} onchange={onPick} />选择文件
  </label>
{/if}

{#if busy}<span class="hint">上传中…</span>{/if}
{#if err}<span class="err">{err}</span>{/if}

<style>
  .grid {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .cell {
    position: relative;
    width: 96px;
    height: 96px;
    border-radius: 6px;
    overflow: hidden;
    background: #f3f4f6;
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.12);
  }
  .cell img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .pdf,
  .single .pdf {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    font-size: 12px;
    color: var(--muted);
    letter-spacing: 1px;
  }
  .x {
    position: absolute;
    top: 2px;
    right: 2px;
    width: 18px;
    height: 18px;
    padding: 0;
    line-height: 1;
    border-radius: 50%;
    background: rgba(0, 0, 0, 0.55);
    color: #fff;
    font-size: 13px;
  }
  .mv {
    position: absolute;
    bottom: 2px;
    left: 2px;
    display: flex;
    gap: 2px;
  }
  .mv button {
    width: 18px;
    height: 18px;
    padding: 0;
    line-height: 1;
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.45);
    color: #fff;
    font-size: 11px;
  }
  .add {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 96px;
    height: 96px;
    border-radius: 6px;
    border: 1px dashed var(--placeholder, #b1b4bb);
    color: var(--muted);
    font-size: 24px;
    cursor: pointer;
  }
  .add input,
  .btn input {
    display: none;
  }
  .single {
    display: flex;
    align-items: flex-start;
    gap: 12px;
  }
  .single img {
    width: 120px;
    height: 120px;
    object-fit: cover;
    border-radius: 6px;
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.12);
  }
  .single .pdf {
    width: 120px;
    height: 120px;
    border-radius: 6px;
    background: #f3f4f6;
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.12);
  }
  .ops {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 5px 12px;
    font-size: 12px;
    cursor: pointer;
    border-radius: 6px;
    background: var(--surface-50, #f4f4f5);
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.1);
  }
  .btn:hover {
    background: var(--surface-100, #ececee);
  }
  .pick {
    width: fit-content;
  }
  .hint {
    margin-left: 8px;
    font-size: 12px;
    color: var(--muted);
  }
  .err {
    display: block;
    margin-top: 6px;
    font-size: 12px;
    color: var(--danger, #dc2626);
  }
</style>
