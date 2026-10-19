import type {
  HookContext,
  ValidationIssue,
  ValidationStatus,
  VariableDef,
} from "./types";

export type HookEvent = HookContext["event"];

export type ReadTemplateText = (
  templateDir: string,
  path: string,
) => Promise<string>;

export interface TemplateValidationOptions {
  templateDir: string;
  hookPath?: string | null;
  variables: VariableDef[];
  readText: ReadTemplateText;
  timeoutMs?: number;
}

const DEFAULT_TIMEOUT_MS = 3000;

const WORKER_BOOTSTRAP = `
self.onmessage = async function (event) {
  var data = event.data || {};
  var id = data.id;
  try {
    var fn = self.__RS_TEMPLATE_VALIDATE__;
    if (typeof fn !== "function") {
      throw new Error("未找到校验函数 __RS_TEMPLATE_VALIDATE__");
    }
    var result = await fn(data.ctx);
    self.postMessage({ id: id, result: result });
  } catch (err) {
    var message = err && err.message ? err.message : String(err);
    self.postMessage({ id: id, error: message });
  }
};
`;

interface QueuedRun {
  event: HookEvent;
  values: Record<string, unknown>;
  rawValues: Record<string, unknown>;
  changedKey?: string;
  changedValue?: unknown;
  resolve: (issues: ValidationIssue[]) => void;
}

interface ActiveRun extends QueuedRun {
  id: number;
  timer: number;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === "object" && !Array.isArray(value);
}

/**
 * Svelte 的 $state 是 Proxy，直接 postMessage 可能触发 DataCloneError。
 * 变量数据本身就是 JSON 结构，这里统一转成普通对象再发进 Worker。
 */
function cloneJson<T>(value: T): T {
  try {
    return JSON.parse(JSON.stringify(value)) as T;
  } catch {
    return value;
  }
}

function asString(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

function normalizeStatus(item: Record<string, unknown>): ValidationStatus {
  const raw = String(item.status ?? item.severity ?? "").toLowerCase();
  if (["ok", "pass", "passed", "success", "valid"].includes(raw)) return "ok";
  if (["warning", "warn"].includes(raw)) return "warning";
  if (["error", "invalid", "fail", "failed"].includes(raw)) return "error";

  if (typeof item.ok === "boolean") return item.ok ? "ok" : "error";
  if (typeof item.valid === "boolean") return item.valid ? "ok" : "error";
  return "error";
}

function normalizeItem(
  raw: unknown,
  fallbackKey?: string,
): ValidationIssue[] {
  if (raw === null || raw === undefined) return [];

  if (typeof raw === "boolean") {
    if (raw) return [];
    return [
      {
        status: "error",
        keys: fallbackKey ? [fallbackKey] : [],
        msg: "校验未通过",
      },
    ];
  }

  if (typeof raw === "string") {
    return [
      {
        status: "error",
        keys: fallbackKey ? [fallbackKey] : [],
        msg: raw,
      },
    ];
  }

  if (!isRecord(raw)) return [];

  const status = normalizeStatus(raw);
  let keys: string[] = [];
  if (Array.isArray(raw.keys)) {
    keys = raw.keys.filter((key): key is string => typeof key === "string");
  }

  const key =
    asString(raw.key) ?? asString(raw.field) ?? asString(raw.fieldKey);
  if (key && !keys.includes(key)) keys = [key, ...keys];
  if (!keys.length && fallbackKey) keys = [fallbackKey];

  const msg =
    asString(raw.msg) ??
    asString(raw.message) ??
    (status === "ok" ? "" : status === "warning" ? "校验警告" : "校验未通过");

  return [
    {
      status,
      keys,
      key: key ?? keys[0],
      msg,
    },
  ];
}

export function normalizeHookResult(
  raw: unknown,
  fallbackKey?: string,
): ValidationIssue[] {
  if (raw === true || raw === null || raw === undefined) return [];
  if (raw === false) {
    return [
      {
        status: "error",
        keys: fallbackKey ? [fallbackKey] : [],
        msg: "校验未通过",
      },
    ];
  }
  if (typeof raw === "string") return normalizeItem(raw, fallbackKey);
  if (Array.isArray(raw)) {
    return raw.flatMap((item) => normalizeItem(item, fallbackKey));
  }
  if (isRecord(raw) && Array.isArray(raw.issues)) {
    return raw.issues.flatMap((item) => normalizeItem(item, fallbackKey));
  }
  return normalizeItem(raw, fallbackKey);
}

/**
 * 每个项目创建一个实例。hook 源码在第一次校验时读取，Worker 按需创建。
 * Worker 内没有 DOM 和 Tauri 权限，超时后会 terminate 并在下次重建。
 *
 * 请求按顺序执行，避免快速输入时多个请求同时占用 Worker。
 */
export class TemplateValidation {
  private worker: Worker | null = null;
  private loading: Promise<Worker> | null = null;
  private blobUrl: string | null = null;
  private queue: QueuedRun[] = [];
  private current: ActiveRun | null = null;
  private seq = 0;
  private disposed = false;
  private readonly timeoutMs: number;

  constructor(private readonly opts: TemplateValidationOptions) {
    this.timeoutMs = opts.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  }

  get hasHook(): boolean {
    return !!this.opts.hookPath;
  }

  run(
    event: HookEvent,
    values: Record<string, unknown>,
    rawValues: Record<string, unknown>,
    changedKey?: string,
    changedValue?: unknown,
  ): Promise<ValidationIssue[]> {
    if (this.disposed || !this.opts.hookPath) return Promise.resolve([]);

    return new Promise<ValidationIssue[]>((resolve) => {
      this.queue.push({
        event,
        values,
        rawValues,
        changedKey,
        changedValue,
        resolve,
      });
      this.pump();
    });
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;

    if (this.current) {
      window.clearTimeout(this.current.timer);
      this.current.resolve([this.cancelledIssue(this.current.changedKey)]);
      this.current = null;
    }
    for (const item of this.queue) {
      item.resolve([this.cancelledIssue(item.changedKey)]);
    }
    this.queue = [];
    this.stopWorker();
  }

  private pump(): void {
    if (this.disposed || this.current) return;
    const next = this.queue.shift();
    if (!next) return;

    this.ensureWorker().then(
      (worker) => {
        if (this.disposed) {
          next.resolve([]);
          return;
        }
        const id = ++this.seq;
        const ctx: HookContext = {
          event: next.event,
          key: next.changedKey,
          value: cloneJson(next.changedValue),
          values: cloneJson(next.values),
          rawValues: cloneJson(next.rawValues),
          variables: cloneJson(this.opts.variables),
        };
        const timer = window.setTimeout(() => this.onTimeout(id), this.timeoutMs);
        this.current = { ...next, id, timer };

        try {
          worker.postMessage({ id, ctx });
        } catch (error) {
          this.current = null;
          window.clearTimeout(timer);
          next.resolve([this.failureIssue(error, next.changedKey)]);
          this.stopWorker();
          this.pump();
        }
      },
      (error) => {
        next.resolve([this.failureIssue(error, next.changedKey)]);
        this.pump();
      },
    );
  }

  private onTimeout(id: number): void {
    if (!this.current || this.current.id !== id) return;
    const active = this.current;
    this.current = null;
    active.resolve([this.timeoutIssue(active.changedKey)]);
    this.failWorker("校验脚本执行超时");
  }

  private async ensureWorker(): Promise<Worker> {
    if (this.disposed) throw new Error("校验器已释放");
    if (this.worker) return this.worker;
    if (!this.loading) {
      this.loading = this.createWorker().then(
        (worker) => {
          this.loading = null;
          return worker;
        },
        (error) => {
          this.loading = null;
          throw error;
        },
      );
    }
    return this.loading;
  }

  private async createWorker(): Promise<Worker> {
    const hookPath = this.opts.hookPath;
    if (!hookPath) throw new Error("模板未声明校验钩子");

    const source = await this.opts.readText(this.opts.templateDir, hookPath);
    const code = [
      "self.window = self;\n",
      source,
      "\nself.__RS_TEMPLATE_VALIDATE__ = self.__RS_TEMPLATE_VALIDATE__ || self.templateValidate;\n",
      WORKER_BOOTSTRAP,
    ].join("");

    const url = URL.createObjectURL(
      new Blob([code], { type: "text/javascript" }),
    );
    this.blobUrl = url;
    let worker: Worker;
    try {
      worker = new Worker(url);
    } catch (error) {
      URL.revokeObjectURL(url);
      this.blobUrl = null;
      throw error;
    }

    if (this.disposed) {
      worker.terminate();
      URL.revokeObjectURL(url);
      this.blobUrl = null;
      throw new Error("校验器已释放");
    }

    worker.onmessage = (event: MessageEvent) => this.onMessage(event);
    worker.onerror = (event: ErrorEvent) => {
      event.preventDefault();
      this.failWorker(event.message || "校验脚本执行失败");
    };
    this.worker = worker;
    return worker;
  }

  private onMessage(event: MessageEvent): void {
    if (this.disposed) return;
    const data = event.data as {
      id?: unknown;
      result?: unknown;
      error?: unknown;
    };
    const id = typeof data?.id === "number" ? data.id : -1;
    const active = this.current;
    if (!active || active.id !== id) return;

    this.current = null;
    window.clearTimeout(active.timer);

    if (typeof data.error === "string") {
      active.resolve([
        {
          status: "error",
          keys: active.changedKey ? [active.changedKey] : [],
          msg: `校验脚本执行失败：${data.error}`,
        },
      ]);
    } else {
      active.resolve(normalizeHookResult(data.result, active.changedKey));
    }
    this.pump();
  }

  private failWorker(message: string): void {
    if (this.current) {
      const active = this.current;
      this.current = null;
      window.clearTimeout(active.timer);
      active.resolve([this.errorIssue(message, active.changedKey)]);
    }
    for (const item of this.queue) {
      item.resolve([this.errorIssue(message, item.changedKey)]);
    }
    this.queue = [];
    this.stopWorker();
  }

  private stopWorker(): void {
    this.worker?.terminate();
    this.worker = null;
    this.loading = null;
    if (this.blobUrl) {
      URL.revokeObjectURL(this.blobUrl);
      this.blobUrl = null;
    }
  }

  private errorIssue(
    message: string,
    key?: string,
  ): ValidationIssue {
    return {
      status: "error",
      keys: key ? [key] : [],
      msg: message,
    };
  }

  private failureIssue(error: unknown, key?: string): ValidationIssue {
    return this.errorIssue(`校验脚本加载失败：${String(error)}`, key);
  }

  private timeoutIssue(key?: string): ValidationIssue {
    return this.errorIssue("校验脚本执行超时", key);
  }

  private cancelledIssue(key?: string): ValidationIssue {
    return this.errorIssue("校验已取消", key);
  }
}
