// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Write;

/// Iterates through the error sources and appends their
/// debug representations to a string which is returned upon
/// completion.
pub fn source_chain<E: std::error::Error>(e: &E) -> String {
    let mut result = String::new();
    let mut source = e.source();
    while let Some(e) = source {
        let _ = writeln!(&mut result, "{:?}", e);
        source = e.source();
    }
    result
}

/// Produces a closure that takes an error as input and logs its debug, display and source chain together with the string literal
/// passed to the macro. The generated closure returns the unit type `()` hence the closure can be passed directly to
/// [`Result::inspect_err`].
#[macro_export]
macro_rules! log_err {
    ($message:literal) => {
        |e| tracing::error!(error.dbg = ?e, error.msg = %e, error.source_chain = error_utils::source_chain(e), $message)
    };
}

/// Produces a closure that takes an error as input and logts its debug, display and source chain together with the string literal
/// provided to the macro. The generated closure returns the second argument passed to the macro hence the closure can be passed
/// directly to [`Result::map_err`].
#[macro_export]
macro_rules! log_then_replace_err {
    ($message:literal, $error:tt) => {
        |e| {
            tracing::error!(error.dbg = ?e, error.msg = %e, error.source_chain = error_utils::source_chain(&e), $message);
            $error
            }
        };
}
