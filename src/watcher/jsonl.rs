use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use color_eyre::eyre::{Result, WrapErr};

use crate::constants::{JSONL_MAX_LINE_BYTES, JSONL_READ_CAP};

/// Tails a JSONL file, buffering partial lines across reads.
///
/// Bytes are buffered raw and decoded only once a full line is present, so a
/// multibyte character split across two reads is never corrupted.
#[derive(Debug)]
pub struct JsonlReader {
    path: PathBuf,
    offset: u64,
    /// Bytes after the last newline seen.
    pending: Vec<u8>,
    /// Drop bytes up to the next newline (partial first line, or oversized line).
    discarding: bool,
}

impl JsonlReader {
    /// Open a JSONL file and seek to the end (tail mode).
    pub fn new(path: PathBuf) -> Result<Self> {
        let end = Self::file_len(&path)?;
        Ok(Self::at(path, end, false))
    }

    /// Open a JSONL file at `tail` bytes before the end (or at the start when
    /// the file is shorter). A line cut by the start offset is discarded.
    pub fn new_from_tail(path: PathBuf, tail: u64) -> Result<Self> {
        let offset = Self::file_len(&path)?.saturating_sub(tail);
        Ok(Self::at(path, offset, offset > 0))
    }

    /// Open a JSONL file and read from the beginning.
    pub fn new_from_start(path: PathBuf) -> Result<Self> {
        Self::file_len(&path)?;
        Ok(Self::at(path, 0, false))
    }

    fn at(path: PathBuf, offset: u64, discarding: bool) -> Self {
        Self {
            path,
            offset,
            pending: Vec::new(),
            discarding,
        }
    }

    fn file_len(path: &PathBuf) -> Result<u64> {
        let mut file = File::open(path)
            .wrap_err_with(|| format!("failed to open JSONL file: {}", path.display()))?;
        file.seek(SeekFrom::End(0))
            .wrap_err("failed to seek to end of JSONL file")
    }

    /// Read new complete lines from the file, up to `JSONL_READ_CAP` bytes per call.
    ///
    /// Partial lines (without trailing `\n`) stay buffered for the next call.
    /// A line longer than `JSONL_MAX_LINE_BYTES` is skipped.
    pub fn read_new_lines(&mut self) -> Result<Vec<String>> {
        let mut file = File::open(&self.path)
            .wrap_err_with(|| format!("failed to open JSONL file: {}", self.path.display()))?;

        let file_len = file
            .metadata()
            .wrap_err("failed to read JSONL file metadata")?
            .len();

        // File was truncated — reset to beginning.
        if file_len < self.offset {
            self.offset = 0;
            self.pending.clear();
            self.discarding = false;
        }

        let available = file_len.saturating_sub(self.offset);
        if available == 0 {
            return Ok(Vec::new());
        }

        let to_read = available.min(JSONL_READ_CAP) as usize;
        let mut buf = vec![0u8; to_read];

        file.seek(SeekFrom::Start(self.offset))
            .wrap_err("failed to seek in JSONL file")?;
        file.read_exact(&mut buf)
            .wrap_err("failed to read JSONL file")?;

        self.offset += to_read as u64;
        Ok(self.split_lines(&buf))
    }

    fn split_lines(&mut self, chunk: &[u8]) -> Vec<String> {
        let mut lines = Vec::new();
        let mut rest = chunk;
        while let Some(pos) = rest.iter().position(|&b| b == b'\n') {
            let (line, tail) = rest.split_at(pos);
            rest = &tail[1..];
            if self.discarding {
                self.discarding = false;
            } else if self.pending.len() + line.len() <= JSONL_MAX_LINE_BYTES {
                self.pending.extend_from_slice(line);
                if !self.pending.is_empty() {
                    lines.push(String::from_utf8_lossy(&self.pending).into_owned());
                }
            }
            self.pending.clear();
        }
        if !self.discarding {
            self.pending.extend_from_slice(rest);
            if self.pending.len() > JSONL_MAX_LINE_BYTES {
                self.pending.clear();
                self.discarding = true;
            }
        }
        lines
    }

    /// Current byte offset into the file.
    pub fn offset(&self) -> u64 {
        self.offset
    }

    #[cfg(test)]
    fn pending_len(&self) -> usize {
        self.pending.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn reads_complete_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.jsonl");
        std::fs::write(&path, "{\"a\":1}\n{\"b\":2}\n").unwrap();

        let mut reader = JsonlReader::new_from_start(path).unwrap();
        let lines = reader.read_new_lines().unwrap();
        assert_eq!(lines, vec!["{\"a\":1}", "{\"b\":2}"]);
    }

    #[test]
    fn buffers_partial_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.jsonl");
        std::fs::write(&path, "{\"a\":1}\n{\"partial").unwrap();

        let mut reader = JsonlReader::new_from_start(path.clone()).unwrap();
        let lines = reader.read_new_lines().unwrap();
        assert_eq!(lines, vec!["{\"a\":1}"]);

        // Append the rest.
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        f.write_all(b"\":true}\n").unwrap();

        let lines = reader.read_new_lines().unwrap();
        assert_eq!(lines, vec!["{\"partial\":true}"]);
    }

    #[test]
    fn tail_mode_skips_existing_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.jsonl");
        std::fs::write(&path, "{\"old\":1}\n").unwrap();

        let mut reader = JsonlReader::new(path.clone()).unwrap();
        let lines = reader.read_new_lines().unwrap();
        assert!(lines.is_empty());

        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        f.write_all(b"{\"new\":2}\n").unwrap();

        let lines = reader.read_new_lines().unwrap();
        assert_eq!(lines, vec!["{\"new\":2}"]);
    }

    #[test]
    fn multibyte_character_split_by_read_cap_is_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.jsonl");
        // The read cap falls inside the 3-byte character.
        let text = format!("{}中文", "a".repeat(JSONL_READ_CAP as usize - 1));
        std::fs::write(&path, format!("{text}\n")).unwrap();

        let mut reader = JsonlReader::new_from_start(path).unwrap();
        let mut lines = reader.read_new_lines().unwrap();
        lines.extend(reader.read_new_lines().unwrap());
        assert_eq!(lines, vec![text]);
    }

    #[test]
    fn oversized_line_is_skipped_without_unbounded_buffering() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.jsonl");
        let mut content = "x".repeat(JSONL_MAX_LINE_BYTES + 1000);
        content.push_str("\n{\"ok\":1}\n");
        std::fs::write(&path, content).unwrap();

        let mut reader = JsonlReader::new_from_start(path).unwrap();
        let mut lines = Vec::new();
        for _ in 0..(JSONL_MAX_LINE_BYTES / JSONL_READ_CAP as usize + 4) {
            lines.extend(reader.read_new_lines().unwrap());
            assert!(reader.pending_len() <= JSONL_MAX_LINE_BYTES);
        }
        assert_eq!(lines, vec!["{\"ok\":1}"]);
        assert_eq!(reader.pending_len(), 0);
    }

    #[test]
    fn tail_start_discards_the_first_partial_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.jsonl");
        std::fs::write(&path, "{\"first\":1}\n{\"second\":2}\n").unwrap();

        let mut reader = JsonlReader::new_from_tail(path, 20).unwrap();
        assert_eq!(reader.read_new_lines().unwrap(), vec!["{\"second\":2}"]);
    }
}
