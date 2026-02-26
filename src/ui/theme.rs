use ratatui::style::{Color, Modifier, Style};

use crate::model::ColumnKind;

pub fn column_color(col: ColumnKind) -> Color {
    match col {
        ColumnKind::Ready => Color::Cyan,
        ColumnKind::Doing => Color::Yellow,
        ColumnKind::Done => Color::Green,
        ColumnKind::Archived => Color::DarkGray,
    }
}

pub fn column_style(col: ColumnKind, selected: bool) -> Style {
    let color = column_color(col);
    if selected {
        Style::default()
            .fg(color)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(color)
    }
}

pub fn card_style(selected: bool) -> Style {
    if selected {
        Style::default()
            .fg(Color::White)
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    }
}

pub fn tag_style() -> Style {
    Style::default()
        .fg(Color::Magenta)
        .add_modifier(Modifier::DIM)
}

pub fn overdue_style() -> Style {
    Style::default()
        .fg(Color::Red)
        .add_modifier(Modifier::BOLD)
}

pub fn due_style() -> Style {
    Style::default().fg(Color::Blue)
}

pub fn header_style() -> Style {
    Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

pub fn footer_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub fn input_style() -> Style {
    Style::default().fg(Color::Yellow)
}

pub fn dialog_border_style() -> Style {
    Style::default().fg(Color::Cyan)
}
