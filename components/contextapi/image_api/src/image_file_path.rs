//! # Utilities for working with file paths of BMR images

use std::path::{Component, Path, PathBuf};

/// A fully resolved image file path.
///
/// Note that the file this object resolves to is not guaranteed to exist. It may also describe an
/// image file path for an image that is yet to be written.
///
/// # Implementation details
///
/// This type is primarily necessary to allow efficient use of `async` in e.g. maintenance tasks.
/// When defining this functionality on [`ImageHandler`] directly, the requirement of `&self` as
/// function argument prevents calling the function in `'static` contexts (such as tokios
/// `JoinSet`).
#[derive(Debug, Clone, PartialEq)]
pub struct ImageFilePath(PathBuf);

/// invalid input path {path:?}: {reason}
#[derive(Debug, thiserror::Error, displaydoc::Display)]
pub struct Error {
    path: PathBuf,
    reason: &'static str,
}

impl ImageFilePath {
    /// Resolve an image file path.
    ///
    /// Takes an `image_store` which is the base directory of the BMR image store on disk and a
    /// `image`, which must point to a file inside the image store.
    ///
    /// If `image` is an absolute path, it must be a subpath of `image_store`. Otherwise, `image` is
    /// appended to the `image_store` as relative path and the resulting filepath is turned into an
    /// absolute path.
    ///
    /// # Security
    ///
    /// Any `../` path components in `image` are discarded during path resolution to prevent
    /// trivially escaping the actual image store directory. As a consequence, if `image_store` has
    /// `../` path components and `image` is an absolute path derived thereof using `.join()` for
    /// example, the path will be mangled and an error will be raised that in fact shouldn't be one.
    /// Hence, callers of this function should ensure that `image` has no such components if it's an
    /// absolute path.
    /// Furthermore, as this function may have to deal with nonexistent paths, too, it cannot
    /// prevent escaping the image store directory using e.g. symlinks. That is because resolving
    /// symlinks requires file system accesses which introduces vast amounts of complexity. Due to
    /// the internal nature of this API, adding this complexity doesn't seem justified at the
    /// moment.
    pub(crate) fn resolve<P1: AsRef<Path>, P2: AsRef<Path>>(
        image_store: P1,
        image: P2,
    ) -> Result<Self, Error> {
        let rel_image_path = image.as_ref();
        let image_store_path = image_store.as_ref();

        let sane_image_path = rel_image_path
            .components()
            .filter(|c| (c == &Component::RootDir) || matches!(c, Component::Normal(_)))
            .collect::<PathBuf>();
        let full_image_path = if sane_image_path.is_absolute() {
            sane_image_path.to_owned()
        } else {
            image_store_path.join(sane_image_path)
        };
        let abs_image_path = std::path::absolute(&full_image_path).map_err(|_| Error {
            path: full_image_path.to_owned(),
            reason: "failed to resolve absolute path for image location",
        })?;

        if !abs_image_path.starts_with(image_store_path) {
            return Err(Error {
                path: abs_image_path.to_owned(),
                reason: "resolved image path points outside of the image store directory",
            });
        }

        Ok(Self(abs_image_path))
    }
}

impl AsRef<Path> for ImageFilePath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl std::ops::Deref for ImageFilePath {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cannot_resolve_abs_path_outside_of_store() {
        let base_path = PathBuf::from("/a/b/c");
        let rel_abs_path = PathBuf::from("/d");

        let result = ImageFilePath::resolve(base_path, rel_abs_path);

        assert!(result.is_err());
    }

    #[test]
    fn can_resolve_abs_path_into_store() {
        let base_path = PathBuf::from("/a/b/c");
        let full_abs_path = PathBuf::from("/a/b/c/d");

        let result = ImageFilePath::resolve(base_path, full_abs_path);

        assert!(result.is_ok());
    }

    #[test]
    fn can_detect_abs_path_confusion() {
        let base_path = PathBuf::from("/a/b/c");
        let escaping_abs_path = PathBuf::from("/a/b/c/../d");

        let result = ImageFilePath::resolve(base_path, escaping_abs_path);

        let path = result.unwrap();
        // NOTE: The '..' is removed and 'd' is simply appended
        assert_eq!(path.0, PathBuf::from("/a/b/c/d"));
    }

    #[test]
    fn can_resolve_rel_path() {
        let base_path = PathBuf::from("/a/b/c");
        let valid_rel_path = PathBuf::from("d");

        let result = ImageFilePath::resolve(base_path, valid_rel_path);

        assert!(result.is_ok());
    }

    #[test]
    fn cannot_resolve_escaping_rel_path() {
        let base_path = PathBuf::from("/a/b/c");
        let escaping_rel_path = PathBuf::from("../d");

        let result = ImageFilePath::resolve(base_path, escaping_rel_path);

        let path = result.unwrap();
        // NOTE: The '..' is removed and 'd' is simply appended
        assert_eq!(path.0, PathBuf::from("/a/b/c/d"));
    }

    #[test]
    fn rejects_parent_dir_traversal() {
        let tmpdir = "/tmp/hwaas-tests";
        std::fs::create_dir_all(tmpdir).unwrap();
        let store = tempfile::tempdir_in(tmpdir).unwrap();
        std::fs::write(store.path().join("ok.png"), b"x").unwrap();

        assert!(ImageFilePath::resolve(store.path(), "ok.png").is_ok());
        let result = ImageFilePath::resolve(store.path(), "../../../etc/passwd").unwrap();
        assert_eq!(result.0, store.as_ref().join("etc/passwd"));
    }
}
