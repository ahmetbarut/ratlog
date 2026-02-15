//! Helpers: format_bytes, centered_rect, current_process_memory.

use ratatui::layout::Rect;

pub fn format_bytes(bytes: u64) -> String {
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

pub fn centered_rect(area: Rect, width_pct: u16, height_pct: u16) -> Rect {
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

pub fn current_process_memory() -> String {
    use sysinfo::System;
    let mut sys = System::new_all();
    sys.refresh_all();
    if let Some(me) = sys.process(sysinfo::Pid::from(std::process::id() as usize)) {
        format_bytes(me.memory())
    } else {
        "—".to_string()
    }
}
