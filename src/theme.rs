//! Theme: focus, accent/text/border/status colours and text style.

use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Filter,
    LogList,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccentColor {
    #[default]
    Cyan,
    Green,
    Yellow,
    Magenta,
    Blue,
}

impl AccentColor {
    pub fn to_ratatui(self) -> Color {
        match self {
            AccentColor::Cyan => Color::Cyan,
            AccentColor::Green => Color::Green,
            AccentColor::Yellow => Color::Yellow,
            AccentColor::Magenta => Color::Magenta,
            AccentColor::Blue => Color::Blue,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            AccentColor::Cyan => "Cyan",
            AccentColor::Green => "Green",
            AccentColor::Yellow => "Yellow",
            AccentColor::Magenta => "Magenta",
            AccentColor::Blue => "Blue",
        }
    }
    pub fn all() -> &'static [AccentColor] {
        &[
            AccentColor::Cyan,
            AccentColor::Green,
            AccentColor::Yellow,
            AccentColor::Magenta,
            AccentColor::Blue,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextColor {
    #[default]
    White,
    Gray,
    Cyan,
    Green,
    Yellow,
}

impl TextColor {
    pub fn to_ratatui(self) -> Color {
        match self {
            TextColor::White => Color::White,
            TextColor::Gray => Color::Gray,
            TextColor::Cyan => Color::Cyan,
            TextColor::Green => Color::Green,
            TextColor::Yellow => Color::Yellow,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            TextColor::White => "White",
            TextColor::Gray => "Gray",
            TextColor::Cyan => "Cyan",
            TextColor::Green => "Green",
            TextColor::Yellow => "Yellow",
        }
    }
    pub fn all() -> &'static [TextColor] {
        &[
            TextColor::White,
            TextColor::Gray,
            TextColor::Cyan,
            TextColor::Green,
            TextColor::Yellow,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextStyle {
    #[default]
    Normal,
    Bold,
    Dim,
}

impl TextStyle {
    pub fn name(self) -> &'static str {
        match self {
            TextStyle::Normal => "Normal",
            TextStyle::Bold => "Bold",
            TextStyle::Dim => "Dim",
        }
    }
    pub fn modifier(self) -> Modifier {
        match self {
            TextStyle::Normal => Modifier::empty(),
            TextStyle::Bold => Modifier::BOLD,
            TextStyle::Dim => Modifier::DIM,
        }
    }
    pub fn all() -> &'static [TextStyle] {
        &[TextStyle::Normal, TextStyle::Bold, TextStyle::Dim]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderColor {
    White,
    #[default]
    Gray,
    DarkGray,
}

impl BorderColor {
    pub fn to_ratatui(self) -> Color {
        match self {
            BorderColor::White => Color::White,
            BorderColor::Gray => Color::Gray,
            BorderColor::DarkGray => Color::DarkGray,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            BorderColor::White => "White",
            BorderColor::Gray => "Gray",
            BorderColor::DarkGray => "Dark",
        }
    }
    pub fn all() -> &'static [BorderColor] {
        &[BorderColor::White, BorderColor::Gray, BorderColor::DarkGray]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusColor {
    #[default]
    Gray,
    DarkGray,
    White,
}

impl StatusColor {
    pub fn to_ratatui(self) -> Color {
        match self {
            StatusColor::Gray => Color::Gray,
            StatusColor::DarkGray => Color::DarkGray,
            StatusColor::White => Color::White,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            StatusColor::Gray => "Gray",
            StatusColor::DarkGray => "Dark",
            StatusColor::White => "White",
        }
    }
    pub fn all() -> &'static [StatusColor] {
        &[StatusColor::Gray, StatusColor::DarkGray, StatusColor::White]
    }
}

pub fn border_style(border_color: BorderColor) -> Style {
    Style::default().fg(border_color.to_ratatui())
}

pub fn accent_style(accent_color: AccentColor) -> Style {
    Style::default().fg(accent_color.to_ratatui())
}

pub fn log_text_style(text_color: TextColor, text_style: TextStyle) -> Style {
    Style::default()
        .fg(text_color.to_ratatui())
        .add_modifier(text_style.modifier())
}

pub fn status_style(status_color: StatusColor) -> Style {
    Style::default().fg(status_color.to_ratatui())
}
