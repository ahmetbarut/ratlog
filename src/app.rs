//! Main TUI app: state, draw, event handling.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures::FutureExt;
use futures::StreamExt;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    widgets::{Block, Clear, HighlightSpacing, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::constants::{MAX_LINES, POLL_READ_CAP};
use crate::logs::apply_filter;
use crate::settings::{load_settings, save_settings};
use crate::theme::{
    self, AccentColor, BorderColor, Focus, StatusColor, TextColor, TextStyle,
};
use crate::util::{centered_rect, current_process_memory};

pub struct App {
    running: bool,
    event_stream: EventStream,
    all_lines: Vec<String>,
    filter: String,
    filter_cursor: usize,
    focus: Focus,
    list_state: ListState,
    live: bool,
    live_file_path: Option<PathBuf>,
    live_file_offset: u64,
    live_partial: String,
    file_line_start: usize,
    show_settings: bool,
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
            live_file_offset,
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
        theme::border_style(self.border_color)
    }

    fn accent_style(&self) -> Style {
        theme::accent_style(self.accent_color)
    }

    fn log_text_style(&self) -> Style {
        theme::log_text_style(self.text_color, self.text_style)
    }

    fn status_style(&self) -> Style {
        theme::status_style(self.status_color)
    }

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
        let mut buf = Vec::with_capacity(POLL_READ_CAP);
        let mut limited = (&mut file).take(POLL_READ_CAP as u64);
        if limited.read_to_end(&mut buf).is_err() {
            return;
        }
        let new_len = self.live_file_offset + buf.len() as u64;
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
        let filter_display = self.filter.to_string();
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
                _ = tokio::time::sleep(Duration::from_millis(400)) => {}
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
