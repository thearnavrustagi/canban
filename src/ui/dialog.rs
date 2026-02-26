use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::ui::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogVimMode {
    Normal,
    Insert,
}

pub struct InputDialog {
    pub title: String,
    pub fields: Vec<FieldState>,
    pub active_field: usize,
    pub vim_mode: DialogVimMode,
    pub tag_suggestions: Vec<String>,
}

pub struct FieldState {
    pub label: String,
    pub value: String,
    pub cursor: usize,
    pub placeholder: String,
}

impl FieldState {
    pub fn new(label: &str, value: &str) -> Self {
        let cursor = value.len();
        Self {
            label: label.to_string(),
            value: value.to_string(),
            cursor,
            placeholder: String::new(),
        }
    }

    pub fn with_placeholder(mut self, ph: &str) -> Self {
        self.placeholder = ph.to_string();
        self
    }

    pub fn insert(&mut self, ch: char) {
        self.value.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            let prev = prev_char_boundary(&self.value, self.cursor);
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
            self.cursor = prev_char_boundary(&self.value, self.cursor);
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.value.len() {
            self.cursor += next_char_len(&self.value, self.cursor);
        }
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.value.len();
    }

    pub fn move_word_forward(&mut self) {
        let bytes = self.value.as_bytes();
        let mut i = self.cursor;
        while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        self.cursor = i;
    }

    pub fn move_word_backward(&mut self) {
        let bytes = self.value.as_bytes();
        let mut i = self.cursor;
        if i > 0 {
            i -= 1;
        }
        while i > 0 && bytes[i].is_ascii_whitespace() {
            i -= 1;
        }
        while i > 0 && !bytes[i - 1].is_ascii_whitespace() {
            i -= 1;
        }
        self.cursor = i;
    }

    pub fn delete_word_backward(&mut self) {
        let start = self.cursor;
        self.move_word_backward();
        let end = self.cursor;
        if end < start {
            self.value.drain(end..start);
        }
    }

    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }
}

fn prev_char_boundary(s: &str, pos: usize) -> usize {
    s[..pos]
        .char_indices()
        .last()
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn next_char_len(s: &str, pos: usize) -> usize {
    s[pos..]
        .chars()
        .next()
        .map(|c| c.len_utf8())
        .unwrap_or(0)
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

    pub fn vim_mode_label(&self) -> &str {
        match self.vim_mode {
            DialogVimMode::Normal => "NORMAL",
            DialogVimMode::Insert => "INSERT",
        }
    }
}

pub fn render_input_dialog(f: &mut Frame, area: Rect, dialog: &InputDialog) {
    let width = 64u16.min(area.width.saturating_sub(4));
    let inner_width = width.saturating_sub(2);

    let value_heights: Vec<u16> = dialog
        .fields
        .iter()
        .map(|f| value_row_count(&f.value, inner_width))
        .collect();

    let field_rows: u16 = value_heights.iter().map(|h| 1 + h + 1).sum();
    let suggestion_rows = if !dialog.tag_suggestions.is_empty() { 3 } else { 0 };
    let height = (field_rows + suggestion_rows + 6).min(area.height.saturating_sub(2));
    let popup = centered(area, width, height);
    f.render_widget(Clear, popup);

    let border_color = match dialog.vim_mode {
        DialogVimMode::Normal => theme::ACCENT_CYAN,
        DialogVimMode::Insert => theme::ACCENT_GREEN,
    };

    let block = Block::default()
        .title(format!(" {} ", dialog.title))
        .title_bottom(Line::from(vec![
            Span::raw(" "),
            Span::styled(
                format!(" {} ", dialog.vim_mode_label()),
                match dialog.vim_mode {
                    DialogVimMode::Normal => theme::mode_badge_normal(),
                    DialogVimMode::Insert => theme::mode_badge_insert(),
                },
            ),
            Span::styled(
                match dialog.vim_mode {
                    DialogVimMode::Normal => " i:insert  j/k:fields  Enter:confirm  Esc:cancel ",
                    DialogVimMode::Insert => " Esc:normal  Tab:next field  Enter:confirm ",
                },
                Style::default().fg(theme::FG_MUTED),
            ),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let mut constraints: Vec<Constraint> = dialog
        .fields
        .iter()
        .enumerate()
        .flat_map(|(i, _)| {
            [
                Constraint::Length(1),
                Constraint::Length(value_heights[i]),
                Constraint::Length(1),
            ]
        })
        .collect();

    if !dialog.tag_suggestions.is_empty() {
        constraints.push(Constraint::Length(1));
        constraints.push(Constraint::Length(1));
        constraints.push(Constraint::Length(1));
    }
    constraints.push(Constraint::Min(0));

    let rows = Layout::vertical(constraints).split(inner);

    for (i, field) in dialog.fields.iter().enumerate() {
        let base = i * 3;
        let is_active = i == dialog.active_field;
        render_field(f, &rows, base, field, is_active, dialog.vim_mode);
    }

    if !dialog.tag_suggestions.is_empty() {
        let sug_base = dialog.fields.len() * 3;
        render_tag_suggestions(f, &rows, sug_base, &dialog.tag_suggestions);
    }
}

fn render_field(
    f: &mut Frame,
    rows: &[Rect],
    base: usize,
    field: &FieldState,
    is_active: bool,
    vim_mode: DialogVimMode,
) {
    let label_style = if is_active {
        Style::default()
            .fg(theme::ACCENT_CYAN)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::FG_DIM)
    };

    let indicator = if is_active { "▸ " } else { "  " };
    let label = Paragraph::new(Line::from(vec![
        Span::styled(indicator, label_style),
        Span::styled(format!("{}:", field.label), label_style),
    ]));
    if base < rows.len() {
        f.render_widget(label, rows[base]);
    }

    if base + 1 < rows.len() {
        let value_area = Rect {
            x: rows[base + 1].x + 2,
            width: rows[base + 1].width.saturating_sub(2),
            ..rows[base + 1]
        };

        let line = if field.value.is_empty() && !is_active {
            Line::from(Span::styled(
                &field.placeholder,
                Style::default().fg(theme::FG_MUTED).add_modifier(Modifier::ITALIC),
            ))
        } else {
            let cursor_style = if is_active {
                match vim_mode {
                    DialogVimMode::Normal => theme::vim_normal_cursor(),
                    DialogVimMode::Insert => theme::vim_insert_cursor(),
                }
            } else {
                theme::input_style()
            };

            let (before, cursor_ch, after) = split_at_cursor(&field.value, field.cursor);
            Line::from(vec![
                Span::styled(before, theme::input_style()),
                Span::styled(
                    if is_active { cursor_ch } else { cursor_ch.clone() },
                    if is_active { cursor_style } else { theme::input_style() },
                ),
                Span::styled(after, theme::input_style()),
            ])
        };
        f.render_widget(
            Paragraph::new(line).wrap(Wrap { trim: false }),
            value_area,
        );
    }
}

fn render_tag_suggestions(f: &mut Frame, rows: &[Rect], base: usize, suggestions: &[String]) {
    if base >= rows.len() {
        return;
    }

    let sep = Paragraph::new(Line::from(Span::styled(
        "  ── existing tags ──",
        Style::default().fg(theme::FG_MUTED),
    )));
    f.render_widget(sep, rows[base]);

    if base + 1 < rows.len() {
        let spans: Vec<Span> = suggestions
            .iter()
            .enumerate()
            .flat_map(|(i, tag)| {
                let color = theme::tag_color(i);
                let mut s = vec![Span::styled(
                    format!(" #{tag} "),
                    Style::default().fg(color),
                )];
                if i + 1 < suggestions.len() {
                    s.push(Span::styled("·", Style::default().fg(theme::FG_MUTED)));
                }
                s
            })
            .collect();

        let tag_line = Line::from(
            std::iter::once(Span::raw("  "))
                .chain(spans)
                .collect::<Vec<_>>(),
        );
        f.render_widget(Paragraph::new(tag_line), rows[base + 1]);
    }
}

pub fn render_confirm(f: &mut Frame, area: Rect, message: &str) {
    let width = 50u16.min(area.width.saturating_sub(4));
    let popup = centered(area, width, 5);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Confirm ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT_RED));

    let text = vec![
        Line::from(message.to_string()),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "y",
                Style::default()
                    .fg(theme::ACCENT_GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" / "),
            Span::styled(
                "n",
                Style::default()
                    .fg(theme::ACCENT_RED)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    let para = Paragraph::new(text).block(block);
    f.render_widget(para, popup);
}

pub fn render_unsaved_confirm(f: &mut Frame, area: Rect) {
    let width = 50u16.min(area.width.saturating_sub(4));
    let popup = centered(area, width, 7);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Unsaved Changes ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT_YELLOW));

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "You have unsaved changes.",
            Style::default().fg(theme::FG_BRIGHT),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                " s ",
                Style::default()
                    .fg(Color::Black)
                    .bg(theme::ACCENT_GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" save  ", Style::default().fg(theme::FG_DIM)),
            Span::styled(
                " d ",
                Style::default()
                    .fg(Color::Black)
                    .bg(theme::ACCENT_RED)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" discard  ", Style::default().fg(theme::FG_DIM)),
            Span::styled(
                " Esc ",
                Style::default()
                    .fg(Color::Black)
                    .bg(theme::ACCENT_CYAN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" go back", Style::default().fg(theme::FG_DIM)),
        ]),
    ];

    let para = Paragraph::new(text)
        .block(block)
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(para, popup);
}

pub fn render_board_picker(f: &mut Frame, area: Rect, boards: &[String], selected: usize) {
    let width = 44u16.min(area.width.saturating_sub(4));
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
                Line::from(vec![
                    Span::styled(
                        " ▸ ",
                        Style::default()
                            .fg(theme::ACCENT_CYAN)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        name.to_string(),
                        Style::default()
                            .fg(theme::FG_BRIGHT)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::raw("   "),
                    Span::styled(name.to_string(), Style::default().fg(theme::FG_DIM)),
                ])
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

fn value_row_count(value: &str, max_width: u16) -> u16 {
    let w = max_width.saturating_sub(2) as usize;
    if value.is_empty() || w == 0 {
        return 1;
    }
    let display_len = value.len() + 1;
    ((display_len + w - 1) / w).max(1) as u16
}

fn split_at_cursor(s: &str, cursor: usize) -> (String, String, String) {
    let before = s[..cursor].to_string();
    if cursor < s.len() {
        let ch_len = s[cursor..].chars().next().map(|c| c.len_utf8()).unwrap_or(0);
        let ch = s[cursor..cursor + ch_len].to_string();
        let after = s[cursor + ch_len..].to_string();
        (before, ch, after)
    } else {
        (before, " ".to_string(), String::new())
    }
}
