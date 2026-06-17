// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use crate::sha256hash::Sha256Hash;
use sha2::{Digest, Sha256};
use std::io::Error;
use std::path::Path;
use std::time::SystemTime;
use tokio::fs::{File, metadata, read_dir, read_to_string};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReadDirStream;

/// Write the given Stream to the specified filepath and return the hash of the stream content
pub async fn write_and_hash<S>(stream: S, file: File) -> Result<(Sha256Hash, usize), Error>
where
    S: AsyncRead,
{
    // Convert the stream into an `AsyncRead`.
    futures::pin_mut!(stream);

    let mut file_writer = BufWriter::new(file);

    // Copy the body into the file.
    let mut buf = vec![0; 8 * 1024];
    let mut hasher = Sha256::new();
    let mut size = 0usize;

    loop {
        match stream.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => {
                hasher.update(&buf[..n]);
                size = size.strict_add(n);

                // Copy the data to the file
                file_writer.write_all(&buf[..n]).await?
            }
            Err(e) => return Err(e),
        }
    }

    file_writer.flush().await?;

    // returns a string with a lower case hash
    // `{:X}` would result in upper case.
    // because comparing it to a URL parameter it should be lower-case (REST design rule)
    Sha256Hash::new(format!("{:x}", hasher.finalize()))
        .map_err(Error::other)
        .map(|hash| (hash, size))
}
