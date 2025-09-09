/* tslint:disable */
/* eslint-disable */
export function init(): void;
/**
 * Get version information
 */
export function version(): string;
/**
 * Browser application wrapper for WASM (Main Thread UI Worker)
 */
export class BrowserApp {
  free(): void;
  /**
   * Get the tab ID (method form for JS compatibility)
   */
  get_tab_id(): string;
  /**
   * Initialize the BrowserApp with Service Worker and create SharedArrayBuffers
   */
  initialize(): void;
  /**
   * Get field metadata (version, offset, size)
   */
  get_field_info(field_id: number): any;
  /**
   * Send REGISTER_WORKER message to Service Worker
   */
  register_worker(worker_id: string, worker_type: string): void;
  /**
   * Get field value as bytes
   */
  get_field_as_bytes(field_id: number): Uint8Array;
  /**
   * Get field value as number
   */
  get_field_as_number(field_id: number): number;
  /**
   * Get field value as object (JSON)
   */
  get_field_as_object(field_id: number): any;
  /**
   * Get field value as string
   */
  get_field_as_string(field_id: number): string;
  /**
   * Register to watch a field for changes
   */
  register_field_watch(field_id: number): void;
  /**
   * Request a field update
   */
  request_field_update(field_id: number, _value: any): void;
  /**
   * Spawn a dedicated worker and register it
   */
  spawn_dedicated_worker(worker_type: string): string;
  /**
   * Unregister field watching
   */
  unregister_field_watch(field_id: number): void;
  /**
   * Create a new browser application instance
   */
  constructor();
}
/**
 * Dedicated worker for background processing (WebRTC, Delta, etc.)
 */
export class DedicatedWorker {
  free(): void;
  /**
   * Initialize the worker with SharedArrayBuffers
   */
  initialize(control: SharedArrayBuffer, notifications: SharedArrayBuffer, deltas: SharedArrayBuffer, data: SharedArrayBuffer, worker: SharedArrayBuffer): void;
  /**
   * Process a message from the main thread
   */
  process_message(message: any): void;
  /**
   * Create a new dedicated worker
   */
  constructor(tab_id: string, worker_id: string, worker_type: string);
  /**
   * Stop processing
   */
  stop(): void;
  /**
   * Start processing
   */
  start(): void;
  /**
   * Get the worker type (WebRTCWorker, DeltaProcessor, etc.)
   */
  readonly worker_type: string;
  /**
   * Is the worker initialized?
   */
  readonly is_initialized: boolean;
  /**
   * Get the tab ID this worker belongs to
   */
  readonly tab_id: string;
  /**
   * Get the worker ID
   */
  readonly worker_id: string;
}
/**
 * Service Worker context for WASM - handles tab tracking and message routing
 */
export class ServiceWorkerContext {
  free(): void;
  /**
   * Initialize the context
   */
  initialize(): void;
  /**
   * Handle incoming messages
   */
  handle_message(event: MessageEvent): void;
  /**
   * Create a new service worker context
   */
  constructor();
  /**
   * Check if initialized
   */
  readonly is_initialized: boolean;
  /**
   * Get the number of connected tabs
   */
  readonly tab_count: number;
}
/**
 * WebRTC manager for the dedicated worker
 */
export class WebRtcWorkerManager {
  free(): void;
  /**
   * Check if connected
   */
  is_connected(): boolean;
  /**
   * Create a new WebRTC worker manager
   */
  constructor(server_url?: string | null);
  /**
   * Close the connection
   */
  close(): Promise<void>;
  /**
   * Connect to the server
   */
  connect(): Promise<void>;
  /**
   * Send a test ping message
   */
  send_ping(message: string): Promise<void>;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly __wbg_webrtcworkermanager_free: (a: number, b: number) => void;
  readonly webrtcworkermanager_close: (a: number) => any;
  readonly webrtcworkermanager_connect: (a: number) => any;
  readonly webrtcworkermanager_is_connected: (a: number) => number;
  readonly webrtcworkermanager_new: (a: number, b: number) => number;
  readonly webrtcworkermanager_send_ping: (a: number, b: number, c: number) => any;
  readonly init: () => void;
  readonly version: () => [number, number];
  readonly __wbg_dedicatedworker_free: (a: number, b: number) => void;
  readonly dedicatedworker_initialize: (a: number, b: any, c: any, d: any, e: any, f: any) => void;
  readonly dedicatedworker_is_initialized: (a: number) => number;
  readonly dedicatedworker_new: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
  readonly dedicatedworker_process_message: (a: number, b: any) => [number, number];
  readonly dedicatedworker_start: (a: number) => void;
  readonly dedicatedworker_stop: (a: number) => void;
  readonly dedicatedworker_tab_id: (a: number) => [number, number];
  readonly dedicatedworker_worker_id: (a: number) => [number, number];
  readonly dedicatedworker_worker_type: (a: number) => [number, number];
  readonly __wbg_browserapp_free: (a: number, b: number) => void;
  readonly browserapp_get_field_as_bytes: (a: number, b: number) => any;
  readonly browserapp_get_field_as_number: (a: number, b: number) => number;
  readonly browserapp_get_field_as_object: (a: number, b: number) => any;
  readonly browserapp_get_field_as_string: (a: number, b: number) => [number, number];
  readonly browserapp_get_field_info: (a: number, b: number) => any;
  readonly browserapp_get_tab_id: (a: number) => [number, number];
  readonly browserapp_initialize: (a: number) => [number, number];
  readonly browserapp_new: () => number;
  readonly browserapp_register_field_watch: (a: number, b: number) => void;
  readonly browserapp_register_worker: (a: number, b: number, c: number, d: number, e: number) => [number, number];
  readonly browserapp_request_field_update: (a: number, b: number, c: any) => [number, number];
  readonly browserapp_spawn_dedicated_worker: (a: number, b: number, c: number) => [number, number, number, number];
  readonly browserapp_unregister_field_watch: (a: number, b: number) => void;
  readonly __wbg_serviceworkercontext_free: (a: number, b: number) => void;
  readonly serviceworkercontext_handle_message: (a: number, b: any) => [number, number];
  readonly serviceworkercontext_initialize: (a: number) => void;
  readonly serviceworkercontext_is_initialized: (a: number) => number;
  readonly serviceworkercontext_new: () => number;
  readonly serviceworkercontext_tab_count: (a: number) => number;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly memory: WebAssembly.Memory;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_export_7: WebAssembly.Table;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly closure69_externref_shim: (a: number, b: number, c: any) => void;
  readonly _dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h996148edd6c56f98: (a: number, b: number) => void;
  readonly closure145_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure143_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure197_externref_shim: (a: number, b: number, c: any, d: any) => void;
  readonly __wbindgen_thread_destroy: (a?: number, b?: number, c?: number) => void;
  readonly __wbindgen_start: (a: number) => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput, memory?: WebAssembly.Memory, thread_stack_size?: number }} module - Passing `SyncInitInput` directly is deprecated.
* @param {WebAssembly.Memory} memory - Deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput, memory?: WebAssembly.Memory, thread_stack_size?: number } | SyncInitInput, memory?: WebAssembly.Memory): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput>, memory?: WebAssembly.Memory, thread_stack_size?: number }} module_or_path - Passing `InitInput` directly is deprecated.
* @param {WebAssembly.Memory} memory - Deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput>, memory?: WebAssembly.Memory, thread_stack_size?: number } | InitInput | Promise<InitInput>, memory?: WebAssembly.Memory): Promise<InitOutput>;
