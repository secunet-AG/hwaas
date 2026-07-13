// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

//! This module contains a function for extracting the full chain of debug representations of an error's
//! sources. Note that this is a temporary solution. We should move this into a better place in the
//! workplace such that it may be reused in more crates.

use std::fmt::Write;

/// Recursively writes an error's source debug representation into
/// a line of a string.
pub(crate) fn error_debug_sources<E: std::error::Error>(error: &E) -> String {
    let mut debug_sources = String::new();
    let mut source = error.source();
    while let Some(e) = source {
        let _ = writeln!(&mut debug_sources, "{:?}", e);
        source = e.source();
    }
    debug_sources
}
