use model_go::mmap_reader::*;
use std::path::Path;
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
