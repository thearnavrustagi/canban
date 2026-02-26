use ratatui::style::{Color, Modifier, Style};

use crate::model::ColumnKind;

pub const BG_DARK: Color = Color::Rgb(18, 18, 28);
pub const BG_MID: Color = Color::Rgb(30, 30, 45);
pub const BG_RAISED: Color = Color::Rgb(40, 40, 60);
pub const FG_MUTED: Color = Color::Rgb(100, 100, 120);
pub const FG_DIM: Color = Color::Rgb(140, 140, 160);
pub const FG_TEXT: Color = Color::Rgb(200, 200, 210);
pub const FG_BRIGHT: Color = Color::Rgb(230, 230, 240);
pub const ACCENT_CYAN: Color = Color::Rgb(80, 200, 220);
pub const ACCENT_YELLOW: Color = Color::Rgb(240, 200, 80);
pub const ACCENT_GREEN: Color = Color::Rgb(80, 220, 140);
pub const ACCENT_ARCHIVE: Color = Color::Rgb(140, 130, 170);
#[allow(dead_code)]
pub const ACCENT_MAGENTA: Color = Color::Rgb(200, 120, 220);
pub const ACCENT_BLUE: Color = Color::Rgb(100, 140, 240);
pub const ACCENT_RED: Color = Color::Rgb(240, 90, 90);
#[allow(dead_code)]
pub const ACCENT_ORANGE: Color = Color::Rgb(240, 160, 80);

pub const TAG_COLORS: &[Color] = &[
    Color::Rgb(200, 120, 220),
    Color::Rgb(80, 180, 220),
    Color::Rgb(240, 160, 80),
    Color::Rgb(120, 220, 160),
    Color::Rgb(220, 120, 140),
    Color::Rgb(160, 160, 240),
    Color::Rgb(200, 200, 100),
    Color::Rgb(100, 200, 200),
];

pub fn tag_color(idx: usize) -> Color {
    TAG_COLORS[idx % TAG_COLORS.len()]
}

pub fn column_color(col: ColumnKind) -> Color {
    match col {
        ColumnKind::Ready => ACCENT_CYAN,
        ColumnKind::Doing => ACCENT_YELLOW,
        ColumnKind::Done => ACCENT_GREEN,
        ColumnKind::Archived => ACCENT_ARCHIVE,
    }
}

pub fn column_icon(col: ColumnKind) -> &'static str {
    match col {
        ColumnKind::Ready => "◇",
        ColumnKind::Doing => "▸",
        ColumnKind::Done => "✓",
        ColumnKind::Archived => "◌",
    }
}

pub fn column_style(col: ColumnKind, selected: bool) -> Style {
    let color = column_color(col);
    if selected {
        Style::default()
            .fg(color)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(color).add_modifier(Modifier::DIM)
    }
}

pub fn card_style(selected: bool) -> Style {
    if selected {
        Style::default()
            .fg(FG_BRIGHT)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(FG_TEXT)
    }
}

#[allow(dead_code)]
pub fn tag_style() -> Style {
    Style::default().fg(ACCENT_MAGENTA)
}

pub fn overdue_style() -> Style {
    Style::default()
        .fg(ACCENT_RED)
        .add_modifier(Modifier::BOLD)
}

pub fn due_style() -> Style {
    Style::default().fg(ACCENT_BLUE)
}

pub fn header_style() -> Style {
    Style::default()
        .fg(FG_BRIGHT)
        .add_modifier(Modifier::BOLD)
}

#[allow(dead_code)]
pub fn footer_style() -> Style {
    Style::default().fg(FG_MUTED)
}

pub fn input_style() -> Style {
    Style::default().fg(ACCENT_YELLOW)
}

pub fn dialog_border_style() -> Style {
    Style::default().fg(ACCENT_CYAN)
}

pub fn vim_normal_cursor() -> Style {
    Style::default().fg(Color::Black).bg(ACCENT_CYAN)
}

pub fn vim_insert_cursor() -> Style {
    Style::default().fg(Color::Black).bg(ACCENT_YELLOW)
}

pub fn mode_badge_normal() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(ACCENT_CYAN)
        .add_modifier(Modifier::BOLD)
}

pub fn mode_badge_insert() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(ACCENT_GREEN)
        .add_modifier(Modifier::BOLD)
}
