let wasm;
export function __wbg_set_wasm(val) {
    wasm = val;
}


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

let WASM_VECTOR_LEN = 0;

let cachedUint8ArrayMemory0 = null;

function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.buffer !== wasm.memory.buffer) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

const lTextEncoder = typeof TextEncoder === 'undefined' ? (0, module.require)('util').TextEncoder : TextEncoder;

let cachedTextEncoder = new lTextEncoder('utf-8');

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

const lTextDecoder = typeof TextDecoder === 'undefined' ? (0, module.require)('util').TextDecoder : TextDecoder;

let cachedTextDecoder = new lTextDecoder('utf-8', { ignoreBOM: true, fatal: true });

cachedTextDecoder.decode();

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return cachedTextDecoder.decode(getUint8ArrayMemory0().slice(ptr, ptr + len));
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

function __wbg_adapter_22(arg0, arg1, arg2) {
    wasm.closure14_externref_shim(arg0, arg1, arg2);
}

function __wbg_adapter_25(arg0, arg1, arg2) {
    wasm.closure16_externref_shim(arg0, arg1, arg2);
}

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

export function __wbg_buffer_609cc3eee51ed158(arg0) {
    const ret = arg0.buffer;
    return ret;
};

export function __wbg_call_672a4d21634d4a24() { return handleError(function (arg0, arg1) {
    const ret = arg0.call(arg1);
    return ret;
}, arguments) };

export function __wbg_controller_ad3ef4f431565d93(arg0) {
    const ret = arg0.controller;
    return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
};

export function __wbg_createObjectURL_6e98d2f9c7bd9764() { return handleError(function (arg0, arg1) {
    const ret = URL.createObjectURL(arg1);
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
}, arguments) };

export function __wbg_data_432d9c3df2630942(arg0) {
    const ret = arg0.data;
    return ret;
};

export function __wbg_error_7534b8e9a36f1ab4(arg0, arg1) {
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

export function __wbg_getRandomValues_80578b2ff2a093ba() { return handleError(function (arg0) {
    globalThis.crypto.getRandomValues(arg0);
}, arguments) };

export function __wbg_get_67b2ba62fc30de12() { return handleError(function (arg0, arg1) {
    const ret = Reflect.get(arg0, arg1);
    return ret;
}, arguments) };

export function __wbg_get_b9b93047fe3cf45b(arg0, arg1) {
    const ret = arg0[arg1 >>> 0];
    return ret;
};

export function __wbg_instanceof_Window_def73ea0955fc569(arg0) {
    let result;
    try {
        result = arg0 instanceof Window;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
};

export function __wbg_length_a446193dc22c12f8(arg0) {
    const ret = arg0.length;
    return ret;
};

export function __wbg_length_e2d2a49132c1b256(arg0) {
    const ret = arg0.length;
    return ret;
};

export function __wbg_log_1ae1e9f741096e91(arg0, arg1) {
    console.log(arg0, arg1);
};

export function __wbg_log_c222819a41e063d3(arg0) {
    console.log(arg0);
};

export function __wbg_message_d1685a448ba00178(arg0, arg1) {
    const ret = arg1.message;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
};

export function __wbg_navigator_1577371c070c8947(arg0) {
    const ret = arg0.navigator;
    return ret;
};

export function __wbg_new_24b2c5b645cded8d() { return handleError(function () {
    const ret = new MessageChannel();
    return ret;
}, arguments) };

export function __wbg_new_405e22f390576ce2() {
    const ret = new Object();
    return ret;
};

export function __wbg_new_78feb108b6472713() {
    const ret = new Array();
    return ret;
};

export function __wbg_new_8a6f238a6ece86ea() {
    const ret = new Error();
    return ret;
};

export function __wbg_new_a12002a7f91c75be(arg0) {
    const ret = new Uint8Array(arg0);
    return ret;
};

export function __wbg_new_b1a33e5095abf678() { return handleError(function (arg0, arg1) {
    const ret = new Worker(getStringFromWasm0(arg0, arg1));
    return ret;
}, arguments) };

export function __wbg_new_c757c17a3a479543(arg0) {
    const ret = new SharedArrayBuffer(arg0 >>> 0);
    return ret;
};

export function __wbg_newnoargs_105ed471475aaf50(arg0, arg1) {
    const ret = new Function(getStringFromWasm0(arg0, arg1));
    return ret;
};

export function __wbg_newwithbyteoffsetandlength_d97e637ebe145a9a(arg0, arg1, arg2) {
    const ret = new Uint8Array(arg0, arg1 >>> 0, arg2 >>> 0);
    return ret;
};

export function __wbg_newwithlength_a381634e90c276d4(arg0) {
    const ret = new Uint8Array(arg0 >>> 0);
    return ret;
};

export function __wbg_newwithstrsequenceandoptions_aaff55b467c81b63() { return handleError(function (arg0, arg1) {
    const ret = new Blob(arg0, arg1);
    return ret;
}, arguments) };

export function __wbg_now_807e54c39636c349() {
    const ret = Date.now();
    return ret;
};

export function __wbg_of_2eaf5a02d443ef03(arg0) {
    const ret = Array.of(arg0);
    return ret;
};

export function __wbg_port1_70af0ea6e4a96f9d(arg0) {
    const ret = arg0.port1;
    return ret;
};

export function __wbg_port2_0584c7f0938b6fe6(arg0) {
    const ret = arg0.port2;
    return ret;
};

export function __wbg_ports_b00492ca2866b691(arg0) {
    const ret = arg0.ports;
    return ret;
};

export function __wbg_postMessage_6edafa8f7b9c2f52() { return handleError(function (arg0, arg1) {
    arg0.postMessage(arg1);
}, arguments) };

export function __wbg_postMessage_9c3d08c52898c574() { return handleError(function (arg0, arg1) {
    arg0.postMessage(arg1);
}, arguments) };

export function __wbg_postMessage_e55d059efb191dc5() { return handleError(function (arg0, arg1) {
    arg0.postMessage(arg1);
}, arguments) };

export function __wbg_postMessage_eaed64648caf5119() { return handleError(function (arg0, arg1, arg2) {
    arg0.postMessage(arg1, arg2);
}, arguments) };

export function __wbg_push_737cfc8c1432c2c6(arg0, arg1) {
    const ret = arg0.push(arg1);
    return ret;
};

export function __wbg_revokeObjectURL_27267efebeb457c7() { return handleError(function (arg0, arg1) {
    URL.revokeObjectURL(getStringFromWasm0(arg0, arg1));
}, arguments) };

export function __wbg_serviceWorker_1cf12ee6ff174f53(arg0) {
    const ret = arg0.serviceWorker;
    return ret;
};

export function __wbg_set_65595bdd868b3009(arg0, arg1, arg2) {
    arg0.set(arg1, arg2 >>> 0);
};

export function __wbg_set_bb8cecf6a62b9f46() { return handleError(function (arg0, arg1, arg2) {
    const ret = Reflect.set(arg0, arg1, arg2);
    return ret;
}, arguments) };

export function __wbg_setonerror_57eeef5feb01fe7a(arg0, arg1) {
    arg0.onerror = arg1;
};

export function __wbg_setonmessage_23d122da701b8ddb(arg0, arg1) {
    arg0.onmessage = arg1;
};

export function __wbg_setonmessage_5a885b16bdc6dca6(arg0, arg1) {
    arg0.onmessage = arg1;
};

export function __wbg_settype_39ed370d3edd403c(arg0, arg1, arg2) {
    arg0.type = getStringFromWasm0(arg1, arg2);
};

export function __wbg_stack_0ed75d68575b0f3c(arg0, arg1) {
    const ret = arg1.stack;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
};

export function __wbg_static_accessor_GLOBAL_88a902d13a557d07() {
    const ret = typeof global === 'undefined' ? null : global;
    return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
};

export function __wbg_static_accessor_GLOBAL_THIS_56578be7e9f832b0() {
    const ret = typeof globalThis === 'undefined' ? null : globalThis;
    return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
};

export function __wbg_static_accessor_SELF_37c5d418e4bf5819() {
    const ret = typeof self === 'undefined' ? null : self;
    return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
};

export function __wbg_static_accessor_WINDOW_5de37043a91a9c40() {
    const ret = typeof window === 'undefined' ? null : window;
    return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
};

export function __wbg_subarray_aa9065fa9dc5df96(arg0, arg1, arg2) {
    const ret = arg0.subarray(arg1 >>> 0, arg2 >>> 0);
    return ret;
};

export function __wbindgen_closure_wrapper891(arg0, arg1, arg2) {
    const ret = makeMutClosure(arg0, arg1, 15, __wbg_adapter_22);
    return ret;
};

export function __wbindgen_closure_wrapper893(arg0, arg1, arg2) {
    const ret = makeMutClosure(arg0, arg1, 17, __wbg_adapter_25);
    return ret;
};

export function __wbindgen_debug_string(arg0, arg1) {
    const ret = debugString(arg1);
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
};

export function __wbindgen_init_externref_table() {
    const table = wasm.__wbindgen_export_2;
    const offset = table.grow(4);
    table.set(0, undefined);
    table.set(offset + 0, undefined);
    table.set(offset + 1, null);
    table.set(offset + 2, true);
    table.set(offset + 3, false);
    ;
};

export function __wbindgen_is_undefined(arg0) {
    const ret = arg0 === undefined;
    return ret;
};

export function __wbindgen_memory() {
    const ret = wasm.memory;
    return ret;
};

export function __wbindgen_number_new(arg0) {
    const ret = arg0;
    return ret;
};

export function __wbindgen_string_get(arg0, arg1) {
    const obj = arg1;
    const ret = typeof(obj) === 'string' ? obj : undefined;
    var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
    getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
};

export function __wbindgen_string_new(arg0, arg1) {
    const ret = getStringFromWasm0(arg0, arg1);
    return ret;
};

export function __wbindgen_throw(arg0, arg1) {
    throw new Error(getStringFromWasm0(arg0, arg1));
};

