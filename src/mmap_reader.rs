use std::fs::File;
use std::path::Path;
use anyhow::{Context, Result};
use memmap2::Mmap;

/// A Zero-Copy reader that mmaps a file into memory.
pub struct ZeroCopyMmapReader {
    _file: File,
    mmap: Mmap,
}

impl ZeroCopyMmapReader {
    /// Maps the given file path into memory entirely.
    /// This allows direct, zero-copy pointer access to file contents.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref())
            .with_context(|| format!("Failed to open file for mmap: {}", path.as_ref().display()))?;
        
        // Safety: the file is opened read-only and mapped read-only.
        // It relies on the OS guaranteeing that the mapped memory won't cause UB
        // unless another process mutates the file concurrently.
        let mmap = unsafe {
            Mmap::map(&file)
                .with_context(|| "Failed to mmap the file")?
        };

        Ok(Self {
            _file: file,
            mmap,
        })
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
    fn test_valid_mmap() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"Hello ModelGo Mmap").unwrap();
        let path = temp_file.path().to_path_buf();
        
        let reader = ZeroCopyMmapReader::new(&path).unwrap();
        assert_eq!(reader.as_bytes(), b"Hello ModelGo Mmap");
    }

    #[test]
    fn test_invalid_mmap_path() {
        let path = Path::new("/path/that/does/not/exist.vec101");
        assert!(ZeroCopyMmapReader::new(path).is_err());
    }

    #[test]
    fn test_empty_file_mmap() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();
        
        // Memmap2 might fail or map an empty slice on empty files depending on platform.
        // We just ensure it doesn't panic and gracefully handles it or errors out cleanly.
        let result = ZeroCopyMmapReader::new(&path);
        if let Ok(reader) = result {
            assert_eq!(reader.as_bytes().len(), 0);
        }
    }
}
