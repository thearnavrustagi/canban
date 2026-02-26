use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
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
    render_footer(f, footer, &app.mode_label());
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let title = format!(" canban │ Board: {} ", app.active_board.name);
    let line = Line::from(vec![
        Span::styled(title, theme::header_style()),
        Span::raw("  "),
        Span::styled("[?] Help", Style::default().fg(Color::DarkGray)),
    ]);
    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(Color::Rgb(30, 30, 40))),
        area,
    );
}

fn render_footer(f: &mut Frame, area: Rect, mode_label: &str) {
    let hints = " h/l: cols │ j/k: tasks │ n: new │ Space: advance │ ?: help ";
    let line = Line::from(vec![
        Span::styled(
            format!(" [{mode_label}] "),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(hints, theme::footer_style()),
    ]);
    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(Color::Rgb(30, 30, 40))),
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
    let title = format!(" {} ({}) ", col, count);
    let border_style = theme::column_style(col, is_selected);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let tasks = app.filtered_tasks_in_column(col);
    if tasks.is_empty() {
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
            Style::default().bg(Color::Rgb(50, 50, 70))
        } else {
            Style::default()
        };

        let para = Paragraph::new(lines).style(style);
        f.render_widget(para, card_areas[i]);
    }

    if tasks.len() > visible_count {
        let mut sb_state = ScrollbarState::new(tasks.len())
            .position(scroll_offset)
            .viewport_content_length(visible_count);
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area,
            &mut sb_state,
        );
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
