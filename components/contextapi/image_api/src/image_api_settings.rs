// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

//! # Settings for the ImageAPI

use bytesize::ByteSize;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Settings for the ImageAPI
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct ImageApiSettings {
    /// Maximum allowed file size as string, using [`ByteSize`] format, e.g 128 Mb, 5 GB, etc.
    #[schemars(with = "String")]
    pub max_file_size: ByteSize,
    /// Store path underneath which uploaded images are located.
    pub store: PathBuf,
}

impl Default for ImageApiSettings {
    fn default() -> Self {
        Self {
            max_file_size: ByteSize::mib(128),
            store: "/tmp/image_api_store".parse().unwrap(),
        }
    }
}
