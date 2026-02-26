use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;

const LOGO: &[&str] = &[
    " ██████╗ █████╗ ███╗   ██╗██████╗  █████╗ ███╗   ██╗",
    "██╔════╝██╔══██╗████╗  ██║██╔══██╗██╔══██╗████╗  ██║",
    "██║     ███████║██╔██╗ ██║██████╔╝███████║██╔██╗ ██║",
    "██║     ██╔══██║██║╚██╗██║██╔══██╗██╔══██║██║╚██╗██║",
    "╚██████╗██║  ██║██║ ╚████║██████╔╝██║  ██║██║ ╚████║",
    " ╚═════╝╚═╝  ╚═╝╚═╝  ╚═══╝╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═══╝",
];

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    let logo_height = LOGO.len() as u16;
    let board_box_height = (app.splash_boards.len().max(1) as u16 + 2).min(area.height / 3);
    let total_height = logo_height + 2 + board_box_height + 1 + 1;
    let content_area = center_vertical(area, total_height);

    let [logo_area, _, board_area, _, hint_area] = Layout::vertical([
        Constraint::Length(logo_height),
        Constraint::Length(2),
        Constraint::Length(board_box_height),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(content_area);

    render_logo(f, logo_area, app.tick);
    render_board_list(f, board_area, &app.splash_boards, app.splash_board_idx);
    render_hint(f, hint_area);
}

fn render_logo(f: &mut Frame, area: Rect, tick: u64) {
    let lines: Vec<Line> = LOGO
        .iter()
        .enumerate()
        .map(|(row, line)| rainbow_line(line, row, tick))
        .collect();
    let para = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(para, area);
}

fn rainbow_line(text: &str, row: usize, tick: u64) -> Line<'static> {
    let spans: Vec<Span> = text
        .chars()
        .enumerate()
        .map(|(col, ch)| {
            if ch == ' ' {
                Span::raw(" ")
            } else {
                let hue =
                    ((col as f64 * 6.0) + (row as f64 * 20.0) + (tick as f64 * 8.0)) % 360.0;
                let (r, g, b) = hsl_to_rgb(hue, 0.85, 0.65);
                Span::styled(
                    ch.to_string(),
                    Style::default()
                        .fg(Color::Rgb(r, g, b))
                        .add_modifier(Modifier::BOLD),
                )
            }
        })
        .collect();
    Line::from(spans)
}

pub fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r1, g1, b1) = match h as u32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    (
        ((r1 + m) * 255.0) as u8,
        ((g1 + m) * 255.0) as u8,
        ((b1 + m) * 255.0) as u8,
    )
}

fn render_board_list(f: &mut Frame, area: Rect, boards: &[String], selected: usize) {
    let width = 44u16.min(area.width.saturating_sub(4));
    let [centered] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(area);

    let block = Block::default()
        .title(" Boards ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(centered);
    f.render_widget(block, centered);

    if boards.is_empty() {
        let empty = Paragraph::new(Line::from(Span::styled(
            "No boards yet. Press n to create one.",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )))
        .alignment(Alignment::Center);
        f.render_widget(empty, inner);
        return;
    }

    let visible_count = inner.height as usize;
    let scroll = compute_scroll(selected, boards.len(), visible_count);
    let visible = &boards[scroll..boards.len().min(scroll + visible_count)];

    let constraints: Vec<Constraint> = visible
        .iter()
        .map(|_| Constraint::Length(1))
        .chain(std::iter::once(Constraint::Min(0)))
        .collect();
    let rows = Layout::vertical(constraints).split(inner);

    for (i, name) in visible.iter().enumerate() {
        let abs = scroll + i;
        let is_sel = abs == selected;

        let (marker, name_style) = if is_sel {
            (
                Span::styled(
                    " > ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            (
                Span::styled("   ", Style::default()),
                Style::default().fg(Color::DarkGray),
            )
        };

        let line = Line::from(vec![marker, Span::styled(name.clone(), name_style)]);
        f.render_widget(Paragraph::new(line), rows[i]);
    }
}

fn render_hint(f: &mut Frame, area: Rect) {
    let hints = vec![
        ("j/k", "navigate"),
        ("Enter", "open"),
        ("n", "new board"),
        ("d", "delete"),
        ("q", "quit"),
    ];

    let mut spans = Vec::new();
    for (i, (key, desc)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", Style::default()));
        }
        spans.push(Span::styled(
            (*key).to_string(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(": {desc}"),
            Style::default().fg(Color::DarkGray),
        ));
    }

    let para = Paragraph::new(Line::from(spans)).alignment(Alignment::Center);
    f.render_widget(para, area);
}

fn center_vertical(area: Rect, height: u16) -> Rect {
    let h = height.min(area.height);
    let y = area.y + area.height.saturating_sub(h) / 2;
    Rect::new(area.x, y, area.width, h)
}

fn compute_scroll(selected: usize, total: usize, visible: usize) -> usize {
    if total <= visible {
        return 0;
    }
    if selected < visible / 2 {
        return 0;
    }
    let max = total.saturating_sub(visible);
    (selected.saturating_sub(visible / 2)).min(max)
}
