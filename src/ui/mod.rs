pub mod board;
pub mod card;
pub mod dialog;
pub mod help;
pub mod splash;
pub mod theme;

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

use crate::app::{App, DialogKind, Mode};

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    if let Some(progress) = app.transition_progress() {
        if progress < 1.0 {
            render_transition(f, app, area, progress);
            return;
        }
    }

    match &app.mode {
        Mode::Splash => {
            splash::render(f, app);
            return;
        }
        Mode::Dialog(DialogKind::NewBoard) => {
            splash::render(f, app);
            if let Some(ref dlg) = app.input_dialog {
                dialog::render_input_dialog(f, area, dlg);
            }
            return;
        }
        _ => {}
    }

    board::render(f, app, area);

    match &app.mode {
        Mode::Dialog(DialogKind::Help) => help::render(f, area),
        Mode::Dialog(DialogKind::NewTask) | Mode::Dialog(DialogKind::EditTask(_)) => {
            if let Some(ref dlg) = app.input_dialog {
                dialog::render_input_dialog(f, area, dlg);
            }
        }
        Mode::Dialog(DialogKind::ConfirmDelete(_)) => {
            dialog::render_confirm(f, area, "Delete this task?");
        }
        Mode::Dialog(DialogKind::ConfirmUnsaved { .. }) => {
            dialog::render_unsaved_confirm(f, area);
        }
        Mode::Dialog(DialogKind::BoardPicker) => {
            dialog::render_board_picker(f, area, &app.board_list, app.board_picker_idx);
        }
        Mode::Search => {
            if let Some(ref dlg) = app.input_dialog {
                dialog::render_input_dialog(f, area, dlg);
            }
        }
        Mode::Command => {
            if let Some(ref dlg) = app.input_dialog {
                dialog::render_input_dialog(f, area, dlg);
            }
        }
        _ => {}
    }
}

fn render_transition(f: &mut Frame, app: &App, area: Rect, progress: f64) {
    let eased = ease_out_cubic(progress);
    let revealed_rows = ((area.height as f64) * eased) as u16;

    let blank_rows = area.height.saturating_sub(revealed_rows);
    if blank_rows > 0 {
        let blank_area = Rect::new(area.x, area.y, area.width, blank_rows);
        f.render_widget(Clear, blank_area);

        for row in 0..blank_rows {
            let y = area.y + row;
            let fade = 1.0 - (row as f64 / blank_rows.max(1) as f64);
            let g = (fade * 30.0) as u8;
            let line_area = Rect::new(area.x, y, area.width, 1);
            f.render_widget(
                Paragraph::new("").style(Style::default().bg(Color::Rgb(g, g, g + 10))),
                line_area,
            );
        }
    }

    if revealed_rows > 0 {
        let board_area = Rect::new(area.x, area.y + blank_rows, area.width, revealed_rows);
        board::render(f, app, board_area);
    }
}

fn ease_out_cubic(t: f64) -> f64 {
    1.0 - (1.0 - t).powi(3)
}
