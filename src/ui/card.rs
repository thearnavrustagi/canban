use ratatui::text::{Line, Span};

use crate::model::Task;
use crate::ui::theme;

pub fn render_card(task: &Task, selected: bool, width: u16) -> Vec<Line<'static>> {
    let style = theme::card_style(selected);
    let inner_w = width.saturating_sub(2) as usize;

    let prefix = if selected { "▸ " } else { "  " };
    let title = truncate(&task.title, inner_w.saturating_sub(2));
    let mut lines = vec![Line::from(vec![
        Span::styled(
            prefix,
            if selected {
                theme::card_style(true)
            } else {
                ratatui::style::Style::default().fg(theme::FG_MUTED)
            },
        ),
        Span::styled(title, style),
    ])];

    if !task.tags.is_empty() {
        let mut spans: Vec<Span> = vec![Span::raw("  ")];
        for (i, tag) in task.tags.iter().enumerate() {
            let style = theme::tag_style(tag);
            spans.push(Span::styled(format!(" #{tag} "), style));
            if i + 1 < task.tags.len() {
                spans.push(Span::raw(" "));
            }
        }
        let tag_line = Line::from(truncate_spans(spans, inner_w));
        lines.push(tag_line);
    }

    if let Some(due) = task.due_date {
        let s = format!("  {} {}", "◆", due.format("%Y-%m-%d"));
        let st = if task.is_overdue() {
            theme::overdue_style()
        } else {
            theme::due_style()
        };
        lines.push(Line::from(Span::styled(truncate(&s, inner_w), st)));
    }

    let secs = task.effective_doing_secs();
    if secs > 0 {
        let display = format!("  ⏱ {}", format_duration(secs));
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
        format!("{h}h {m}m")
    } else {
        format!("{m}m")
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

fn truncate_spans(spans: Vec<Span<'static>>, max_width: usize) -> Vec<Span<'static>> {
    let mut result = Vec::new();
    let mut used = 0;
    for span in spans {
        let len = span.content.len();
        if used + len <= max_width {
            result.push(span);
            used += len;
        } else {
            let remaining = max_width.saturating_sub(used);
            if remaining > 3 {
                let truncated = format!("{}…", &span.content[..remaining - 1]);
                result.push(Span::styled(truncated, span.style));
            }
            break;
        }
    }
    result
}
