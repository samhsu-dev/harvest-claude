use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use color_eyre::eyre::{Result, WrapErr};

use crate::constants::JSONL_READ_CAP;

/// Tails a JSONL file, buffering partial lines across reads.
#[derive(Debug)]
pub struct JsonlReader {
    path: PathBuf,
    offset: u64,
    line_buffer: String,
}

impl JsonlReader {
    /// Open a JSONL file and seek to the end (tail mode).
    pub fn new(path: PathBuf) -> Result<Self> {
        let mut file = File::open(&path)
            .wrap_err_with(|| format!("failed to open JSONL file: {}", path.display()))?;
        let end = file
            .seek(SeekFrom::End(0))
            .wrap_err("failed to seek to end of JSONL file")?;
        Ok(Self {
            path,
            offset: end,
            line_buffer: String::new(),
        })
    }

    /// Open a JSONL file and read from the beginning.
    pub fn new_from_start(path: PathBuf) -> Result<Self> {
        // Verify the file exists and is readable.
        File::open(&path)
            .wrap_err_with(|| format!("failed to open JSONL file: {}", path.display()))?;
        Ok(Self {
            path,
            offset: 0,
            line_buffer: String::new(),
        })
    }

    /// Read new complete lines from the file, up to `JSONL_READ_CAP` bytes per call.
    ///
    /// Partial lines (without trailing `\n`) are kept in the internal buffer
    /// and prepended to the next read.
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
            self.line_buffer.clear();
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

        let chunk = String::from_utf8_lossy(&buf);

        // Prepend any leftover partial line from last read.
        let combined = if self.line_buffer.is_empty() {
            chunk.into_owned()
        } else {
            let mut s = std::mem::take(&mut self.line_buffer);
            s.push_str(&chunk);
            s
        };

        let mut lines = Vec::new();
        let mut last_newline = 0;

        for (i, ch) in combined.char_indices() {
            if ch == '\n' {
                let line = &combined[last_newline..i];
                if !line.is_empty() {
                    lines.push(line.to_owned());
                }
                last_newline = i + 1;
            }
        }

        // Anything after the last newline is a partial line — buffer it.
        if last_newline < combined.len() {
            self.line_buffer = combined[last_newline..].to_owned();
        }

        Ok(lines)
    }

    /// Current byte offset into the file.
    pub fn offset(&self) -> u64 {
        self.offset
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
}
