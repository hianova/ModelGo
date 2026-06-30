use memmap2::Mmap;
use safetensors::SafeTensors;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

/// An f16 type represented as u16 for strict no_std compatibility.
#[allow(non_camel_case_types)]
pub type f16 = u16;

#[repr(C)]
#[derive(Debug, Clone, Copy, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[archive(check_bytes)]
#[archive_attr(repr(C))]
pub struct SerializedVec101Block {
    pub w_pos_bits: [u64; 4],
    pub w_neg_bits: [u64; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[archive(check_bytes)]
#[archive_attr(repr(C))]
pub struct SerializedVec101SuperBlock {
    pub scales: [f16; 8],
    pub offsets: [i16; 8],
    pub _padding: [u8; 32],
    pub blocks: [SerializedVec101Block; 8],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[archive(check_bytes)]
#[archive_attr(repr(C))]
pub struct SerializedBlockQ4_0 {
    pub d: f16,
    pub qs: [u8; 16],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[archive(check_bytes)]
pub enum SerializedQuantType {
    Bit1_58,
    Q4_0,
}

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[archive(check_bytes)]
pub enum SerializedLayerData {
    Bit1_58(std::vec::Vec<SerializedVec101SuperBlock>),
    Q4_0(std::vec::Vec<SerializedBlockQ4_0>),
}

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[archive(check_bytes)]
pub struct SerializedLayerWeights {
    pub name: String,
    pub data: SerializedLayerData,
}

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[archive(check_bytes)]
pub struct SerializedModelWeights {
    pub layers: std::vec::Vec<SerializedLayerWeights>,
}

pub struct ZeroCopyModelLoader {
    _mmap: Mmap,
    pub archived_weights: *const ArchivedSerializedModelWeights,
}

impl ZeroCopyModelLoader {
    pub fn new<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        // Use check_archived_root to safely validate the data
        let archived = rkyv::check_archived_root::<SerializedModelWeights>(&mmap).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to validate rkyv archive format: {:?}", e),
            )
        })?;

        let archived_ptr = archived as *const ArchivedSerializedModelWeights;

        Ok(Self {
            _mmap: mmap,
            archived_weights: archived_ptr,
        })
    }
}

unsafe impl Send for ZeroCopyModelLoader {}
unsafe impl Sync for ZeroCopyModelLoader {}

pub struct SafetensorsMmapLoader {
    _mmap: Mmap,
    pub tensors: HashMap<String, *const u8>,
}

unsafe impl Send for SafetensorsMmapLoader {}
unsafe impl Sync for SafetensorsMmapLoader {}

impl SafetensorsMmapLoader {
    pub fn new<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        let mut tensors = HashMap::new();
        // Since SafeTensors::deserialize expects bytes and doesn't own them,
        // we parse the header, then get raw pointers.
        let st = SafeTensors::deserialize(&mmap).expect("Failed to parse safetensors header");
        for (name, tensor) in st.tensors() {
            tensors.insert(name.clone(), tensor.data().as_ptr());
        }

        Ok(Self {
            _mmap: mmap,
            tensors,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_loader_fail() {
        assert!(ZeroCopyModelLoader::new("/does/not/exist.bin").is_err());
        assert!(SafetensorsMmapLoader::new("/does/not/exist.safetensors").is_err());
    }

    #[test]
    fn test_zero_copy_loader_success() {
        let weights = SerializedModelWeights { layers: vec![] };
        let bytes = rkyv::to_bytes::<_, 256>(&weights).unwrap();

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(&bytes).unwrap();

        let loader = ZeroCopyModelLoader::new(temp_file.path());
        assert!(loader.is_ok());
    }
}
