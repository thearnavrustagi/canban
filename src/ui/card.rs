use ratatui::text::{Line, Span};

use crate::model::Task;
use crate::ui::theme;

pub fn render_card(task: &Task, selected: bool, width: u16) -> Vec<Line<'static>> {
    let style = theme::card_style(selected);
    let inner_w = width.saturating_sub(2) as usize;

    let title = truncate(&task.title, inner_w);
    let mut lines = vec![Line::from(Span::styled(title, style))];

    if !task.tags.is_empty() {
        let tag_str = task
            .tags
            .iter()
            .map(|t| format!("#{t}"))
            .collect::<Vec<_>>()
            .join(" ");
        lines.push(Line::from(Span::styled(
            truncate(&tag_str, inner_w),
            theme::tag_style(),
        )));
    }

    if let Some(due) = task.due_date {
        let s = format!("Due: {}", due.format("%Y-%m-%d"));
        let st = if task.is_overdue() {
            theme::overdue_style()
        } else {
            theme::due_style()
        };
        lines.push(Line::from(Span::styled(truncate(&s, inner_w), st)));
    }

    let secs = task.effective_doing_secs();
    if secs > 0 {
        let display = format_duration(secs);
        lines.push(Line::from(Span::styled(
            truncate(&display, inner_w),
            theme::due_style(),
        )));
    }

    lines
}

fn format_duration(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    if h > 0 {
        format!("⏱ {h}h {m}m")
    } else {
        format!("⏱ {m}m")
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else if max > 3 {
        format!("{}...", &s[..max - 3])
    } else {
        s[..max].to_string()
    }
}
