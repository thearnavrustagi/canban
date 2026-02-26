use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::ui::theme;

pub struct InputDialog {
    pub title: String,
    pub fields: Vec<FieldState>,
    pub active_field: usize,
}

pub struct FieldState {
    pub label: String,
    pub value: String,
    pub cursor: usize,
}

impl FieldState {
    pub fn new(label: &str, value: &str) -> Self {
        let cursor = value.len();
        Self {
            label: label.to_string(),
            value: value.to_string(),
            cursor,
        }
    }

    pub fn insert(&mut self, ch: char) {
        self.value.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            let prev = self.value[..self.cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.value.remove(prev);
            self.cursor = prev;
        }
    }

    pub fn delete(&mut self) {
        if self.cursor < self.value.len() {
            self.value.remove(self.cursor);
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.value[..self.cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.value.len() {
            self.cursor += self.value[self.cursor..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
        }
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.value.len();
    }
}

impl InputDialog {
    pub fn active_field_mut(&mut self) -> &mut FieldState {
        &mut self.fields[self.active_field]
    }

    pub fn next_field(&mut self) {
        self.active_field = (self.active_field + 1) % self.fields.len();
    }

    pub fn prev_field(&mut self) {
        if self.active_field == 0 {
            self.active_field = self.fields.len() - 1;
        } else {
            self.active_field -= 1;
        }
    }
}

pub fn render_input_dialog(f: &mut Frame, area: Rect, dialog: &InputDialog) {
    let width = 60u16.min(area.width.saturating_sub(4));
    let height = (dialog.fields.len() as u16 * 3 + 4).min(area.height.saturating_sub(2));
    let popup = centered(area, width, height);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .title(format!(" {} ", dialog.title))
        .borders(Borders::ALL)
        .border_style(theme::dialog_border_style());

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let constraints: Vec<Constraint> = dialog
        .fields
        .iter()
        .flat_map(|_| [Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)])
        .collect();

    let rows = Layout::vertical(constraints).split(inner);

    for (i, field) in dialog.fields.iter().enumerate() {
        let base = i * 3;
        let is_active = i == dialog.active_field;

        let label_style = if is_active {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let label = Paragraph::new(Line::from(Span::styled(
            format!("{}:", field.label),
            label_style,
        )));
        if base < rows.len() {
            f.render_widget(label, rows[base]);
        }

        if base + 1 < rows.len() {
            let (before, cursor_ch, after) = split_at_cursor(&field.value, field.cursor);
            let line = Line::from(vec![
                Span::styled(before, theme::input_style()),
                Span::styled(
                    cursor_ch,
                    if is_active {
                        Style::default().fg(Color::Black).bg(Color::Yellow)
                    } else {
                        theme::input_style()
                    },
                ),
                Span::styled(after, theme::input_style()),
            ]);
            f.render_widget(Paragraph::new(line), rows[base + 1]);
        }
    }
}

pub fn render_confirm(f: &mut Frame, area: Rect, message: &str) {
    let width = 50u16.min(area.width.saturating_sub(4));
    let popup = centered(area, width, 5);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Confirm ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let text = vec![
        Line::from(message.to_string()),
        Line::from(""),
        Line::from(vec![
            Span::styled("y", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" / "),
            Span::styled("n", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        ]),
    ];

    let para = Paragraph::new(text).block(block);
    f.render_widget(para, popup);
}

pub fn render_board_picker(f: &mut Frame, area: Rect, boards: &[String], selected: usize) {
    let width = 40u16.min(area.width.saturating_sub(4));
    let height = (boards.len() as u16 + 4).min(area.height.saturating_sub(2));
    let popup = centered(area, width, height);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Select Board ")
        .borders(Borders::ALL)
        .border_style(theme::dialog_border_style());

    let lines: Vec<Line> = boards
        .iter()
        .enumerate()
        .map(|(i, name)| {
            if i == selected {
                Line::from(Span::styled(
                    format!("> {name}"),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(format!("  {name}"))
            }
        })
        .collect();

    let para = Paragraph::new(lines).block(block);
    f.render_widget(para, popup);
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let [h_area] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(area);
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(h_area.x, y, width, height)
}

fn split_at_cursor(s: &str, cursor: usize) -> (String, String, String) {
    let before = s[..cursor].to_string();
    if cursor < s.len() {
        let ch = &s[cursor..cursor + s[cursor..].chars().next().map(|c| c.len_utf8()).unwrap_or(0)];
        let after = s[cursor + ch.len()..].to_string();
        (before, ch.to_string(), after)
    } else {
        (before, " ".to_string(), String::new())
    }
}
