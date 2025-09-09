use wasm_bindgen::prelude::*;
use web_sys::console;

/// Logs a message with a JavaScript object to the console
/// 
/// # Arguments
/// * `message` - The message prefix (e.g., "📨 Message received:")
/// * `pairs` - A slice of (key, value) tuples to create the JS object
/// 
/// # Example
/// ```
/// log_js_object("📨 Message received:", &[
///     ("type", "PING"),
///     ("tabId", "abc123"),
///     ("timestamp", &timestamp.to_string()),
/// ]);
/// ```
pub fn _log_js_object(message: &str, pairs: &[(&str, &str)]) {
    let obj = js_sys::Object::new();
    
    for (key, value) in pairs {
        js_sys::Reflect::set(&obj, &(*key).into(), &(*value).into()).unwrap();
    }
    
    console::log_2(&message.into(), &obj);
}

/// Macro to log with JS object format
/// 
/// # Example
/// ```
/// log_js!("📨 Message received:", {
///     "type" => msg_type,
///     "tabId" => tab_id,  // Can be Option<String>
///     "count" => count.to_string()
/// });
/// ```
/// Log to JavaScript console with structured data
#[macro_export]
macro_rules! log_js {
    ($message:expr, { $($key:expr => $value:expr),* $(,)? }) => {
        {
            let obj = js_sys::Object::new();
            $(
                let val = $crate::utils::OptionToJsValue::to_js_value(&$value);
                if !val.is_undefined() {
                    js_sys::Reflect::set(&obj, &$key.into(), &val).unwrap();
                }
            )*
            web_sys::console::log_2(&$message.into(), &obj);
        }
    };
}

/// Helper trait to convert values for JS logging
pub trait OptionToJsValue {
    fn to_js_value(&self) -> JsValue;
}

impl OptionToJsValue for Option<String> {
    fn to_js_value(&self) -> JsValue {
        match self {
            Some(v) => v.clone().into(),
            None => JsValue::UNDEFINED,
        }
    }
}

impl OptionToJsValue for String {
    fn to_js_value(&self) -> JsValue {
        self.clone().into()
    }
}

impl OptionToJsValue for &String {
    fn to_js_value(&self) -> JsValue {
        (*self).clone().into()
    }
}

impl OptionToJsValue for &str {
    fn to_js_value(&self) -> JsValue {
        (*self).into()
    }
}

impl OptionToJsValue for &Option<String> {
    fn to_js_value(&self) -> JsValue {
        match self {
            Some(v) => v.clone().into(),
            None => JsValue::UNDEFINED,
        }
    }
}

// Add implementations for numbers
impl OptionToJsValue for i32 {
    fn to_js_value(&self) -> JsValue {
        (*self).into()
    }
}

impl OptionToJsValue for &i32 {
    fn to_js_value(&self) -> JsValue {
        JsValue::from(**self)
    }
}

impl OptionToJsValue for u32 {
    fn to_js_value(&self) -> JsValue {
        (*self).into()
    }
}

impl OptionToJsValue for &u32 {
    fn to_js_value(&self) -> JsValue {
        JsValue::from(**self)
    }
}

impl OptionToJsValue for bool {
    fn to_js_value(&self) -> JsValue {
        (*self).into()
    }
}

impl OptionToJsValue for &bool {
    fn to_js_value(&self) -> JsValue {
        JsValue::from(**self)
    }
}

// Support for JsValue directly
impl OptionToJsValue for JsValue {
    fn to_js_value(&self) -> JsValue {
        self.clone()
    }
}

impl OptionToJsValue for &JsValue {
    fn to_js_value(&self) -> JsValue {
        (*self).clone()
    }
}

/// Generic wrapper for any value that can be logged
/// This allows us to log any JsValue, including complex objects like MessageEvent
pub struct LogValue<T>(pub T);

// Direct implementation for JsValue references
impl OptionToJsValue for LogValue<&JsValue> {
    fn to_js_value(&self) -> JsValue {
        self.0.clone()
    }
}

// Implementation for owned JsValue
impl OptionToJsValue for LogValue<JsValue> {
    fn to_js_value(&self) -> JsValue {
        self.0.clone()
    }
}

// Implementation for MessageEvent
impl OptionToJsValue for LogValue<&web_sys::MessageEvent> {
    fn to_js_value(&self) -> JsValue {
        self.0.clone().into()
    }
}

// Note: We can't implement AsRef<JsValue> for external types like MessageEvent
// due to Rust's orphan rules. Instead, use LogValue wrapper directly.

// Alternative: Use serde for automatic conversion of Rust structs
#[allow(dead_code)]
pub struct SerdeValue<T: serde::Serialize>(pub T);

#[allow(dead_code)]
impl<T: serde::Serialize> OptionToJsValue for SerdeValue<T> {
    fn to_js_value(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.0).unwrap_or(JsValue::NULL)
    }
}

// Helper macro to make LogValue usage cleaner
/// Convert value to JsValue
#[macro_export]
macro_rules! js_val {
    ($val:expr) => {
        $crate::utils::LogValue($val)
    };
}

/// Extension of log_info! macro that supports JS object logging
#[macro_export]
macro_rules! log_info_js {
    // Standard log_info behavior
    ($($arg:tt)*) => {
        $crate::log_info!($($arg)*);
    };
    
    // JS object logging
    ($message:expr, object: { $($key:expr => $value:expr),* $(,)? }) => {
        {
            let obj = js_sys::Object::new();
            $(
                let val: wasm_bindgen::JsValue = match (&$value).into() {
                    Ok(v) => v,
                    Err(_) => $value.to_string().into(),
                };
                js_sys::Reflect::set(&obj, &$key.into(), &val).unwrap();
            )*
            web_sys::console::log_2(&$message.into(), &obj);
        }
    };
}

