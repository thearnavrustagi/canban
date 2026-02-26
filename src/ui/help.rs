use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

const BINDINGS: &[(&str, &str)] = &[
    ("h / l", "Move between columns"),
    ("j / k", "Move between tasks"),
    ("g / G", "First / last task"),
    ("1-4", "Jump to column"),
    ("Tab", "Next column (wrap)"),
    ("n / a", "New task"),
    ("Enter / e", "Edit task"),
    ("r", "Rename task"),
    ("d", "Delete task"),
    ("Space / m", "Advance task →"),
    ("M", "Move task ←"),
    ("t", "Set tag"),
    ("D", "Set due date"),
    ("/", "Search / filter"),
    (":", "Command mode"),
    ("?", "Toggle this help"),
    ("q / Ctrl-c", "Quit"),
];

pub fn render(f: &mut Frame, area: Rect) {
    let width = 50u16.min(area.width.saturating_sub(4));
    let height = (BINDINGS.len() as u16 + 4).min(area.height.saturating_sub(2));

    let [popup_area] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(center_vertical(area, height));

    f.render_widget(Clear, popup_area);

    let lines: Vec<Line> = BINDINGS
        .iter()
        .map(|(key, desc)| {
            Line::from(vec![
                Span::styled(
                    format!("{:<16}", key),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(*desc),
            ])
        })
        .collect();

    let block = Block::default()
        .title(" Keybindings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let para = Paragraph::new(lines).block(block);
    f.render_widget(para, popup_area);
}

fn center_vertical(area: Rect, height: u16) -> Rect {
    let top = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(area.x, top, area.width, height)
}
