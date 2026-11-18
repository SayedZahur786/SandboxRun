export type Language = "python" | "javascript";
export type ExecutionStatus =
  | "ok"
  | "guest_error"
  | "fuel_limit"
  | "time_limit"
  | "memory_limit"
  | "output_limit"
  | "capability_denied"
  | "trap";
export interface Limits {
  fuel: number;
  memory_bytes: number;
  wall_time_ms: number;
  output_bytes: number;
}
export interface ExecuteRequest {
  language: Language;
  code: string;
  stdin?: string;
  limits?: Partial<Limits>;
}
/** A completed guest run is returned even when the guest fails. */
export interface ExecutionReceipt {
  request_id: string;
  status: ExecutionStatus;
  stdout: string;
  stderr: string;
  duration_ms: number;
  fuel_consumed: number;
  limits: Limits;
  runtime: string | Record<string, unknown>;
  output_truncated: boolean;
}
export class SandboxRunError extends Error {
  constructor(public readonly status: number, public readonly body: unknown) {
    super(`SandboxRun request rejected (HTTP ${status})`);
    this.name = "SandboxRunError";
  }
}
export interface ClientOptions {
  baseUrl: string;
  apiKey: string;
  /** Injectable transport for tests, tracing, or proxies. */
  fetch?: typeof globalThis.fetch;
}
export class SandboxRun {
  private readonly baseUrl: string;
  private readonly apiKey: string;
  private readonly transport: typeof globalThis.fetch;
  constructor(options: ClientOptions) {
    const url = new URL(options.baseUrl);
    if (!['http:', 'https:'].includes(url.protocol)) throw new TypeError('Expected an HTTP(S) base URL');
    if (url.username || url.password || url.search || url.hash) throw new TypeError('Base URL must not contain credentials, query, or fragment');
    this.baseUrl = options.baseUrl.replace(/\/+$/, '');
    this.apiKey = options.apiKey;
    this.transport = options.fetch ?? globalThis.fetch;
  }
  /** No automatic retries: each invocation represents exactly one execution request. */
  async run(request: ExecuteRequest, options: { signal?: AbortSignal } = {}): Promise<ExecutionReceipt> {
    return this.request('/api/run', {
      method: 'POST', body: JSON.stringify(request), signal: options.signal,
      headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${this.apiKey}` }
    }) as Promise<ExecutionReceipt>;
  }
  async capabilities(options: { signal?: AbortSignal } = {}): Promise<Record<string, unknown>> {
    return this.request('/api/capabilities', { signal: options.signal }) as Promise<Record<string, unknown>>;
  }
  private async request(path: string, init: RequestInit): Promise<unknown> {
    const response = await this.transport(this.baseUrl + path, { ...init, redirect: 'error' });
    const body: unknown = await response.json().catch(() => ({ error: 'Non-JSON response' }));
    if (!response.ok) throw new SandboxRunError(response.status, body);
    return body;
  }
}
