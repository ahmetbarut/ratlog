//! Log loading: file tail, streaming, filter, sample logs.

use std::collections::VecDeque;
use std::fs;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::PathBuf;

use crate::constants::{MAX_LINE_LEN, MAX_LINES, TAIL_READ_SIZE};

/// Given file content, returns (last MAX_LINES lines, byte offset, 1-based file line number of first line).
#[allow(dead_code)]
pub fn parse_log_content(content: &str) -> (Vec<String>, u64, usize) {
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();
    let skip = total.saturating_sub(MAX_LINES);
    let file_line_start = skip + 1;
    let kept: Vec<String> = lines[skip..].iter().map(|s| s.to_string()).collect();
    let file_offset = content
        .lines()
        .take(skip)
        .map(|l| l.len() + 1)
        .sum::<usize>() as u64;
    (kept, file_offset, file_line_start)
}

/// Filter lines by query (case-insensitive substring); returns at most max_lines (last N matches).
pub fn apply_filter(
    lines: &[String],
    filter: &str,
    max_lines: usize,
) -> Vec<(usize, String)> {
    let q = filter.trim().to_lowercase();
    let with_idx: Vec<(usize, String)> = if q.is_empty() {
        lines
            .iter()
            .enumerate()
            .map(|(i, s)| (i, s.clone()))
            .collect()
    } else {
        lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.to_lowercase().contains(&q))
            .map(|(i, s)| (i, s.clone()))
            .collect()
    };
    if with_idx.len() <= max_lines {
        with_idx
    } else {
        with_idx[with_idx.len() - max_lines..].to_vec()
    }
}

fn read_line_bounded<R: BufRead>(r: &mut R) -> io::Result<Option<String>> {
    let mut buf = Vec::with_capacity(4096.min(MAX_LINE_LEN));
    let mut total = 0usize;
    loop {
        let (consume_amt, done, skip_until_newline) = {
            let chunk = r.fill_buf()?;
            if chunk.is_empty() {
                break;
            }
            let mut found = None;
            for (i, &b) in chunk.iter().enumerate() {
                if b == b'\n' {
                    found = Some(i);
                    break;
                }
                if total + i >= MAX_LINE_LEN {
                    break;
                }
            }
            match found {
                Some(i) => {
                    buf.extend_from_slice(&chunk[..=i]);
                    (i + 1, true, false)
                }
                None if total + chunk.len() >= MAX_LINE_LEN => {
                    let take = (MAX_LINE_LEN - total).min(chunk.len());
                    buf.extend_from_slice(&chunk[..take]);
                    (take, true, true)
                }
                None => {
                    buf.extend_from_slice(chunk);
                    (chunk.len(), false, false)
                }
            }
        };
        r.consume(consume_amt);
        if done {
            if skip_until_newline {
                loop {
                    let (to_consume, found_newline) = {
                        let c = r.fill_buf()?;
                        if c.is_empty() {
                            (0, true)
                        } else {
                            match c.iter().position(|&b| b == b'\n') {
                                Some(pos) => (pos + 1, true),
                                None => (c.len(), false),
                            }
                        }
                    };
                    r.consume(to_consume);
                    if found_newline {
                        break;
                    }
                }
            }
            break;
        }
        total += consume_amt;
    }
    if buf.is_empty() {
        return Ok(None);
    }
    let s = String::from_utf8_lossy(&buf)
        .trim_end_matches('\n')
        .to_string();
    Ok(Some(s))
}

fn offset_after_n_newlines(path: &PathBuf, n: usize) -> io::Result<u64> {
    if n == 0 {
        return Ok(0);
    }
    let f = File::open(path)?;
    let mut r = BufReader::new(f);
    let mut offset: u64 = 0;
    let mut newlines_seen: usize = 0;
    let mut chunk = [0u8; 65536];
    loop {
        let nread = r.read(&mut chunk)?;
        if nread == 0 {
            break;
        }
        for (i, &b) in chunk[..nread].iter().enumerate() {
            if b == b'\n' {
                newlines_seen += 1;
                if newlines_seen == n {
                    return Ok(offset + i as u64 + 1);
                }
            }
        }
        offset += nread as u64;
    }
    Ok(offset)
}

fn parse_tail_lines(mut content: &[u8]) -> Vec<String> {
    if let Some(first_nl) = content.iter().position(|&b| b == b'\n') {
        content = &content[first_nl + 1..];
    }
    let mut lines = Vec::new();
    for line in content.split(|&b| b == b'\n') {
        let s = String::from_utf8_lossy(line).to_string();
        let truncated = if s.len() > MAX_LINE_LEN {
            format!("{}...", &s[..MAX_LINE_LEN])
        } else {
            s
        };
        if !truncated.is_empty() {
            lines.push(truncated);
        }
    }
    if lines.len() > MAX_LINES {
        lines[lines.len() - MAX_LINES..].to_vec()
    } else {
        lines
    }
}

/// Load last MAX_LINES from file. For large files, only reads the last TAIL_READ_SIZE bytes.
pub fn load_logs(
    file_arg: Option<PathBuf>,
) -> io::Result<(Vec<String>, Option<PathBuf>, u64, usize)> {
    if let Some(path) = file_arg {
        if !path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Log file not found: {}", path.display()),
            ));
        }
        let meta = fs::metadata(&path)?;
        let file_size = meta.len();

        if file_size > TAIL_READ_SIZE {
            let mut file = File::open(&path)?;
            let start = file_size.saturating_sub(TAIL_READ_SIZE);
            file.seek(SeekFrom::Start(start))?;
            let cap = TAIL_READ_SIZE.min(usize::MAX as u64) as usize;
            let mut buf = Vec::with_capacity(cap);
            let mut limited = (&mut file).take(TAIL_READ_SIZE);
            let _ = limited.read_to_end(&mut buf);
            buf.truncate(buf.len().min(cap));
            let kept = parse_tail_lines(&buf);
            let file_offset = file_size;
            let file_line_start = 1;
            return Ok((kept, Some(path), file_offset, file_line_start));
        }

        let file = File::open(&path)?;
        let mut reader = BufReader::new(file);
        let mut deque: VecDeque<String> = VecDeque::with_capacity(MAX_LINES + 1);
        let mut total_lines: usize = 0;
        while let Some(line) = read_line_bounded(&mut reader)? {
            total_lines += 1;
            deque.push_back(line);
            if deque.len() > MAX_LINES {
                deque.pop_front();
            }
        }
        let kept: Vec<String> = deque.into_iter().collect();
        let file_line_start = total_lines.saturating_sub(kept.len()) + 1;

        let file_offset = if file_line_start <= 1 {
            0
        } else {
            offset_after_n_newlines(&path, file_line_start - 1)?
        };

        Ok((kept, Some(path), file_offset, file_line_start))
    } else {
        Ok((sample_logs(), None, 0, 1))
    }
}

pub fn sample_logs() -> Vec<String> {
    vec![
        "2025-02-15T10:00:00Z INFO  Server started on 0.0.0.0:8080".into(),
        "2025-02-15T10:00:01Z DEBUG Connecting to database...".into(),
        "2025-02-15T10:00:02Z INFO  Database connection pool ready".into(),
        "2025-02-15T10:00:05Z WARN  High memory usage: 85%".into(),
        "2025-02-15T10:00:10Z ERROR Failed to connect to cache: connection refused".into(),
        "2025-02-15T10:00:11Z INFO  Retrying cache connection (attempt 2)".into(),
        "2025-02-15T10:00:15Z ERROR Timeout waiting for response from auth service".into(),
        "2025-02-15T10:00:20Z DEBUG Request GET /api/health completed in 2ms".into(),
        "2025-02-15T10:00:21Z INFO  Request GET /api/users completed in 45ms".into(),
        "2025-02-15T10:00:25Z WARN  Rate limit approaching for client 192.168.1.1".into(),
        "2025-02-15T10:00:30Z ERROR Database deadlock detected, retrying transaction".into(),
        "2025-02-15T10:00:35Z INFO  Backup job started".into(),
        "2025-02-15T10:00:40Z DEBUG Cache hit ratio: 0.92".into(),
        "2025-02-15T10:00:45Z WARN  Disk space below 20% on /var/log".into(),
        "2025-02-15T10:00:50Z INFO  Backup job completed successfully".into(),
    ]
}
