//! Log viewer with live text filtering — TUI application.
//!
//! Usage: `cargo run` (sample logs) or `cargo run -- <log-file>`
//! Live mode: press L or F to toggle (only when loaded from a file).
//!
//! # Memory (RAM)
//! - At most `MAX_LINES` (150) lines are kept; only the last 150 lines are loaded from file and in live mode.
//! - When filtering, the last 150 matching lines are shown.
pub const MAX_LINES: usize = 150;

use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures::{FutureExt, StreamExt};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Clear, HighlightSpacing, List, ListItem, ListState, Paragraph, Wrap},
};
use std::collections::VecDeque;
use std::env;
use std::fs;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

/// Path to the saved settings file (e.g. ~/.config/ratlog/settings.json).
fn settings_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("ratlog").join("settings.json"))
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SavedSettings {
    accent: String,
    text_color: String,
    text_style: String,
    border_color: String,
    status_color: String,
}

fn load_settings() -> (AccentColor, TextColor, TextStyle, BorderColor, StatusColor) {
    let path = match settings_path() {
        Some(p) => p,
        None => {
            return (
                AccentColor::default(),
                TextColor::default(),
                TextStyle::default(),
                BorderColor::default(),
                StatusColor::default(),
            );
        }
    };
    let s = match fs::read_to_string(&path) {
        Ok(x) => x,
        Err(_) => {
            return (
                AccentColor::default(),
                TextColor::default(),
                TextStyle::default(),
                BorderColor::default(),
                StatusColor::default(),
            );
        }
    };
    let saved: SavedSettings = match serde_json::from_str(&s) {
        Ok(x) => x,
        Err(_) => {
            return (
                AccentColor::default(),
                TextColor::default(),
                TextStyle::default(),
                BorderColor::default(),
                StatusColor::default(),
            );
        }
    };
    let parse_accent = |v: &str| {
        AccentColor::all()
            .iter()
            .find(|c| c.name().eq_ignore_ascii_case(v))
            .copied()
            .unwrap_or_default()
    };
    let parse_text_color = |v: &str| {
        TextColor::all()
            .iter()
            .find(|c| c.name().eq_ignore_ascii_case(v))
            .copied()
            .unwrap_or_default()
    };
    let parse_text_style = |v: &str| {
        TextStyle::all()
            .iter()
            .find(|c| c.name().eq_ignore_ascii_case(v))
            .copied()
            .unwrap_or_default()
    };
    let parse_border = |v: &str| {
        BorderColor::all()
            .iter()
            .find(|c| c.name().eq_ignore_ascii_case(v))
            .copied()
            .unwrap_or_default()
    };
    let parse_status = |v: &str| {
        StatusColor::all()
            .iter()
            .find(|c| c.name().eq_ignore_ascii_case(v))
            .copied()
            .unwrap_or_default()
    };
    (
        parse_accent(&saved.accent),
        parse_text_color(&saved.text_color),
        parse_text_style(&saved.text_style),
        parse_border(&saved.border_color),
        parse_status(&saved.status_color),
    )
}

fn save_settings(
    accent: AccentColor,
    text_color: TextColor,
    text_style: TextStyle,
    border_color: BorderColor,
    status_color: StatusColor,
) {
    let path = match settings_path() {
        Some(p) => p,
        None => return,
    };
    let saved = SavedSettings {
        accent: accent.name().to_string(),
        text_color: text_color.name().to_string(),
        text_style: text_style.name().to_string(),
        border_color: border_color.name().to_string(),
        status_color: status_color.name().to_string(),
    };
    let s = match serde_json::to_string_pretty(&saved) {
        Ok(x) => x,
        Err(_) => return,
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .and_then(|mut f| f.write_all(s.as_bytes()));
}

/// Returns the current process memory usage (RSS) in a readable format (e.g. "12.5 MiB").
fn current_process_memory() -> String {
    use sysinfo::System;
    let mut sys = System::new_all();
    sys.refresh_all();
    if let Some(me) = sys.process(sysinfo::Pid::from(std::process::id() as usize)) {
        format_bytes(me.memory())
    } else {
        "—".to_string()
    }
}

/// Returns a rect centered in `area` with given width/height percent (0-100).
fn centered_rect(area: Rect, width_pct: u16, height_pct: u16) -> Rect {
    let w = area.width * width_pct / 100;
    let h = area.height * height_pct / 100;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect {
        x,
        y,
        width: w,
        height: h,
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.1} GiB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MiB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KiB", bytes / KB)
    } else {
        format!("{} B", bytes)
    }
}

/// Given file content, returns (last MAX_LINES lines, byte offset of first kept line, 1-based file line number of first line).
fn parse_log_content(content: &str) -> (Vec<String>, u64, usize) {
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

/// Filter lines by query (case-insensitive substring); returns at most max_lines (last N matches). Returns (index_in_original, line).
fn apply_filter(lines: &[String], filter: &str, max_lines: usize) -> Vec<(usize, String)> {
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

/// Load last MAX_LINES from file by streaming (avoids loading huge files into memory).
/// Returns (lines, file_path, file_offset, file_line_start) where file_line_start is 1-based.
fn load_logs() -> io::Result<(Vec<String>, Option<PathBuf>, u64, usize)> {
    let args: Vec<String> = env::args().collect();
    if let Some(path) = args.get(1) {
        let path = PathBuf::from(path);
        if !path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Log file not found: {}", path.display()),
            ));
        }
        let file = File::open(&path)?;
        let reader = BufReader::new(file);
        let mut deque: VecDeque<String> = VecDeque::with_capacity(MAX_LINES + 1);
        let mut total_lines: usize = 0;
        for line in reader.lines() {
            let line = line?;
            total_lines += 1;
            deque.push_back(line);
            if deque.len() > MAX_LINES {
                deque.pop_front();
            }
        }
        let kept: Vec<String> = deque.into_iter().collect();
        let file_line_start = total_lines.saturating_sub(kept.len()) + 1;

        // Byte offset of first kept line (for live tail)
        let file_offset = if file_line_start <= 1 {
            0
        } else {
            let f = File::open(&path)?;
            let r = BufReader::new(f);
            let mut offset: u64 = 0;
            for line in r.lines().take(file_line_start - 1) {
                let s = line?;
                offset += s.len() as u64 + 1;
            }
            offset
        };

        Ok((kept, Some(path), file_offset, file_line_start))
    } else {
        Ok((sample_logs(), None, 0, 1))
    }
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let (logs, file_path, file_offset, file_line_start) = load_logs()?;
    let result = App::new(logs, file_path, file_offset, file_line_start)
        .run(terminal)
        .await;
    ratatui::restore();
    result
}

/// Sample log lines (used when no file argument is given).
fn sample_logs() -> Vec<String> {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Filter,
    LogList,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AccentColor {
    Cyan,
    Green,
    Yellow,
    Magenta,
    Blue,
}

impl Default for AccentColor {
    fn default() -> Self {
        AccentColor::Cyan
    }
}

impl AccentColor {
    fn to_ratatui(self) -> Color {
        match self {
            AccentColor::Cyan => Color::Cyan,
            AccentColor::Green => Color::Green,
            AccentColor::Yellow => Color::Yellow,
            AccentColor::Magenta => Color::Magenta,
            AccentColor::Blue => Color::Blue,
        }
    }
    fn name(self) -> &'static str {
        match self {
            AccentColor::Cyan => "Cyan",
            AccentColor::Green => "Green",
            AccentColor::Yellow => "Yellow",
            AccentColor::Magenta => "Magenta",
            AccentColor::Blue => "Blue",
        }
    }
    fn all() -> &'static [AccentColor] {
        &[
            AccentColor::Cyan,
            AccentColor::Green,
            AccentColor::Yellow,
            AccentColor::Magenta,
            AccentColor::Blue,
        ]
    }
}

/// Text colour for log lines (and other UI text where applicable).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextColor {
    White,
    Gray,
    Cyan,
    Green,
    Yellow,
}

impl Default for TextColor {
    fn default() -> Self {
        TextColor::White
    }
}

impl TextColor {
    fn to_ratatui(self) -> Color {
        match self {
            TextColor::White => Color::White,
            TextColor::Gray => Color::Gray,
            TextColor::Cyan => Color::Cyan,
            TextColor::Green => Color::Green,
            TextColor::Yellow => Color::Yellow,
        }
    }
    fn name(self) -> &'static str {
        match self {
            TextColor::White => "White",
            TextColor::Gray => "Gray",
            TextColor::Cyan => "Cyan",
            TextColor::Green => "Green",
            TextColor::Yellow => "Yellow",
        }
    }
    fn all() -> &'static [TextColor] {
        &[
            TextColor::White,
            TextColor::Gray,
            TextColor::Cyan,
            TextColor::Green,
            TextColor::Yellow,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextStyle {
    Normal,
    Bold,
    Dim,
}

impl Default for TextStyle {
    fn default() -> Self {
        TextStyle::Normal
    }
}

impl TextStyle {
    fn name(self) -> &'static str {
        match self {
            TextStyle::Normal => "Normal",
            TextStyle::Bold => "Bold",
            TextStyle::Dim => "Dim",
        }
    }
    fn modifier(self) -> Modifier {
        match self {
            TextStyle::Normal => Modifier::empty(),
            TextStyle::Bold => Modifier::BOLD,
            TextStyle::Dim => Modifier::DIM,
        }
    }
    fn all() -> &'static [TextStyle] {
        &[TextStyle::Normal, TextStyle::Bold, TextStyle::Dim]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BorderColor {
    White,
    Gray,
    DarkGray,
}

impl Default for BorderColor {
    fn default() -> Self {
        BorderColor::Gray
    }
}

impl BorderColor {
    fn to_ratatui(self) -> Color {
        match self {
            BorderColor::White => Color::White,
            BorderColor::Gray => Color::Gray,
            BorderColor::DarkGray => Color::DarkGray,
        }
    }
    fn name(self) -> &'static str {
        match self {
            BorderColor::White => "White",
            BorderColor::Gray => "Gray",
            BorderColor::DarkGray => "Dark",
        }
    }
    fn all() -> &'static [BorderColor] {
        &[BorderColor::White, BorderColor::Gray, BorderColor::DarkGray]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatusColor {
    Gray,
    DarkGray,
    White,
}

impl Default for StatusColor {
    fn default() -> Self {
        StatusColor::Gray
    }
}

impl StatusColor {
    fn to_ratatui(self) -> Color {
        match self {
            StatusColor::Gray => Color::Gray,
            StatusColor::DarkGray => Color::DarkGray,
            StatusColor::White => Color::White,
        }
    }
    fn name(self) -> &'static str {
        match self {
            StatusColor::Gray => "Gray",
            StatusColor::DarkGray => "Dark",
            StatusColor::White => "White",
        }
    }
    fn all() -> &'static [StatusColor] {
        &[StatusColor::Gray, StatusColor::DarkGray, StatusColor::White]
    }
}

pub struct App {
    running: bool,
    event_stream: EventStream,
    all_lines: Vec<String>,
    filter: String,
    filter_cursor: usize,
    focus: Focus,
    list_state: ListState,
    /// Live mode: follow new lines from file
    live: bool,
    /// File path for live mode (only when loaded from file)
    live_file_path: Option<PathBuf>,
    /// Last byte position read in the file
    live_file_offset: u64,
    /// Incomplete line from last read (merged on next read for full line)
    live_partial: String,
    /// 1-based file line number of the first line in all_lines (for display)
    file_line_start: usize,
    /// Settings panel visible
    show_settings: bool,
    /// Selected row in settings (0=Accent, 1=Text color, 2=Text style, 3=Border, 4=Status, 5=Back)
    settings_list_state: ListState,
    accent_color: AccentColor,
    text_color: TextColor,
    text_style: TextStyle,
    border_color: BorderColor,
    status_color: StatusColor,
}

impl App {
    pub fn new(
        mut all_lines: Vec<String>,
        live_file_path: Option<PathBuf>,
        live_file_offset: u64,
        mut file_line_start: usize,
    ) -> Self {
        if all_lines.len() > MAX_LINES {
            let drop = all_lines.len() - MAX_LINES;
            all_lines.drain(0..drop);
            file_line_start += drop;
        }
        let mut list_state = ListState::default();
        if !all_lines.is_empty() {
            list_state.select(Some(0));
        }
        let (accent_color, text_color, text_style, border_color, status_color) = load_settings();
        Self {
            running: true,
            event_stream: EventStream::default(),
            all_lines,
            filter: String::new(),
            filter_cursor: 0,
            focus: Focus::LogList,
            list_state,
            live: false,
            live_file_path,
            live_file_offset: live_file_offset,
            live_partial: String::new(),
            file_line_start,
            show_settings: false,
            settings_list_state: ListState::default().with_selected(Some(0)),
            accent_color,
            text_color,
            text_style,
            border_color,
            status_color,
        }
    }

    fn save_settings_to_disk(&self) {
        save_settings(
            self.accent_color,
            self.text_color,
            self.text_style,
            self.border_color,
            self.status_color,
        );
    }

    fn border_style(&self) -> Style {
        Style::default().fg(self.border_color.to_ratatui())
    }

    fn accent_style(&self) -> Style {
        Style::default().fg(self.accent_color.to_ratatui())
    }

    fn log_text_style(&self) -> Style {
        Style::default()
            .fg(self.text_color.to_ratatui())
            .add_modifier(self.text_style.modifier())
    }

    fn status_style(&self) -> Style {
        Style::default().fg(self.status_color.to_ratatui())
    }

    /// If live mode is on, reads new lines from file and appends to `all_lines`.
    fn poll_live_file(&mut self) {
        let path = match &self.live_file_path {
            Some(p) => p.clone(),
            None => return,
        };
        let mut file = match File::open(&path) {
            Ok(f) => f,
            Err(_) => return,
        };
        let _ = file.seek(SeekFrom::Start(self.live_file_offset));
        let mut buf = Vec::new();
        if file.read_to_end(&mut buf).is_err() {
            return;
        }
        let new_len = match file.stream_position() {
            Ok(n) => n,
            Err(_) => self.live_file_offset + buf.len() as u64,
        };
        if buf.is_empty() {
            return;
        }
        let s = match String::from_utf8(buf) {
            Ok(x) => x,
            Err(_) => return,
        };
        let mut full = std::mem::take(&mut self.live_partial);
        full.push_str(&s);
        let lines: Vec<&str> = full.split('\n').collect();
        if full.ends_with('\n') {
            self.live_partial.clear();
            for line in lines {
                if !line.is_empty() {
                    self.all_lines.push(line.to_string());
                }
            }
        } else {
            let (complete, last) = lines.split_at(lines.len().saturating_sub(1));
            for line in complete {
                self.all_lines.push(line.to_string());
            }
            self.live_partial = last.first().copied().unwrap_or("").to_string();
        }
        self.live_file_offset = new_len;
        if self.all_lines.len() > MAX_LINES {
            let drop = self.all_lines.len() - MAX_LINES;
            self.all_lines.drain(0..drop);
            self.file_line_start += drop;
        }
        self.list_state.select_last();
    }

    /// Returns (all_lines_index, line) for display; used to show file line numbers. Index is 0-based in all_lines.
    fn filtered_lines_with_indices(&self) -> Vec<(usize, String)> {
        apply_filter(&self.all_lines, &self.filter, MAX_LINES)
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        while self.running {
            terminal.draw(|frame| self.draw(frame))?;
            if self.live {
                self.poll_live_file();
            }
            self.handle_crossterm_events().await?;
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        if self.show_settings {
            self.draw_settings(frame);
            return;
        }
        let area = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area);

        let filtered_with_idx = self.filtered_lines_with_indices();
        self.ensure_list_selection_in_bounds(filtered_with_idx.len());

        let border_style = self.border_style();
        let accent = self.accent_style();
        let log_style = self.log_text_style();

        // Top: filter area
        let filter_label = if self.focus == Focus::Filter {
            " Filter (focus) "
        } else {
            " Filter "
        };
        let block = Block::bordered()
            .title(filter_label)
            .border_style(border_style)
            .style(if self.focus == Focus::Filter {
                accent
            } else {
                Style::default()
            });
        let filter_display = format!("{}", self.filter);
        let cursor_pos = self.filter_cursor.min(filter_display.len());
        let para = Paragraph::new(filter_display.as_str())
            .block(block)
            .wrap(Wrap { trim: true });
        frame.render_widget(para, chunks[0]);
        if self.focus == Focus::Filter && chunks[0].width > 2 && chunks[0].height > 0 {
            let x = chunks[0].x + 1 + cursor_pos as u16;
            if x < chunks[0].x + chunks[0].width {
                frame.set_cursor_position((x, chunks[0].y + 1));
            }
        }

        // Middle: log list with file line numbers (text colour and style from settings)
        let items: Vec<ListItem> = filtered_with_idx
            .iter()
            .map(|(idx, s)| {
                let file_line = self.file_line_start + idx;
                let line = format!("{:>6} │ {}", file_line, s.as_str());
                ListItem::new(line).style(log_style)
            })
            .collect();
        let list = List::new(items)
            .block(Block::bordered().title(" Logs ").border_style(border_style))
            .highlight_style(accent.add_modifier(Modifier::REVERSED))
            .highlight_symbol(" ▸ ")
            .highlight_spacing(HighlightSpacing::Always);
        frame.render_stateful_widget(list, chunks[1], &mut self.list_state);

        // Status bar (colour from settings)
        let live_tag = if self.live { " LIVE " } else { "" };
        let mem = current_process_memory();
        let status = format!(
            " {} / {} lines {} |  RAM: {}  |  Filter: \"{}\"  |  Tab/ /: filter  |  L: live  |  S: settings  |  q/Esc: quit ",
            filtered_with_idx.len(),
            self.all_lines.len(),
            live_tag,
            mem,
            if self.filter.is_empty() {
                "(none)"
            } else {
                self.filter.as_str()
            }
        );
        let status_para = Paragraph::new(status).style(self.status_style());
        frame.render_widget(status_para, chunks[2]);

        // Bottom line: en üst / en alt (g / G)
        let bottom_hint = " g: en üst  │  G: en alt ";
        let hint_para = Paragraph::new(bottom_hint).style(self.status_style());
        frame.render_widget(hint_para, chunks[3]);
    }

    fn draw_settings(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let items = [
            ListItem::new(format!(
                " Accent (focus/highlight): {}  (←/→) ",
                self.accent_color.name()
            )),
            ListItem::new(format!(" Text colour: {}  (←/→) ", self.text_color.name())),
            ListItem::new(format!(" Text style: {}  (←/→) ", self.text_style.name())),
            ListItem::new(format!(
                " Border colour: {}  (←/→) ",
                self.border_color.name()
            )),
            ListItem::new(format!(
                " Status bar colour: {}  (←/→) ",
                self.status_color.name()
            )),
            ListItem::new(" Back (Enter or Esc) "),
        ];
        let list = List::new(items)
            .block(
                Block::bordered()
                    .title(" Settings ")
                    .style(self.accent_style()),
            )
            .highlight_style(Style::default().reversed())
            .highlight_symbol(" ▸ ")
            .highlight_spacing(HighlightSpacing::Always);
        let settings_area = centered_rect(area, 56, 50);
        frame.render_widget(Clear, settings_area);
        frame.render_stateful_widget(list, settings_area, &mut self.settings_list_state);
    }

    fn ensure_list_selection_in_bounds(&mut self, len: usize) {
        if len == 0 {
            self.list_state.select(None);
            *self.list_state.offset_mut() = 0;
            return;
        }
        let selected = self.list_state.selected().unwrap_or(0);
        if selected >= len {
            self.list_state.select(Some(len.saturating_sub(1)));
        }
        // When filter shrinks the list, offset may be too large; reset so all lines are visible.
        if *self.list_state.offset_mut() >= len {
            *self.list_state.offset_mut() = 0;
        }
    }

    async fn handle_crossterm_events(&mut self) -> color_eyre::Result<()> {
        let next_event = self.event_stream.next().fuse();
        if self.live {
            tokio::select! {
                event = next_event => {
                    if let Some(Ok(evt)) = event {
                        match evt {
                            Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
                            Event::Resize(_, _) => {}
                            _ => {}
                        }
                    }
                }
                _ = sleep(Duration::from_millis(400)) => { /* wake for periodic refresh */ }
            }
        } else {
            let event = next_event.await;
            if let Some(Ok(evt)) = event {
                match evt {
                    Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
                    Event::Resize(_, _) => {}
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn on_key_event(&mut self, key: KeyEvent) {
        if self.show_settings {
            self.on_key_settings(key);
            return;
        }
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => {
                self.quit();
                return;
            }
            (_, KeyCode::Esc) if self.focus != Focus::Filter => {
                self.quit();
                return;
            }
            _ => {}
        }

        if self.focus == Focus::Filter {
            self.on_key_filter(key);
            return;
        }
        self.on_key_log_list(key);
    }

    fn on_key_settings(&mut self, key: KeyEvent) {
        const SETTINGS_LEN: usize = 6;
        let cycle_next = |current: usize, len: usize| (current + 1) % len;
        let cycle_prev = |current: usize, len: usize| (current + len - 1) % len;
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                self.show_settings = false;
            }
            (_, KeyCode::Enter) => {
                let i = self.settings_list_state.selected().unwrap_or(0);
                if i == 5 {
                    self.show_settings = false;
                } else {
                    // Same as Right: cycle to next
                    match i {
                        0 => {
                            let opts = AccentColor::all();
                            let idx = opts
                                .iter()
                                .position(|&c| c == self.accent_color)
                                .unwrap_or(0);
                            self.accent_color = opts[cycle_next(idx, opts.len())];
                        }
                        1 => {
                            let opts = TextColor::all();
                            let idx = opts.iter().position(|&c| c == self.text_color).unwrap_or(0);
                            self.text_color = opts[cycle_next(idx, opts.len())];
                        }
                        2 => {
                            let opts = TextStyle::all();
                            let idx = opts.iter().position(|&c| c == self.text_style).unwrap_or(0);
                            self.text_style = opts[cycle_next(idx, opts.len())];
                        }
                        3 => {
                            let opts = BorderColor::all();
                            let idx = opts
                                .iter()
                                .position(|&c| c == self.border_color)
                                .unwrap_or(0);
                            self.border_color = opts[cycle_next(idx, opts.len())];
                        }
                        4 => {
                            let opts = StatusColor::all();
                            let idx = opts
                                .iter()
                                .position(|&c| c == self.status_color)
                                .unwrap_or(0);
                            self.status_color = opts[cycle_next(idx, opts.len())];
                        }
                        _ => {}
                    }
                    if (0..=4).contains(&i) {
                        self.save_settings_to_disk();
                    }
                }
            }
            (_, KeyCode::Up | KeyCode::Char('k')) => {
                self.settings_list_state.select_previous();
                if self.settings_list_state.selected().is_none() {
                    self.settings_list_state.select(Some(SETTINGS_LEN - 1));
                }
            }
            (_, KeyCode::Down | KeyCode::Char('j')) => {
                self.settings_list_state.select_next();
                if self.settings_list_state.selected().unwrap_or(0) >= SETTINGS_LEN {
                    self.settings_list_state.select(Some(0));
                }
            }
            (_, KeyCode::Left) => {
                let i = self.settings_list_state.selected().unwrap_or(0);
                match i {
                    0 => {
                        let opts = AccentColor::all();
                        let idx = opts
                            .iter()
                            .position(|&c| c == self.accent_color)
                            .unwrap_or(0);
                        self.accent_color = opts[cycle_prev(idx, opts.len())];
                    }
                    1 => {
                        let opts = TextColor::all();
                        let idx = opts.iter().position(|&c| c == self.text_color).unwrap_or(0);
                        self.text_color = opts[cycle_prev(idx, opts.len())];
                    }
                    2 => {
                        let opts = TextStyle::all();
                        let idx = opts.iter().position(|&c| c == self.text_style).unwrap_or(0);
                        self.text_style = opts[cycle_prev(idx, opts.len())];
                    }
                    3 => {
                        let opts = BorderColor::all();
                        let idx = opts
                            .iter()
                            .position(|&c| c == self.border_color)
                            .unwrap_or(0);
                        self.border_color = opts[cycle_prev(idx, opts.len())];
                    }
                    4 => {
                        let opts = StatusColor::all();
                        let idx = opts
                            .iter()
                            .position(|&c| c == self.status_color)
                            .unwrap_or(0);
                        self.status_color = opts[cycle_prev(idx, opts.len())];
                    }
                    _ => {}
                }
                if (0..=4).contains(&i) {
                    self.save_settings_to_disk();
                }
            }
            (_, KeyCode::Right) => {
                let i = self.settings_list_state.selected().unwrap_or(0);
                match i {
                    0 => {
                        let opts = AccentColor::all();
                        let idx = opts
                            .iter()
                            .position(|&c| c == self.accent_color)
                            .unwrap_or(0);
                        self.accent_color = opts[cycle_next(idx, opts.len())];
                    }
                    1 => {
                        let opts = TextColor::all();
                        let idx = opts.iter().position(|&c| c == self.text_color).unwrap_or(0);
                        self.text_color = opts[cycle_next(idx, opts.len())];
                    }
                    2 => {
                        let opts = TextStyle::all();
                        let idx = opts.iter().position(|&c| c == self.text_style).unwrap_or(0);
                        self.text_style = opts[cycle_next(idx, opts.len())];
                    }
                    3 => {
                        let opts = BorderColor::all();
                        let idx = opts
                            .iter()
                            .position(|&c| c == self.border_color)
                            .unwrap_or(0);
                        self.border_color = opts[cycle_next(idx, opts.len())];
                    }
                    4 => {
                        let opts = StatusColor::all();
                        let idx = opts
                            .iter()
                            .position(|&c| c == self.status_color)
                            .unwrap_or(0);
                        self.status_color = opts[cycle_next(idx, opts.len())];
                    }
                    _ => {}
                }
                if (0..=4).contains(&i) {
                    self.save_settings_to_disk();
                }
            }
            _ => {}
        }
    }

    fn on_key_filter(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                if self.filter.is_empty() {
                    self.quit();
                } else {
                    self.filter.clear();
                    self.filter_cursor = 0;
                }
            }
            (_, KeyCode::Enter) | (_, KeyCode::Tab) => {
                self.focus = Focus::LogList;
            }
            (_, KeyCode::Backspace) => {
                if self.filter_cursor > 0 {
                    self.filter_cursor -= 1;
                    self.filter.remove(self.filter_cursor);
                }
            }
            (_, KeyCode::Char(c)) if !c.is_control() => {
                self.filter.insert(self.filter_cursor, c);
                self.filter_cursor += 1;
            }
            (KeyModifiers::CONTROL, KeyCode::Char('a')) => {
                self.filter_cursor = 0;
            }
            (KeyModifiers::CONTROL, KeyCode::Char('e')) => {
                self.filter_cursor = self.filter.len();
            }
            (_, KeyCode::Left) => {
                self.filter_cursor = self.filter_cursor.saturating_sub(1);
            }
            (_, KeyCode::Right) => {
                self.filter_cursor = (self.filter_cursor + 1).min(self.filter.len());
            }
            _ => {}
        }
    }

    fn on_key_log_list(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Char('s') | KeyCode::Char('S')) => {
                self.show_settings = true;
                self.settings_list_state.select(Some(0));
            }
            (_, KeyCode::Char('/')) | (KeyModifiers::CONTROL, KeyCode::Char('f')) => {
                self.focus = Focus::Filter;
            }
            (_, KeyCode::Tab) => {
                self.focus = Focus::Filter;
            }
            (
                _,
                KeyCode::Char('l') | KeyCode::Char('L') | KeyCode::Char('f') | KeyCode::Char('F'),
            ) => {
                if self.live_file_path.is_some() {
                    self.live = !self.live;
                }
            }
            (_, KeyCode::Up | KeyCode::Char('k')) => {
                self.list_state.select_previous();
            }
            (_, KeyCode::Down | KeyCode::Char('j')) => {
                self.list_state.select_next();
            }
            (_, KeyCode::PageUp) => {
                self.list_state.scroll_up_by(10);
            }
            (_, KeyCode::PageDown) => {
                self.list_state.scroll_down_by(10);
            }
            (_, KeyCode::Home) | (_, KeyCode::Char('g')) => {
                self.list_state.select_first();
            }
            (_, KeyCode::End) | (_, KeyCode::Char('G')) => {
                self.list_state.select_last();
            }
            _ => {}
        }
    }

    fn quit(&mut self) {
        self.running = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(start, 51); // 1-based first kept line
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
