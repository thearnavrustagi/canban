use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::ui::theme;

const BOARD_BINDINGS: &[(&str, &str)] = &[
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
    ("t", "Set tags"),
    ("D", "Set due date"),
    ("/", "Search / filter"),
    (":", "Command mode"),
    ("b", "Switch board"),
    ("?", "Toggle this help"),
    ("q / Ctrl-c", "Quit"),
];

const DIALOG_NORMAL: &[(&str, &str)] = &[
    ("i / a / A / I", "Enter insert mode"),
    ("v", "Enter visual mode"),
    ("R", "Enter replace mode"),
    ("h / l", "Move cursor left / right"),
    ("w / b / e", "Word forward / back / end"),
    ("0 / ^ / $", "Start / first non-blank / end"),
    ("f / F / t / T", "Find char forward / backward"),
    ("; / ,", "Repeat / reverse last find"),
    ("j / k", "Next / prev field"),
];

const DIALOG_OPERATORS: &[(&str, &str)] = &[
    ("d{motion}", "Delete (e.g. dw, d$, dd)"),
    ("c{motion}", "Change (e.g. cw, ciw, cc)"),
    ("y{motion}", "Yank (e.g. yw, yy)"),
    ("x / X", "Delete char / backspace"),
    ("s / S / C / D", "Substitute / clear / change"),
    ("p / P", "Paste after / before cursor"),
    ("r{char}", "Replace char under cursor"),
    ("~", "Toggle case"),
    ("u / Ctrl-r", "Undo / redo"),
    (".", "Repeat last edit"),
];

const DIALOG_TEXT_OBJ: &[(&str, &str)] = &[
    ("iw / aw", "Inner word / a word"),
    ("i\" / a\"", "Inner / around quotes"),
    ("i( / a(", "Inner / around parens"),
];

const DIALOG_VISUAL: &[(&str, &str)] = &[
    ("d / x", "Delete selection"),
    ("c / s", "Change selection"),
    ("y", "Yank selection"),
    ("r{char}", "Replace all in selection"),
    ("~", "Toggle case of selection"),
    ("o", "Swap cursor and anchor"),
    ("Esc / v", "Exit visual mode"),
];

const DIALOG_INSERT: &[(&str, &str)] = &[
    ("Esc", "Return to normal mode"),
    ("Tab / S-Tab", "Next / prev field"),
    ("Ctrl-w", "Delete word backward"),
    ("Ctrl-u", "Clear field"),
    ("Enter", "Confirm"),
];

pub fn render(f: &mut Frame, area: Rect) {
    let width = 56u16.min(area.width.saturating_sub(4));
    let total_lines = BOARD_BINDINGS.len()
        + DIALOG_NORMAL.len()
        + DIALOG_OPERATORS.len()
        + DIALOG_TEXT_OBJ.len()
        + DIALOG_VISUAL.len()
        + DIALOG_INSERT.len()
        + 14;
    let height = (total_lines as u16 + 4).min(area.height.saturating_sub(2));

    let [popup_area] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(center_vertical(area, height));

    f.render_widget(Clear, popup_area);

    let mut lines = Vec::new();

    lines.push(section_header("Board Navigation"));
    for (key, desc) in BOARD_BINDINGS {
        lines.push(binding_line(key, desc));
    }

    lines.push(Line::from(""));
    lines.push(section_header("Dialog ─ Normal Mode"));
    for (key, desc) in DIALOG_NORMAL {
        lines.push(binding_line(key, desc));
    }

    lines.push(Line::from(""));
    lines.push(section_header("Dialog ─ Operators & Edits"));
    for (key, desc) in DIALOG_OPERATORS {
        lines.push(binding_line(key, desc));
    }

    lines.push(Line::from(""));
    lines.push(section_header("Dialog ─ Text Objects"));
    for (key, desc) in DIALOG_TEXT_OBJ {
        lines.push(binding_line(key, desc));
    }

    lines.push(Line::from(""));
    lines.push(section_header("Dialog ─ Visual Mode"));
    for (key, desc) in DIALOG_VISUAL {
        lines.push(binding_line(key, desc));
    }

    lines.push(Line::from(""));
    lines.push(section_header("Dialog ─ Insert Mode"));
    for (key, desc) in DIALOG_INSERT {
        lines.push(binding_line(key, desc));
    }

    let block = Block::default()
        .title(" Keybindings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT_CYAN));

    let para = Paragraph::new(lines).block(block);
    f.render_widget(para, popup_area);
}

fn section_header(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!("── {title} ──"),
        Style::default()
            .fg(theme::ACCENT_YELLOW)
            .add_modifier(Modifier::BOLD),
    ))
}

fn binding_line(key: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {:<16}", key),
            Style::default()
                .fg(theme::ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(desc.to_string(), Style::default().fg(theme::FG_TEXT)),
    ])
}

fn center_vertical(area: Rect, height: u16) -> Rect {
    let top = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(area.x, top, area.width, height)
}
