// example of what first_or_else! cleans up
// let pm = fr_ident
//     .into_iter()
//     .next()
//     .unwrap()
//     .possible_matches
//     .into_iter()
//     .next()
//     .ok_or_else(|| {
//         FRError::Engine(
//             "duplicate_check: image processing returned no possible matches".to_string(),
//         )
//     })?;

/// Drills into the first element of a nested collection.
/// Returns the provided error if any part of the path is empty.
#[macro_export]
macro_rules! first_or_else {
    ($val:expr, $err:expr) => {
        $val.into_iter()
            .next()
            .ok_or_else(|| $err)?
    };

    ($val:expr, $($prop:ident).+, $err:expr) => {
        $val.into_iter()
            .next()
            $(.and_then(|i| i.$prop.into_iter().next()))+
            .ok_or_else(|| $err)?
    };
}

/// Shorthand for extracting a string from a JSON Value.
/// Useful for the logging logic we wrote earlier.
#[macro_export]
macro_rules! json_str {
    ($value:expr, $key:expr) => {
        $value.get($key).and_then(|v| v.as_str()).unwrap_or("")
    };
    // Version with a default fallback
    ($value:expr, $key:expr, $default:expr) => {
        $value.get($key).and_then(|v| v.as_str()).unwrap_or($default)
    };
}

/// A "ternary-like" macro for cleaner inline conditional strings.
/// Useful for your logging status messages.
#[macro_export]
macro_rules! either {
    ($cond:expr, $then:expr, $else:expr) => {
        if $cond {
            $then
        } else {
            $else
        }
    };
}
