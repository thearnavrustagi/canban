pub mod board;
pub mod card;
pub mod dialog;
pub mod help;
pub mod theme;

use ratatui::Frame;

use crate::app::{App, DialogKind, Mode};

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();
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
