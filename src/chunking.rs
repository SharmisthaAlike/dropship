use std::path::Path;

use anyhow::{bail, Result};
use sha2::{Digest, Sha256};
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub struct FileChunk {
    pub id: u64,
    pub data: Vec<u8>,
    pub hash: String,
}

pub struct FileChunker {
    file: File,
    file_size: u64,
    chunk_size: usize,
    next_id: u64,
    file_hasher: Sha256,
    finished: bool,
}

impl FileChunker {
    pub async fn open(path: impl AsRef<Path>, chunk_size: usize) -> Result<Self> {
        if chunk_size == 0 {
            bail!("chunk size must be greater than zero");
        }

        let file_size = tokio::fs::metadata(path.as_ref()).await?.len();
        let file = File::open(path.as_ref()).await?;

        Ok(Self {
            file,
            file_size,
            chunk_size,
            next_id: 0,
            file_hasher: Sha256::new(),
            finished: false,
        })
    }

    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    pub fn total_chunks(&self) -> u64 {
        self.file_size.div_ceil(self.chunk_size as u64)
    }

    pub fn file_hash(&self) -> Option<String> {
        self.finished
            .then(|| hex_digest(self.file_hasher.clone().finalize()))
    }

    pub async fn next_chunk(&mut self) -> Result<Option<FileChunk>> {
        if self.finished {
            return Ok(None);
        }

        let mut data = vec![0; self.chunk_size];
        let bytes_read = self.file.read(&mut data).await?;
        if bytes_read == 0 {
            self.finished = true;
            return Ok(None);
        }

        data.truncate(bytes_read);
        self.file_hasher.update(&data);

        let hash = hex_digest(Sha256::digest(&data));
        let chunk = FileChunk {
            id: self.next_id,
            data,
            hash,
        };
        self.next_id += 1;
        Ok(Some(chunk))
    }
}

fn hex_digest(digest: impl AsRef<[u8]>) -> String {
    digest
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static FIXTURE_ID: AtomicUsize = AtomicUsize::new(0);

    async fn write_fixture(data: &[u8]) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let id = FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("dropship-chunk-test-{suffix}-{id}"));
        tokio::fs::write(&path, data).await.unwrap();
        path
    }

    #[tokio::test]
    async fn streams_chunks_and_hashes_partial_final_chunk() {
        let path = write_fixture(b"abcdefghij").await;
        let mut chunker = FileChunker::open(&path, 4).await.unwrap();

        assert_eq!(chunker.file_size(), 10);
        assert_eq!(chunker.total_chunks(), 3);
        assert_eq!(chunker.next_chunk().await.unwrap().unwrap().data, b"abcd");
        assert_eq!(chunker.next_chunk().await.unwrap().unwrap().data, b"efgh");
        assert_eq!(chunker.next_chunk().await.unwrap().unwrap().data, b"ij");
        assert!(chunker.next_chunk().await.unwrap().is_none());
        assert_eq!(
            chunker.file_hash().unwrap(),
            hex_digest(Sha256::digest(b"abcdefghij"))
        );

        tokio::fs::remove_file(path).await.unwrap();
    }

    #[tokio::test]
    async fn hashes_empty_files_after_streaming_to_end() {
        let path = write_fixture(b"").await;
        let mut chunker = FileChunker::open(&path, DEFAULT_CHUNK_SIZE).await.unwrap();

        assert_eq!(chunker.total_chunks(), 0);
        assert!(chunker.next_chunk().await.unwrap().is_none());
        assert_eq!(chunker.file_hash().unwrap(), hex_digest(Sha256::digest([])));

        tokio::fs::remove_file(path).await.unwrap();
    }

    #[tokio::test]
    async fn rejects_zero_chunk_size() {
        let path = write_fixture(b"data").await;
        let error = match FileChunker::open(&path, 0).await {
            Ok(_) => panic!("zero chunk size should be rejected"),
            Err(error) => error,
        };

        assert!(error
            .to_string()
            .contains("chunk size must be greater than zero"));
        tokio::fs::remove_file(path).await.unwrap();
    }
}
