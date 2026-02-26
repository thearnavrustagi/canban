use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::Frame;

use crate::app::App;
use crate::model::ColumnKind;
use crate::ui::{card, theme};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);

    render_header(f, app, header);
    render_columns(f, app, body);
    render_footer(f, footer, &app.mode_label(), &app.context_hints());
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let board_name = &app.active_board.name;
    let search_indicator = if !app.search_query.is_empty() {
        format!("  /{}", app.search_query)
    } else {
        String::new()
    };

    let line = Line::from(vec![
        Span::styled(" canban", Style::default().fg(theme::ACCENT_CYAN).add_modifier(Modifier::BOLD)),
        Span::styled(" │ ", Style::default().fg(theme::FG_MUTED)),
        Span::styled(board_name.to_string(), theme::header_style()),
        Span::styled(search_indicator, Style::default().fg(theme::ACCENT_YELLOW)),
        Span::raw("  "),
        Span::styled("[?] Help", Style::default().fg(theme::FG_MUTED)),
    ]);
    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(theme::BG_MID)),
        area,
    );
}

fn render_footer(f: &mut Frame, area: Rect, mode_label: &str, hints: &[(&str, &str)]) {
    let mode_style = if mode_label == "INSERT" {
        theme::mode_badge_insert()
    } else {
        theme::mode_badge_normal()
    };

    let mut spans = vec![Span::styled(
        format!(" {mode_label} "),
        mode_style,
    )];

    for (i, (key, desc)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(
                " │ ",
                Style::default().fg(theme::FG_MUTED),
            ));
        } else {
            spans.push(Span::raw(" "));
        }
        spans.push(Span::styled(
            (*key).to_string(),
            Style::default()
                .fg(theme::ACCENT_YELLOW)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(": {desc}"),
            Style::default().fg(theme::FG_DIM),
        ));
    }

    let line = Line::from(spans);
    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(theme::BG_DARK)),
        area,
    );
}

fn render_columns(f: &mut Frame, app: &App, area: Rect) {
    let visible = &app.visible_columns;
    if visible.is_empty() {
        return;
    }

    let constraints: Vec<Constraint> = visible
        .iter()
        .map(|_| Constraint::Ratio(1, visible.len() as u32))
        .collect();

    let cols = Layout::horizontal(constraints).split(area);

    for (col_idx, &col_kind) in visible.iter().enumerate() {
        let is_selected_col = col_idx == app.selected_column;
        render_single_column(f, app, cols[col_idx], col_kind, col_idx, is_selected_col);
    }
}

fn render_single_column(
    f: &mut Frame,
    app: &App,
    area: Rect,
    col: ColumnKind,
    col_idx: usize,
    is_selected: bool,
) {
    let count = app.active_board.column_count(col);
    let icon = theme::column_icon(col);
    let title = format!(" {icon} {} ({count}) ", col);
    let border_style = theme::column_style(col, is_selected);

    let block = Block::default()
        .title(Span::styled(title, border_style))
        .borders(Borders::ALL)
        .border_style(if is_selected {
            border_style
        } else {
            Style::default().fg(theme::FG_MUTED)
        });

    let inner = block.inner(area);
    f.render_widget(block, area);

    let tasks = app.filtered_tasks_in_column(col);
    if tasks.is_empty() {
        render_empty_placeholder(f, inner, col, is_selected);
        return;
    }

    let selected_task_idx = app.selected_task_in_column(col_idx);
    let card_height = 4u16;
    let visible_count = (inner.height / card_height).max(1) as usize;
    let scroll_offset = compute_scroll(selected_task_idx, tasks.len(), visible_count);

    let visible_tasks = &tasks[scroll_offset..tasks.len().min(scroll_offset + visible_count)];

    let card_constraints: Vec<Constraint> = visible_tasks
        .iter()
        .map(|_| Constraint::Length(card_height))
        .chain(std::iter::once(Constraint::Min(0)))
        .collect();

    let card_areas = Layout::vertical(card_constraints).split(inner);

    for (i, task) in visible_tasks.iter().enumerate() {
        let abs_idx = scroll_offset + i;
        let is_task_selected = is_selected && abs_idx == selected_task_idx;
        let lines = card::render_card(task, is_task_selected, inner.width);

        let style = if is_task_selected {
            Style::default().bg(theme::BG_RAISED)
        } else {
            Style::default()
        };

        let para = Paragraph::new(lines).style(style);
        f.render_widget(para, card_areas[i]);
    }

    if tasks.len() > visible_count {
        let col_color = theme::column_color(col);
        let mut sb_state = ScrollbarState::new(tasks.len())
            .position(scroll_offset)
            .viewport_content_length(visible_count);
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .thumb_style(Style::default().fg(col_color))
                .track_style(Style::default().fg(theme::FG_MUTED)),
            area,
            &mut sb_state,
        );
    }
}

fn render_empty_placeholder(f: &mut Frame, inner: Rect, col: ColumnKind, is_selected: bool) {
    if inner.height < 3 || inner.width < 10 {
        return;
    }

    if is_selected {
        let box_w = 22u16.min(inner.width);
        let box_h = 5u16.min(inner.height);
        let x = inner.x + inner.width.saturating_sub(box_w) / 2;
        let y = inner.y + inner.height.saturating_sub(box_h) / 2;
        let placeholder_area = Rect::new(x, y, box_w, box_h);

        let col_color = theme::column_color(col);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(
                Style::default()
                    .fg(col_color)
                    .add_modifier(Modifier::DIM),
            );
        let block_inner = block.inner(placeholder_area);
        f.render_widget(block, placeholder_area);

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "No tasks",
                Style::default().fg(theme::FG_DIM),
            )),
            Line::from(Span::styled(
                "n: add task",
                Style::default()
                    .fg(col_color)
                    .add_modifier(Modifier::BOLD),
            )),
        ];
        let para = Paragraph::new(lines).alignment(Alignment::Center);
        f.render_widget(para, block_inner);
    } else {
        let y = inner.y + inner.height / 2;
        let text_area = Rect::new(inner.x, y, inner.width, 1);
        let para = Paragraph::new(Line::from(Span::styled(
            "─",
            Style::default()
                .fg(theme::FG_MUTED)
                .add_modifier(Modifier::DIM),
        )))
        .alignment(Alignment::Center);
        f.render_widget(para, text_area);
    }
}

fn compute_scroll(selected: usize, total: usize, visible: usize) -> usize {
    if total <= visible {
        return 0;
    }
    if selected < visible / 2 {
        return 0;
    }
    let max_offset = total.saturating_sub(visible);
    (selected.saturating_sub(visible / 2)).min(max_offset)
}
