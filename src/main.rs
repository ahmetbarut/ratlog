//! Log viewer with live text filtering — TUI application.
//!
//! Usage: `cargo run` (sample logs) or `cargo run -- <log-file>`
//! Live mode: press L or F to toggle (only when loaded from a file).

mod app;
mod cli;
mod constants;
mod logs;
mod settings;
mod theme;
mod util;

use std::env;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let args: Vec<String> = env::args().collect();
    let file_arg: Option<PathBuf> = cli::parse_args(&args);

    color_eyre::install()?;
    let terminal = ratatui::init();
    let (logs, file_path, file_offset, file_line_start) = logs::load_logs(file_arg)?;
    let result = app::App::new(logs, file_path, file_offset, file_line_start)
        .run(terminal)
        .await;
    ratatui::restore();
    result
}

#[cfg(test)]
mod tests {
    use crate::constants::MAX_LINES;
    use crate::logs::{apply_filter, parse_log_content, sample_logs};
    use crate::settings::SavedSettings;
    use crate::util::{centered_rect, format_bytes};
    use ratatui::layout::Rect;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1 KiB");
        assert_eq!(format_bytes(1536), "1 KiB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MiB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GiB");
        assert_eq!(format_bytes(1536 * 1024 * 1024), "1.5 GiB");
    }

    #[test]
    fn test_parse_log_content_empty() {
        let (lines, offset, start) = parse_log_content("");
        assert!(lines.is_empty());
        assert_eq!(offset, 0);
        assert_eq!(start, 1);
    }

    #[test]
    fn test_parse_log_content_one_line() {
        let (lines, offset, start) = parse_log_content("hello\n");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "hello");
        assert_eq!(offset, 0);
        assert_eq!(start, 1);
    }

    #[test]
    fn test_parse_log_content_last_max_lines() {
        let n = MAX_LINES + 50;
        let content = (0..n)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let (lines, _offset, start) = parse_log_content(&content);
        assert_eq!(lines.len(), MAX_LINES);
        assert_eq!(start, 51);
        assert_eq!(lines[0], "line 50");
        assert_eq!(lines[MAX_LINES - 1], format!("line {}", n - 1));
    }

    #[test]
    fn test_apply_filter_empty_query_returns_all() {
        let lines = vec!["a".into(), "b".into(), "c".into()];
        let out = apply_filter(&lines, "", 10);
        assert_eq!(out.len(), 3);
        assert_eq!(out[0], (0, "a".to_string()));
        assert_eq!(out[1], (1, "b".to_string()));
        assert_eq!(out[2], (2, "c".to_string()));
    }

    #[test]
    fn test_apply_filter_matching_case_insensitive() {
        let lines = vec!["INFO foo".into(), "ERROR bar".into(), "info baz".into()];
        let out = apply_filter(&lines, "info", 10);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], (0, "INFO foo".to_string()));
        assert_eq!(out[1], (2, "info baz".to_string()));
    }

    #[test]
    fn test_apply_filter_cap_max_lines() {
        let lines: Vec<String> = (0..20).map(|i| format!("x {}", i)).collect();
        let out = apply_filter(&lines, "x", 5);
        assert_eq!(out.len(), 5);
        assert_eq!(out[0].1, "x 15");
        assert_eq!(out[4].1, "x 19");
    }

    #[test]
    fn test_sample_logs_non_empty() {
        let logs = sample_logs();
        assert!(!logs.is_empty());
        assert!(logs.len() <= MAX_LINES);
        assert!(
            logs[0].contains("INFO")
                || logs[0].contains("DEBUG")
                || logs[0].contains("WARN")
                || logs[0].contains("ERROR")
        );
    }

    #[test]
    fn test_centered_rect() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 20,
        };
        let r = centered_rect(area, 50, 50);
        assert_eq!(r.width, 50);
        assert_eq!(r.height, 10);
        assert_eq!(r.x, 25);
        assert_eq!(r.y, 5);
    }

    #[test]
    fn test_saved_settings_roundtrip() {
        let saved = SavedSettings {
            accent: "Cyan".to_string(),
            text_color: "White".to_string(),
            text_style: "Normal".to_string(),
            border_color: "Gray".to_string(),
            status_color: "Gray".to_string(),
        };
        let s = serde_json::to_string_pretty(&saved).unwrap();
        let loaded: SavedSettings = serde_json::from_str(&s).unwrap();
        assert_eq!(loaded.accent, saved.accent);
        assert_eq!(loaded.text_color, saved.text_color);
        assert_eq!(loaded.text_style, saved.text_style);
        assert_eq!(loaded.border_color, saved.border_color);
        assert_eq!(loaded.status_color, saved.status_color);
    }
}
