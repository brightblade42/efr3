/// Shorthand for extracting a string from a JSON Value.
#[macro_export]
macro_rules! json_str {
    ($value:expr_2021, $key:expr_2021) => {
        $value.get($key).and_then(|v| v.as_str()).unwrap_or("")
    };

    ($value:expr_2021, $key:expr_2021, $default:expr_2021) => {
        $value.get($key).and_then(|v| v.as_str()).unwrap_or($default)
    };
}
