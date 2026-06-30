use anyhow::{Context, Result};
use memmap2::Mmap;
use std::fs::File;
use std::path::Path;

/// A Zero-Copy reader that mmaps a file into memory.
pub struct ZeroCopyMmapReader {
    _file: File,
    mmap: Mmap,
}

impl ZeroCopyMmapReader {
    /// Maps the given file path into memory entirely.
    /// This allows direct, zero-copy pointer access to file contents.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref()).with_context(|| {
            format!("Failed to open file for mmap: {}", path.as_ref().display())
        })?;

        // Safety: the file is opened read-only and mapped read-only.
        // It relies on the OS guaranteeing that the mapped memory won't cause UB
        // unless another process mutates the file concurrently.
        let mmap = unsafe { Mmap::map(&file).with_context(|| "Failed to mmap the file")? };

        Ok(Self { _file: file, mmap })
    }

    /// Retrieves a zero-copy slice to the file's bytes.
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        &self.mmap
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_mmap_reader() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"hello world").unwrap();

        let reader = ZeroCopyMmapReader::new(temp_file.path()).unwrap();
        assert_eq!(reader.as_bytes(), b"hello world");

        let bad_reader = ZeroCopyMmapReader::new("/does/not/exist");
        assert!(bad_reader.is_err());
    }
}
