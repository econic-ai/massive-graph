let wasm;

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_export_2.set(idx, obj);
    return idx;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

const cachedTextDecoder = (typeof TextDecoder !== 'undefined' ? new TextDecoder('utf-8', { ignoreBOM: true, fatal: true }) : { decode: () => { throw Error('TextDecoder not available') } } );

if (typeof TextDecoder !== 'undefined') { cachedTextDecoder.decode(); };

let cachedUint8ArrayMemory0 = null;

function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.buffer !== wasm.memory.buffer) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return cachedTextDecoder.decode(getUint8ArrayMemory0().slice(ptr, ptr + len));
}

let WASM_VECTOR_LEN = 0;

const cachedTextEncoder = (typeof TextEncoder !== 'undefined' ? new TextEncoder('utf-8') : { encode: () => { throw Error('TextEncoder not available') } } );

const encodeString = function (arg, view) {
    const buf = cachedTextEncoder.encode(arg);
    view.set(buf);
    return {
        read: arg.length,
        written: buf.length
    };
};

function passStringToWasm0(arg, malloc, realloc) {

    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }

    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = encodeString(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

let cachedDataViewMemory0 = null;

function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer !== wasm.memory.buffer) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => {
    wasm.__wbindgen_export_7.get(state.dtor)(state.a, state.b)
});

function makeMutClosure(arg0, arg1, dtor, f) {
    const state = { a: arg0, b: arg1, cnt: 1, dtor };
    const real = (...args) => {
        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            if (--state.cnt === 0) {
                wasm.__wbindgen_export_7.get(state.dtor)(a, state.b);
                CLOSURE_DTORS.unregister(state);
            } else {
                state.a = a;
            }
        }
    };
    real.original = state;
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches && builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_export_2.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}
/**
 * Get version information
 * @returns {string}
 */
export function version() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.version();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

export function init() {
    wasm.init();
}

function __wbg_adapter_44(arg0, arg1, arg2) {
    wasm.closure29_externref_shim(arg0, arg1, arg2);
}

function __wbg_adapter_47(arg0, arg1) {
    wasm._dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h996148edd6c56f98(arg0, arg1);
}

function __wbg_adapter_50(arg0, arg1, arg2) {
    wasm.closure147_externref_shim(arg0, arg1, arg2);
}

function __wbg_adapter_53(arg0, arg1, arg2) {
    wasm.closure145_externref_shim(arg0, arg1, arg2);
}

function __wbg_adapter_246(arg0, arg1, arg2, arg3) {
    wasm.closure197_externref_shim(arg0, arg1, arg2, arg3);
}

const __wbindgen_enum_RtcSdpType = ["offer", "pranswer", "answer", "rollback"];

const __wbindgen_enum_WorkerType = ["classic", "module"];

const BrowserAppFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_browserapp_free(ptr >>> 0, 1));
/**
 * Browser application wrapper for WASM (Main Thread UI Worker)
 */
export class BrowserApp {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        BrowserAppFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_browserapp_free(ptr, 0);
    }
    /**
     * Get the tab ID (method form for JS compatibility)
     * @returns {string}
     */
    get_tab_id() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.browserapp_get_tab_id(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Initialize the BrowserApp with Service Worker and create SharedArrayBuffers
     */
    initialize() {
        const ret = wasm.browserapp_initialize(this.__wbg_ptr);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Get field metadata (version, offset, size)
     * @param {number} field_id
     * @returns {any}
     */
    get_field_info(field_id) {
        const ret = wasm.browserapp_get_field_info(this.__wbg_ptr, field_id);
        return ret;
    }
    /**
     * Send REGISTER_WORKER message to Service Worker
     * @param {string} worker_id
     * @param {string} worker_type
     */
    register_worker(worker_id, worker_type) {
        const ptr0 = passStringToWasm0(worker_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(worker_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.browserapp_register_worker(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Get field value as bytes
     * @param {number} field_id
     * @returns {Uint8Array}
     */
    get_field_as_bytes(field_id) {
        const ret = wasm.browserapp_get_field_as_bytes(this.__wbg_ptr, field_id);
        return ret;
    }
    /**
     * Get field value as number
     * @param {number} field_id
     * @returns {number}
     */
    get_field_as_number(field_id) {
        const ret = wasm.browserapp_get_field_as_number(this.__wbg_ptr, field_id);
        return ret;
    }
    /**
     * Get field value as object (JSON)
     * @param {number} field_id
     * @returns {any}
     */
    get_field_as_object(field_id) {
        const ret = wasm.browserapp_get_field_as_object(this.__wbg_ptr, field_id);
        return ret;
    }
    /**
     * Get field value as string
     * @param {number} field_id
     * @returns {string}
     */
    get_field_as_string(field_id) {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.browserapp_get_field_as_string(this.__wbg_ptr, field_id);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Register to watch a field for changes
     * @param {number} field_id
     */
    register_field_watch(field_id) {
        wasm.browserapp_register_field_watch(this.__wbg_ptr, field_id);
    }
    /**
     * Request a field update
     * @param {number} field_id
     * @param {any} _value
     */
    request_field_update(field_id, _value) {
        const ret = wasm.browserapp_request_field_update(this.__wbg_ptr, field_id, _value);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Spawn a dedicated worker and register it
     * @param {string} worker_type
     * @returns {string}
     */
    spawn_dedicated_worker(worker_type) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(worker_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.browserapp_spawn_dedicated_worker(this.__wbg_ptr, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Unregister field watching
     * @param {number} field_id
     */
    unregister_field_watch(field_id) {
        wasm.browserapp_unregister_field_watch(this.__wbg_ptr, field_id);
    }
    /**
     * Create a new browser application instance
     */
    constructor() {
        const ret = wasm.browserapp_new();
        this.__wbg_ptr = ret >>> 0;
        BrowserAppFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
}

const DedicatedWorkerFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_dedicatedworker_free(ptr >>> 0, 1));
/**
 * Dedicated worker for background processing (WebRTC, Delta, etc.)
 */
export class DedicatedWorker {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        DedicatedWorkerFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_dedicatedworker_free(ptr, 0);
    }
    /**
     * Initialize the worker with SharedArrayBuffers
     * @param {SharedArrayBuffer} control
     * @param {SharedArrayBuffer} notifications
     * @param {SharedArrayBuffer} deltas
     * @param {SharedArrayBuffer} data
     * @param {SharedArrayBuffer} worker
     */
    initialize(control, notifications, deltas, data, worker) {
        wasm.dedicatedworker_initialize(this.__wbg_ptr, control, notifications, deltas, data, worker);
    }
    /**
     * Get the worker type (WebRTCWorker, DeltaProcessor, etc.)
     * @returns {string}
     */
    get worker_type() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.dedicatedworker_worker_type(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Is the worker initialized?
     * @returns {boolean}
     */
    get is_initialized() {
        const ret = wasm.dedicatedworker_is_initialized(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Process a message from the main thread
     * @param {any} message
     */
    process_message(message) {
        const ret = wasm.dedicatedworker_process_message(this.__wbg_ptr, message);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Create a new dedicated worker
     * @param {string} tab_id
     * @param {string} worker_id
     * @param {string} worker_type
     */
    constructor(tab_id, worker_id, worker_type) {
        const ptr0 = passStringToWasm0(tab_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(worker_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(worker_type, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.dedicatedworker_new(ptr0, len0, ptr1, len1, ptr2, len2);
        this.__wbg_ptr = ret >>> 0;
        DedicatedWorkerFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Stop processing
     */
    stop() {
        wasm.dedicatedworker_stop(this.__wbg_ptr);
    }
    /**
     * Start processing
     */
    start() {
        wasm.dedicatedworker_start(this.__wbg_ptr);
    }
    /**
     * Get the tab ID this worker belongs to
     * @returns {string}
     */
    get tab_id() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.dedicatedworker_tab_id(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get the worker ID
     * @returns {string}
     */
    get worker_id() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.dedicatedworker_worker_id(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
}

const ServiceWorkerContextFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_serviceworkercontext_free(ptr >>> 0, 1));
/**
 * Service Worker context for WASM - handles tab tracking and message routing
 */
export class ServiceWorkerContext {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        ServiceWorkerContextFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_serviceworkercontext_free(ptr, 0);
    }
    /**
     * Initialize the context
     */
    initialize() {
        wasm.serviceworkercontext_initialize(this.__wbg_ptr);
    }
    /**
     * Handle incoming messages
     * @param {MessageEvent} event
     */
    handle_message(event) {
        const ret = wasm.serviceworkercontext_handle_message(this.__wbg_ptr, event);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Check if initialized
     * @returns {boolean}
     */
    get is_initialized() {
        const ret = wasm.serviceworkercontext_is_initialized(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Create a new service worker context
     */
    constructor() {
        const ret = wasm.serviceworkercontext_new();
        this.__wbg_ptr = ret >>> 0;
        ServiceWorkerContextFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Get the number of connected tabs
     * @returns {number}
     */
    get tab_count() {
        const ret = wasm.serviceworkercontext_tab_count(this.__wbg_ptr);
        return ret >>> 0;
    }
}

const WebRtcWorkerManagerFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_webrtcworkermanager_free(ptr >>> 0, 1));
/**
 * WebRTC manager for the dedicated worker
 */
export class WebRtcWorkerManager {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WebRtcWorkerManagerFinalization.unregister(this);
        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_webrtcworkermanager_free(ptr, 0);
    }
    /**
     * Check if connected
     * @returns {boolean}
     */
    is_connected() {
        const ret = wasm.webrtcworkermanager_is_connected(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Create a new WebRTC worker manager
     * @param {string | null} [server_url]
     */
    constructor(server_url) {
        var ptr0 = isLikeNone(server_url) ? 0 : passStringToWasm0(server_url, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        const ret = wasm.webrtcworkermanager_new(ptr0, len0);
        this.__wbg_ptr = ret >>> 0;
        WebRtcWorkerManagerFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Close the connection
     * @returns {Promise<void>}
     */
    close() {
        const ret = wasm.webrtcworkermanager_close(this.__wbg_ptr);
        return ret;
    }
    /**
     * Connect to the server
     * @returns {Promise<void>}
     */
    connect() {
        const ret = wasm.webrtcworkermanager_connect(this.__wbg_ptr);
        return ret;
    }
    /**
     * Send a test ping message
     * @param {string} message
     * @returns {Promise<void>}
     */
    send_ping(message) {
        const ptr0 = passStringToWasm0(message, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.webrtcworkermanager_send_ping(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);

            } catch (e) {
                if (module.headers.get('Content-Type') != 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else {
                    throw e;
                }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);

    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };

        } else {
            return instance;
        }
    }
}

function __wbg_get_imports() {
    const imports = {};
    imports.wbg = {};
    imports.wbg.__wbg_async_9ff6d9e405f13772 = function(arg0) {
        const ret = arg0.async;
        return ret;
    };
    imports.wbg.__wbg_buffer_609cc3eee51ed158 = function(arg0) {
        const ret = arg0.buffer;
        return ret;
    };
    imports.wbg.__wbg_call_672a4d21634d4a24 = function() { return handleError(function (arg0, arg1) {
        const ret = arg0.call(arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_call_7cccdd69e0791ae2 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = arg0.call(arg1, arg2);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_candidate_16483e17c3c24717 = function(arg0) {
        const ret = arg0.candidate;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_clearTimeout_5a54f8841c30079a = function(arg0) {
        const ret = clearTimeout(arg0);
        return ret;
    };
    imports.wbg.__wbg_close_314acc3eb034fb66 = function(arg0) {
        arg0.close();
    };
    imports.wbg.__wbg_controller_ad3ef4f431565d93 = function(arg0) {
        const ret = arg0.controller;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_createDataChannel_284b1e73e52d8464 = function(arg0, arg1, arg2, arg3) {
        const ret = arg0.createDataChannel(getStringFromWasm0(arg1, arg2), arg3);
        return ret;
    };
    imports.wbg.__wbg_createOffer_e32f67b73f2e8bf0 = function(arg0, arg1) {
        const ret = arg0.createOffer(arg1);
        return ret;
    };
    imports.wbg.__wbg_data_432d9c3df2630942 = function(arg0) {
        const ret = arg0.data;
        return ret;
    };
    imports.wbg.__wbg_done_769e5ede4b31c67b = function(arg0) {
        const ret = arg0.done;
        return ret;
    };
    imports.wbg.__wbg_entries_3265d4158b33e5dc = function(arg0) {
        const ret = Object.entries(arg0);
        return ret;
    };
    imports.wbg.__wbg_error_7534b8e9a36f1ab4 = function(arg0, arg1) {
        let deferred0_0;
        let deferred0_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            console.error(getStringFromWasm0(arg0, arg1));
        } finally {
            wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
        }
    };
    imports.wbg.__wbg_fetch_b7bf320f681242d2 = function(arg0, arg1) {
        const ret = arg0.fetch(arg1);
        return ret;
    };
    imports.wbg.__wbg_getRandomValues_80578b2ff2a093ba = function() { return handleError(function (arg0) {
        globalThis.crypto.getRandomValues(arg0);
    }, arguments) };
    imports.wbg.__wbg_get_67b2ba62fc30de12 = function() { return handleError(function (arg0, arg1) {
        const ret = Reflect.get(arg0, arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_get_b9b93047fe3cf45b = function(arg0, arg1) {
        const ret = arg0[arg1 >>> 0];
        return ret;
    };
    imports.wbg.__wbg_getwithrefkey_1dc361bd10053bfe = function(arg0, arg1) {
        const ret = arg0[arg1];
        return ret;
    };
    imports.wbg.__wbg_instanceof_ArrayBuffer_e14585432e3737fc = function(arg0) {
        let result;
        try {
            result = arg0 instanceof ArrayBuffer;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Response_f2cc20d9f7dfd644 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof Response;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Uint8Array_17156bcf118086a9 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof Uint8Array;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Window_def73ea0955fc569 = function(arg0) {
        let result;
        try {
            result = arg0 instanceof Window;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_isArray_a1eab7e0d067391b = function(arg0) {
        const ret = Array.isArray(arg0);
        return ret;
    };
    imports.wbg.__wbg_isSafeInteger_343e2beeeece1bb0 = function(arg0) {
        const ret = Number.isSafeInteger(arg0);
        return ret;
    };
    imports.wbg.__wbg_iterator_9a24c88df860dc65 = function() {
        const ret = Symbol.iterator;
        return ret;
    };
    imports.wbg.__wbg_json_1671bfa3e3625686 = function() { return handleError(function (arg0) {
        const ret = arg0.json();
        return ret;
    }, arguments) };
    imports.wbg.__wbg_label_674e88e28b0f69db = function(arg0, arg1) {
        const ret = arg1.label;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_length_a446193dc22c12f8 = function(arg0) {
        const ret = arg0.length;
        return ret;
    };
    imports.wbg.__wbg_length_e2d2a49132c1b256 = function(arg0) {
        const ret = arg0.length;
        return ret;
    };
    imports.wbg.__wbg_log_1ae1e9f741096e91 = function(arg0, arg1) {
        console.log(arg0, arg1);
    };
    imports.wbg.__wbg_log_c222819a41e063d3 = function(arg0) {
        console.log(arg0);
    };
    imports.wbg.__wbg_navigator_1577371c070c8947 = function(arg0) {
        const ret = arg0.navigator;
        return ret;
    };
    imports.wbg.__wbg_new_018dcc2d6c8c2f6a = function() { return handleError(function () {
        const ret = new Headers();
        return ret;
    }, arguments) };
    imports.wbg.__wbg_new_23a2665fac83c611 = function(arg0, arg1) {
        try {
            var state0 = {a: arg0, b: arg1};
            var cb0 = (arg0, arg1) => {
                const a = state0.a;
                state0.a = 0;
                try {
                    return __wbg_adapter_246(a, state0.b, arg0, arg1);
                } finally {
                    state0.a = a;
                }
            };
            const ret = new Promise(cb0);
            return ret;
        } finally {
            state0.a = state0.b = 0;
        }
    };
    imports.wbg.__wbg_new_24b2c5b645cded8d = function() { return handleError(function () {
        const ret = new MessageChannel();
        return ret;
    }, arguments) };
    imports.wbg.__wbg_new_405e22f390576ce2 = function() {
        const ret = new Object();
        return ret;
    };
    imports.wbg.__wbg_new_78feb108b6472713 = function() {
        const ret = new Array();
        return ret;
    };
    imports.wbg.__wbg_new_8a6f238a6ece86ea = function() {
        const ret = new Error();
        return ret;
    };
    imports.wbg.__wbg_new_a12002a7f91c75be = function(arg0) {
        const ret = new Uint8Array(arg0);
        return ret;
    };
    imports.wbg.__wbg_new_b1a33e5095abf678 = function() { return handleError(function (arg0, arg1) {
        const ret = new Worker(getStringFromWasm0(arg0, arg1));
        return ret;
    }, arguments) };
    imports.wbg.__wbg_new_c757c17a3a479543 = function(arg0) {
        const ret = new SharedArrayBuffer(arg0 >>> 0);
        return ret;
    };
    imports.wbg.__wbg_new_e9a4a67dbababe57 = function(arg0) {
        const ret = new Int32Array(arg0);
        return ret;
    };
    imports.wbg.__wbg_newnoargs_105ed471475aaf50 = function(arg0, arg1) {
        const ret = new Function(getStringFromWasm0(arg0, arg1));
        return ret;
    };
    imports.wbg.__wbg_newwithbyteoffsetandlength_d97e637ebe145a9a = function(arg0, arg1, arg2) {
        const ret = new Uint8Array(arg0, arg1 >>> 0, arg2 >>> 0);
        return ret;
    };
    imports.wbg.__wbg_newwithconfiguration_d716d1bf0ec3af80 = function() { return handleError(function (arg0) {
        const ret = new RTCPeerConnection(arg0);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_newwithlength_a381634e90c276d4 = function(arg0) {
        const ret = new Uint8Array(arg0 >>> 0);
        return ret;
    };
    imports.wbg.__wbg_newwithoptions_0419cac3977d7f7f = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = new Worker(getStringFromWasm0(arg0, arg1), arg2);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_newwithstrandinit_06c535e0a867c635 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = new Request(getStringFromWasm0(arg0, arg1), arg2);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_next_25feadfc0913fea9 = function(arg0) {
        const ret = arg0.next;
        return ret;
    };
    imports.wbg.__wbg_next_6574e1a8a62d1055 = function() { return handleError(function (arg0) {
        const ret = arg0.next();
        return ret;
    }, arguments) };
    imports.wbg.__wbg_now_807e54c39636c349 = function() {
        const ret = Date.now();
        return ret;
    };
    imports.wbg.__wbg_of_4a05197bfc89556f = function(arg0, arg1, arg2) {
        const ret = Array.of(arg0, arg1, arg2);
        return ret;
    };
    imports.wbg.__wbg_ok_3aaf32d069979723 = function(arg0) {
        const ret = arg0.ok;
        return ret;
    };
    imports.wbg.__wbg_port1_70af0ea6e4a96f9d = function(arg0) {
        const ret = arg0.port1;
        return ret;
    };
    imports.wbg.__wbg_port2_0584c7f0938b6fe6 = function(arg0) {
        const ret = arg0.port2;
        return ret;
    };
    imports.wbg.__wbg_ports_b00492ca2866b691 = function(arg0) {
        const ret = arg0.ports;
        return ret;
    };
    imports.wbg.__wbg_postMessage_6edafa8f7b9c2f52 = function() { return handleError(function (arg0, arg1) {
        arg0.postMessage(arg1);
    }, arguments) };
    imports.wbg.__wbg_postMessage_9c3d08c52898c574 = function() { return handleError(function (arg0, arg1) {
        arg0.postMessage(arg1);
    }, arguments) };
    imports.wbg.__wbg_postMessage_e55d059efb191dc5 = function() { return handleError(function (arg0, arg1) {
        arg0.postMessage(arg1);
    }, arguments) };
    imports.wbg.__wbg_postMessage_eaed64648caf5119 = function() { return handleError(function (arg0, arg1, arg2) {
        arg0.postMessage(arg1, arg2);
    }, arguments) };
    imports.wbg.__wbg_push_737cfc8c1432c2c6 = function(arg0, arg1) {
        const ret = arg0.push(arg1);
        return ret;
    };
    imports.wbg.__wbg_queueMicrotask_97d92b4fcc8a61c5 = function(arg0) {
        queueMicrotask(arg0);
    };
    imports.wbg.__wbg_queueMicrotask_d3219def82552485 = function(arg0) {
        const ret = arg0.queueMicrotask;
        return ret;
    };
    imports.wbg.__wbg_resolve_4851785c9c5f573d = function(arg0) {
        const ret = Promise.resolve(arg0);
        return ret;
    };
    imports.wbg.__wbg_send_48418c7374637896 = function() { return handleError(function (arg0, arg1, arg2) {
        arg0.send(getArrayU8FromWasm0(arg1, arg2));
    }, arguments) };
    imports.wbg.__wbg_serviceWorker_1cf12ee6ff174f53 = function(arg0) {
        const ret = arg0.serviceWorker;
        return ret;
    };
    imports.wbg.__wbg_setLocalDescription_3b7809adf05c8170 = function(arg0, arg1) {
        const ret = arg0.setLocalDescription(arg1);
        return ret;
    };
    imports.wbg.__wbg_setRemoteDescription_2bd5c020ee5cf31f = function(arg0, arg1) {
        const ret = arg0.setRemoteDescription(arg1);
        return ret;
    };
    imports.wbg.__wbg_setTimeout_db2dbaeefb6f39c7 = function() { return handleError(function (arg0, arg1) {
        const ret = setTimeout(arg0, arg1);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_set_11cd83f45504cedf = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        arg0.set(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_set_37837023f3d740e8 = function(arg0, arg1, arg2) {
        arg0[arg1 >>> 0] = arg2;
    };
    imports.wbg.__wbg_set_3f1d0b984ed272ed = function(arg0, arg1, arg2) {
        arg0[arg1] = arg2;
    };
    imports.wbg.__wbg_set_65595bdd868b3009 = function(arg0, arg1, arg2) {
        arg0.set(arg1, arg2 >>> 0);
    };
    imports.wbg.__wbg_set_bb8cecf6a62b9f46 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = Reflect.set(arg0, arg1, arg2);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_setbody_5923b78a95eedf29 = function(arg0, arg1) {
        arg0.body = arg1;
    };
    imports.wbg.__wbg_setheaders_834c0bdb6a8949ad = function(arg0, arg1) {
        arg0.headers = arg1;
    };
    imports.wbg.__wbg_setid_69c7a6a10123dc92 = function(arg0, arg1) {
        arg0.id = arg1;
    };
    imports.wbg.__wbg_setmaxretransmits_ff7e384cb0d69b44 = function(arg0, arg1) {
        arg0.maxRetransmits = arg1;
    };
    imports.wbg.__wbg_setmaxretransmittime_33eb32c83357485b = function(arg0, arg1) {
        arg0.maxRetransmitTime = arg1;
    };
    imports.wbg.__wbg_setmethod_3c5280fe5d890842 = function(arg0, arg1, arg2) {
        arg0.method = getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_setnegotiated_09517bfd67caa14c = function(arg0, arg1) {
        arg0.negotiated = arg1 !== 0;
    };
    imports.wbg.__wbg_setonconnectionstatechange_43e2a90cecbb0abc = function(arg0, arg1) {
        arg0.onconnectionstatechange = arg1;
    };
    imports.wbg.__wbg_setonerror_57eeef5feb01fe7a = function(arg0, arg1) {
        arg0.onerror = arg1;
    };
    imports.wbg.__wbg_setonerror_7b736fa0ef019542 = function(arg0, arg1) {
        arg0.onerror = arg1;
    };
    imports.wbg.__wbg_setonicecandidate_4482190aeed37791 = function(arg0, arg1) {
        arg0.onicecandidate = arg1;
    };
    imports.wbg.__wbg_setonmessage_23d122da701b8ddb = function(arg0, arg1) {
        arg0.onmessage = arg1;
    };
    imports.wbg.__wbg_setonmessage_5a885b16bdc6dca6 = function(arg0, arg1) {
        arg0.onmessage = arg1;
    };
    imports.wbg.__wbg_setonmessage_6cfa79abe114a189 = function(arg0, arg1) {
        arg0.onmessage = arg1;
    };
    imports.wbg.__wbg_setonopen_64765262b192af8a = function(arg0, arg1) {
        arg0.onopen = arg1;
    };
    imports.wbg.__wbg_setordered_615e09076fb57ed9 = function(arg0, arg1) {
        arg0.ordered = arg1 !== 0;
    };
    imports.wbg.__wbg_setsdp_ac3f34abbbed4f34 = function(arg0, arg1, arg2) {
        arg0.sdp = getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_settype_0c7017dfd63b2ace = function(arg0, arg1) {
        arg0.type = __wbindgen_enum_RtcSdpType[arg1];
    };
    imports.wbg.__wbg_settype_47fae7d6c82625e7 = function(arg0, arg1) {
        arg0.type = __wbindgen_enum_WorkerType[arg1];
    };
    imports.wbg.__wbg_stack_0ed75d68575b0f3c = function(arg0, arg1) {
        const ret = arg1.stack;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_static_accessor_GLOBAL_88a902d13a557d07 = function() {
        const ret = typeof global === 'undefined' ? null : global;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_static_accessor_GLOBAL_THIS_56578be7e9f832b0 = function() {
        const ret = typeof globalThis === 'undefined' ? null : globalThis;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_static_accessor_SELF_37c5d418e4bf5819 = function() {
        const ret = typeof self === 'undefined' ? null : self;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_static_accessor_WINDOW_5de37043a91a9c40 = function() {
        const ret = typeof window === 'undefined' ? null : window;
        return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
    };
    imports.wbg.__wbg_status_f6360336ca686bf0 = function(arg0) {
        const ret = arg0.status;
        return ret;
    };
    imports.wbg.__wbg_stringify_f7ed6987935b4a24 = function() { return handleError(function (arg0) {
        const ret = JSON.stringify(arg0);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_subarray_aa9065fa9dc5df96 = function(arg0, arg1, arg2) {
        const ret = arg0.subarray(arg1 >>> 0, arg2 >>> 0);
        return ret;
    };
    imports.wbg.__wbg_then_44b73946d2fb3e7d = function(arg0, arg1) {
        const ret = arg0.then(arg1);
        return ret;
    };
    imports.wbg.__wbg_then_48b406749878a531 = function(arg0, arg1, arg2) {
        const ret = arg0.then(arg1, arg2);
        return ret;
    };
    imports.wbg.__wbg_value_cd1ffa7b1ab794f1 = function(arg0) {
        const ret = arg0.value;
        return ret;
    };
    imports.wbg.__wbg_value_dab73d3d5d4abaaf = function(arg0) {
        const ret = arg0.value;
        return ret;
    };
    imports.wbg.__wbg_waitAsync_61f0a081053dd3c2 = function(arg0, arg1, arg2) {
        const ret = Atomics.waitAsync(arg0, arg1 >>> 0, arg2);
        return ret;
    };
    imports.wbg.__wbg_waitAsync_7ce6c8a047c752c3 = function() {
        const ret = Atomics.waitAsync;
        return ret;
    };
    imports.wbg.__wbindgen_as_number = function(arg0) {
        const ret = +arg0;
        return ret;
    };
    imports.wbg.__wbindgen_boolean_get = function(arg0) {
        const v = arg0;
        const ret = typeof(v) === 'boolean' ? (v ? 1 : 0) : 2;
        return ret;
    };
    imports.wbg.__wbindgen_cb_drop = function(arg0) {
        const obj = arg0.original;
        if (obj.cnt-- == 1) {
            obj.a = 0;
            return true;
        }
        const ret = false;
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper2090 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 122, __wbg_adapter_47);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper261 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 30, __wbg_adapter_44);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper2969 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 148, __wbg_adapter_50);
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper2971 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 146, __wbg_adapter_53);
        return ret;
    };
    imports.wbg.__wbindgen_debug_string = function(arg0, arg1) {
        const ret = debugString(arg1);
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbindgen_error_new = function(arg0, arg1) {
        const ret = new Error(getStringFromWasm0(arg0, arg1));
        return ret;
    };
    imports.wbg.__wbindgen_in = function(arg0, arg1) {
        const ret = arg0 in arg1;
        return ret;
    };
    imports.wbg.__wbindgen_init_externref_table = function() {
        const table = wasm.__wbindgen_export_2;
        const offset = table.grow(4);
        table.set(0, undefined);
        table.set(offset + 0, undefined);
        table.set(offset + 1, null);
        table.set(offset + 2, true);
        table.set(offset + 3, false);
        ;
    };
    imports.wbg.__wbindgen_is_function = function(arg0) {
        const ret = typeof(arg0) === 'function';
        return ret;
    };
    imports.wbg.__wbindgen_is_object = function(arg0) {
        const val = arg0;
        const ret = typeof(val) === 'object' && val !== null;
        return ret;
    };
    imports.wbg.__wbindgen_is_string = function(arg0) {
        const ret = typeof(arg0) === 'string';
        return ret;
    };
    imports.wbg.__wbindgen_is_undefined = function(arg0) {
        const ret = arg0 === undefined;
        return ret;
    };
    imports.wbg.__wbindgen_jsval_loose_eq = function(arg0, arg1) {
        const ret = arg0 == arg1;
        return ret;
    };
    imports.wbg.__wbindgen_link_9579f016b4522a24 = function(arg0) {
        const val = `onmessage = function (ev) {
            let [ia, index, value] = ev.data;
            ia = new Int32Array(ia.buffer);
            let result = Atomics.wait(ia, index, value);
            postMessage(result);
        };
        `;
        const ret = typeof URL.createObjectURL === 'undefined' ? "data:application/javascript," + encodeURIComponent(val) : URL.createObjectURL(new Blob([val], { type: "text/javascript" }));
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbindgen_memory = function() {
        const ret = wasm.memory;
        return ret;
    };
    imports.wbg.__wbindgen_number_get = function(arg0, arg1) {
        const obj = arg1;
        const ret = typeof(obj) === 'number' ? obj : undefined;
        getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
    };
    imports.wbg.__wbindgen_number_new = function(arg0) {
        const ret = arg0;
        return ret;
    };
    imports.wbg.__wbindgen_rethrow = function(arg0) {
        throw arg0;
    };
    imports.wbg.__wbindgen_string_get = function(arg0, arg1) {
        const obj = arg1;
        const ret = typeof(obj) === 'string' ? obj : undefined;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbindgen_string_new = function(arg0, arg1) {
        const ret = getStringFromWasm0(arg0, arg1);
        return ret;
    };
    imports.wbg.__wbindgen_throw = function(arg0, arg1) {
        throw new Error(getStringFromWasm0(arg0, arg1));
    };

    return imports;
}

function __wbg_init_memory(imports, memory) {
    imports.wbg.memory = memory || new WebAssembly.Memory({initial:20,maximum:1024,shared:true});
}

function __wbg_finalize_init(instance, module, thread_stack_size) {
    wasm = instance.exports;
    __wbg_init.__wbindgen_wasm_module = module;
    cachedDataViewMemory0 = null;
    cachedUint8ArrayMemory0 = null;

    if (typeof thread_stack_size !== 'undefined' && (typeof thread_stack_size !== 'number' || thread_stack_size === 0 || thread_stack_size % 65536 !== 0)) { throw 'invalid stack size' }
    wasm.__wbindgen_start(thread_stack_size);
    return wasm;
}

function initSync(module, memory) {
    if (wasm !== undefined) return wasm;

    let thread_stack_size
    if (typeof module !== 'undefined') {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module, memory, thread_stack_size} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();

    __wbg_init_memory(imports, memory);

    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }

    const instance = new WebAssembly.Instance(module, imports);

    return __wbg_finalize_init(instance, module, thread_stack_size);
}

async function __wbg_init(module_or_path, memory) {
    if (wasm !== undefined) return wasm;

    let thread_stack_size
    if (typeof module_or_path !== 'undefined') {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path, memory, thread_stack_size} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (typeof module_or_path === 'undefined') {
        module_or_path = new URL('massive_graph_browser_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    __wbg_init_memory(imports, memory);

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module, thread_stack_size);
}

export { initSync };
export default __wbg_init;
