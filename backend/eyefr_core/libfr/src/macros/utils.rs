/// Drills into the first element of a nested collection.
/// Returns the provided error if any part of the path is empty.
#[macro_export]
macro_rules! first_or_else {
    ($val:expr_2021, $err:expr_2021) => {
        ($val)
            .into_iter()
            .next()
            .ok_or_else(|| $err)?
    };

    ($val:expr_2021, $($prop:ident).+, $err:expr_2021) => {
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
    ($cond:expr_2021, $then:expr_2021, $else:expr_2021) => {
        if $cond { $then } else { $else }
    };
}
