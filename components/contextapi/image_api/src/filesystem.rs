// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

use sha2::{Digest, Sha256};
use std::ffi::OsStr;
use std::io::Error;
use std::path::Path;
use std::time::SystemTime;
use tokio::fs::{File, metadata, read_dir, read_to_string};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReadDirStream;

use crate::image_handler::ImageMetadata;

/// List all files with the specified file extension of the given directory.
/// If `None` is provided as file extension, it will match all files that have no extension.
/// For more information refer to [`std::path::Path::extension`].
pub async fn list_files_of_directory<P>(
    dir: P,
    file_extension: Option<&OsStr>,
) -> Result<Vec<String>, Error>
where
    P: AsRef<Path>,
{
    let folder = dir.as_ref();
    let read_dir_stream = read_dir(folder).await.map(ReadDirStream::new)?;

    read_dir_stream
        .filter(|elem| {
            elem.as_ref().map_or(true, |b| {
                b.path().is_file() && b.path().extension() == file_extension
            })
        })
        .map(|entry| match entry {
            Ok(ent) => ent
                .file_name()
                .to_str()
                .map(|f| f.to_owned())
                .ok_or_else(|| Error::other("Error converting the OsStr to UTF-8")),
            Err(err) => Err(err),
        })
        // the first 'Result' that is 'Err' will be returned by this collect
        .collect::<Result<Vec<_>, _>>()
        .await
}

/// Write the given Stream to the specified filepath and return the hash of the stream content
pub async fn write_and_hash<S>(stream: S, file: File) -> Result<String, Error>
where
    S: AsyncRead,
{
    // Convert the stream into an `AsyncRead`.
    futures::pin_mut!(stream);

    let mut file_writer = BufWriter::new(file);

    // Copy the body into the file.
    let mut buf = vec![0; 8 * 1024];
    let mut hasher = Sha256::new();

    loop {
        match stream.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => {
                hasher.update(&buf[..n]);

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
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;
    use tempfile::tempdir;
    use tokio::fs::write;

    #[tokio::test]
    async fn test_list_images() -> Result<(), ()> {
        let dir =
            tempdir().map_err(|e| println!("Error occurred during tempdir creation: {}", e))?;
        let image_name = "image.iso";
        let image_content = "image content".to_string();
        let expected = Vec::from([image_name]);

        let _result = write(dir.path().join(image_name), image_content).await;
        let extension: OsString = "iso".into();
        let actual = list_files_of_directory(dir.path(), Some(extension).as_deref())
            .await
            .unwrap();
        assert_eq!(actual, expected);
        Ok(())
    }
}
