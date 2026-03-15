/// Drills into the first element of a nested collection.
/// Returns the provided error if any part of the path is empty.
#[macro_export]
macro_rules! first_or_else {
    ($val:expr, $err:expr) => {
        ($val)
            .into_iter()
            .next()
            .ok_or_else(|| $err)?
    };

    ($val:expr, $($prop:ident).+, $err:expr) => {
        ($val)
            .into_iter()
            .next()
            $(
                .and_then(|item| item.$prop.into_iter().next())
            )+
            .ok_or_else(|| $err)?
    };
}

/// A "ternary-like" macro for cleaner inline conditional expressions.
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
