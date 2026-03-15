/// Shorthand for extracting a string from a JSON Value.
#[macro_export]
macro_rules! json_str {
    ($value:expr, $key:expr) => {
        $value.get($key).and_then(|v| v.as_str()).unwrap_or("")
    };

    ($value:expr, $key:expr, $default:expr) => {
        $value.get($key).and_then(|v| v.as_str()).unwrap_or($default)
    };
}
