// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use aide::OperationIo;
use axum::response::IntoResponse;
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;
use std::str::FromStr;
use std::sync::OnceLock;

/// Wraps a String and checks whether it is a valid SHA256 hash on construction and deserialization.
#[derive(Deserialize, Clone, Serialize, JsonSchema, Debug, OperationIo, Eq, PartialEq)]
#[serde(try_from = "String")]
pub struct Sha256Hash(pub String);

impl Sha256Hash {
    pub fn new(value: String) -> Result<Self, String> {
        // Use a static OnceLock to avoid recomputing the regex on every call.
        static HASH_REGEX: OnceLock<Regex> = OnceLock::new();
        let hash_regex = HASH_REGEX.get_or_init(|| Regex::new("[A-Fa-f0-9]{64}").unwrap());
        if hash_regex.is_match(&value) {
            return Ok(Self(value));
        }
        Err(format!("Failed to parse to Sha256: {}", value))
    }
}

impl AsRef<Path> for Sha256Hash {
    #[inline]
    fn as_ref(&self) -> &Path {
        let v = &self.0;
        Path::new(v)
    }
}

impl fmt::Display for Sha256Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for Sha256Hash {
    type Error = <Sha256Hash as FromStr>::Err;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl FromStr for Sha256Hash {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_string())
    }
}

impl IntoResponse for Sha256Hash {
    fn into_response(self) -> axum::response::Response {
        self.0.into_response()
    }
}
