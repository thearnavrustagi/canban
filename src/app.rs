use chrono::NaiveDate;
use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use uuid::Uuid;

use crate::config::Config;
use crate::model::{Board, ColumnKind, Task};
use crate::storage::StorageBackend;
use crate::ui::dialog::{DialogVimMode, FieldState, InputDialog};
use crate::vim::{VimAction, VimState};

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Splash,
    Normal,
    Dialog(DialogKind),
    Search,
    Command,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DialogKind {
    NewTask,
    EditTask(Uuid),
    ConfirmDelete(Uuid),
    ConfirmUnsaved { is_new: bool, task_id: Option<Uuid> },
    BoardPicker,
    NewBoard,
    Help,
}

pub const TRANSITION_DURATION: u64 = 10;

pub struct App {
    pub mode: Mode,
    pub active_board: Board,
    pub visible_columns: Vec<ColumnKind>,
    pub selected_column: usize,
    pub selected_tasks: Vec<usize>,
    pub search_query: String,
    pub input_dialog: Option<InputDialog>,
    pub board_list: Vec<String>,
    pub board_picker_idx: usize,
    pub splash_boards: Vec<String>,
    pub splash_board_idx: usize,
    pub tick: u64,
    pub transition_start: Option<u64>,
    pub running: bool,
    pub dirty: bool,
    pub dialog_original_values: Vec<String>,
    pub vim_state: VimState,
    storage: Box<dyn StorageBackend>,
    config: Config,
}

impl App {
    pub fn new(storage: Box<dyn StorageBackend>, config: Config) -> Result<Self> {
        let board = storage.load_board(&config.active_board)?;
        let splash_boards = storage.list_boards().unwrap_or_default();
        let visible_columns = ColumnKind::ALL.to_vec();
        let num_cols = visible_columns.len();
        Ok(Self {
            mode: Mode::Splash,
            active_board: board,
            visible_columns,
            selected_column: 0,
            selected_tasks: vec![0; num_cols],
            search_query: String::new(),
            input_dialog: None,
            board_list: Vec::new(),
            board_picker_idx: 0,
            splash_boards,
            splash_board_idx: 0,
            tick: 0,
            transition_start: None,
            running: true,
            dirty: false,
            dialog_original_values: Vec::new(),
            vim_state: VimState::new(),
            storage,
            config,
        })
    }

    pub fn transition_progress(&self) -> Option<f64> {
        self.transition_start.map(|start| {
            let elapsed = self.tick.saturating_sub(start);
            (elapsed as f64 / TRANSITION_DURATION as f64).min(1.0)
        })
    }

    pub fn is_transitioning(&self) -> bool {
        self.transition_progress()
            .map(|p| p < 1.0)
            .unwrap_or(false)
    }

    pub fn mode_label(&self) -> String {
        match &self.mode {
            Mode::Splash => "MENU".into(),
            Mode::Normal => "NORMAL".into(),
            Mode::Dialog(DialogKind::Help) => "HELP".into(),
            Mode::Dialog(DialogKind::NewTask) | Mode::Dialog(DialogKind::EditTask(_)) => {
                self.input_dialog
                    .as_ref()
                    .map(|d| d.vim_mode_label().to_string())
                    .unwrap_or_else(|| "EDIT".into())
            }
            Mode::Dialog(DialogKind::ConfirmDelete(_)) => "CONFIRM".into(),
            Mode::Dialog(DialogKind::ConfirmUnsaved { .. }) => "CONFIRM".into(),
            Mode::Dialog(DialogKind::BoardPicker) => "BOARDS".into(),
            Mode::Dialog(DialogKind::NewBoard) => "NEW BOARD".into(),
            Mode::Search => "SEARCH".into(),
            Mode::Command => "COMMAND".into(),
        }
    }

    pub fn has_task_under_cursor(&self) -> bool {
        self.selected_task_id().is_some()
    }

    pub fn context_hints(&self) -> Vec<(&str, &str)> {
        match &self.mode {
            Mode::Splash => {
                let mut hints = vec![
                    ("j/k", "navigate"),
                    ("Enter", "open"),
                    ("n", "new board"),
                ];
                if !self.splash_boards.is_empty() {
                    hints.push(("d", "delete board"));
                }
                hints.push(("q", "quit"));
                hints
            }
            Mode::Normal => {
                let mut hints = vec![("h/l", "cols"), ("j/k", "tasks")];
                if self.has_task_under_cursor() {
                    hints.extend_from_slice(&[
                        ("Enter", "edit"),
                        ("d", "delete"),
                        ("Space", "advance"),
                        ("M", "move back"),
                        ("t", "tag"),
                        ("D", "due date"),
                    ]);
                }
                hints.extend_from_slice(&[
                    ("n", "new task"),
                    ("/", "search"),
                    ("b", "boards"),
                    ("?", "help"),
                    ("q", "quit"),
                ]);
                hints
            }
            Mode::Search => vec![("Enter", "apply"), ("Esc", "cancel")],
            Mode::Command => vec![("Enter", "run"), ("Esc", "cancel")],
            Mode::Dialog(DialogKind::Help) => vec![("Esc", "close"), ("?", "close")],
            Mode::Dialog(DialogKind::ConfirmDelete(_)) => {
                vec![("y", "confirm"), ("n", "cancel")]
            }
            Mode::Dialog(DialogKind::ConfirmUnsaved { .. }) => {
                vec![("s", "save"), ("d", "discard"), ("Esc", "go back")]
            }
            Mode::Dialog(DialogKind::BoardPicker) => {
                vec![("j/k", "navigate"), ("Enter", "select"), ("Esc", "cancel")]
            }
            Mode::Dialog(DialogKind::NewTask) | Mode::Dialog(DialogKind::EditTask(_)) => {
                let vim = self
                    .input_dialog
                    .as_ref()
                    .map(|d| d.vim_mode)
                    .unwrap_or(DialogVimMode::Insert);
                match vim {
                    DialogVimMode::Normal => vec![
                        ("i/a", "insert"),
                        ("v", "visual"),
                        ("d/c/y", "operator"),
                        ("Enter", "confirm"),
                        ("Esc", "cancel"),
                    ],
                    DialogVimMode::Insert => vec![
                        ("Esc", "normal mode"),
                        ("Tab", "next field"),
                        ("Enter", "confirm"),
                    ],
                    DialogVimMode::Visual => vec![
                        ("d", "cut"),
                        ("y", "yank"),
                        ("c", "change"),
                        ("Esc", "cancel"),
                    ],
                    DialogVimMode::Replace => vec![
                        ("type", "overwrite"),
                        ("Esc", "normal"),
                    ],
                }
            }
            Mode::Dialog(DialogKind::NewBoard) => {
                vec![("Enter", "confirm"), ("Esc", "cancel")]
            }
        }
    }

    pub fn selected_task_in_column(&self, col_idx: usize) -> usize {
        self.selected_tasks.get(col_idx).copied().unwrap_or(0)
    }

    pub fn filtered_tasks_in_column(&self, col: ColumnKind) -> Vec<&Task> {
        let tasks = self.active_board.tasks_in_column(col);
        if self.search_query.is_empty() {
            tasks
        } else {
            let q = self.search_query.to_lowercase();
            tasks
                .into_iter()
                .filter(|t| {
                    t.title.to_lowercase().contains(&q)
                        || t.tags.iter().any(|tag| tag.to_lowercase().contains(&q))
                        || t.description.to_lowercase().contains(&q)
                })
                .collect()
        }
    }

    fn current_column(&self) -> ColumnKind {
        self.visible_columns[self.selected_column]
    }

    fn task_count_in_current(&self) -> usize {
        self.filtered_tasks_in_column(self.current_column()).len()
    }

    fn clamp_task_cursor(&mut self) {
        let count = self.task_count_in_current();
        let idx = self.selected_column;
        if count == 0 {
            self.selected_tasks[idx] = 0;
        } else if self.selected_tasks[idx] >= count {
            self.selected_tasks[idx] = count - 1;
        }
    }

    fn selected_task_id(&self) -> Option<Uuid> {
        let col = self.current_column();
        let tasks = self.filtered_tasks_in_column(col);
        let idx = self.selected_task_in_column(self.selected_column);
        tasks.get(idx).map(|t| t.id)
    }

    fn collect_tag_suggestions(&self) -> Vec<String> {
        self.active_board.all_tags()
    }

    fn dialog_has_changes(&self) -> bool {
        self.input_dialog.as_ref().map_or(false, |dlg| {
            dlg.fields.iter().enumerate().any(|(i, f)| {
                self.dialog_original_values
                    .get(i)
                    .map_or(true, |orig| *orig != f.value)
            })
        })
    }

    fn try_cancel_dialog(&mut self, is_new: bool) {
        if !self.dialog_has_changes() {
            self.input_dialog = None;
            self.dialog_original_values.clear();
            self.vim_state.reset();
            self.mode = Mode::Normal;
            return;
        }
        let task_id = match &self.mode {
            Mode::Dialog(DialogKind::EditTask(id)) => Some(*id),
            _ => None,
        };
        self.mode = Mode::Dialog(DialogKind::ConfirmUnsaved { is_new, task_id });
    }

    fn make_task_dialog(&mut self, title: &str, fields: Vec<FieldState>) -> InputDialog {
        self.vim_state.reset();
        InputDialog {
            title: title.into(),
            fields,
            active_field: 0,
            vim_mode: DialogVimMode::Normal,
            tag_suggestions: self.collect_tag_suggestions(),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match &self.mode {
            Mode::Splash => self.handle_splash(key),
            Mode::Normal => self.handle_normal(key),
            Mode::Dialog(DialogKind::Help) => self.handle_help(key),
            Mode::Dialog(DialogKind::ConfirmDelete(id)) => {
                let id = *id;
                self.handle_confirm_delete(key, id);
            }
            Mode::Dialog(DialogKind::ConfirmUnsaved { is_new, task_id }) => {
                let is_new = *is_new;
                let task_id = *task_id;
                self.handle_confirm_unsaved(key, is_new, task_id);
            }
            Mode::Dialog(DialogKind::BoardPicker) => self.handle_board_picker(key),
            Mode::Dialog(DialogKind::NewTask) => self.handle_input_dialog(key, true),
            Mode::Dialog(DialogKind::EditTask(_)) => self.handle_input_dialog(key, false),
            Mode::Dialog(DialogKind::NewBoard) => self.handle_new_board_dialog(key),
            Mode::Search => self.handle_search(key),
            Mode::Command => self.handle_command(key),
        }
    }

    fn handle_splash(&mut self, key: KeyEvent) {
        if self.is_transitioning() {
            return;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.running = false;
            return;
        }
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.splash_boards.is_empty() {
                    self.splash_board_idx =
                        (self.splash_board_idx + 1) % self.splash_boards.len();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !self.splash_boards.is_empty() {
                    self.splash_board_idx = if self.splash_board_idx == 0 {
                        self.splash_boards.len() - 1
                    } else {
                        self.splash_board_idx - 1
                    };
                }
            }
            KeyCode::Char('q') => self.running = false,
            KeyCode::Char('n') => self.open_new_board_dialog(),
            KeyCode::Char('d') => self.splash_delete_board(),
            KeyCode::Enter => self.splash_open_selected(),
            _ => {}
        }
    }

    fn splash_open_selected(&mut self) {
        if let Some(name) = self.splash_boards.get(self.splash_board_idx).cloned() {
            self.switch_board(&name);
            self.transition_start = Some(self.tick);
            self.mode = Mode::Normal;
        } else {
            self.open_new_board_dialog();
        }
    }

    fn splash_delete_board(&mut self) {
        if self.splash_boards.is_empty() {
            return;
        }
        let name = self.splash_boards[self.splash_board_idx].clone();
        let _ = self.storage.delete_board(&name);
        self.splash_boards = self.storage.list_boards().unwrap_or_default();
        if self.splash_boards.is_empty() {
            self.splash_board_idx = 0;
        } else if self.splash_board_idx >= self.splash_boards.len() {
            self.splash_board_idx = self.splash_boards.len() - 1;
        }
    }

    fn open_new_board_dialog(&mut self) {
        self.input_dialog = Some(InputDialog {
            title: "New Board".into(),
            fields: vec![FieldState::new("Board name", "")],
            active_field: 0,
            vim_mode: DialogVimMode::Insert,
            tag_suggestions: Vec::new(),
        });
        self.mode = Mode::Dialog(DialogKind::NewBoard);
    }

    fn handle_new_board_dialog(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_dialog = None;
                self.mode = Mode::Splash;
                self.splash_boards = self.storage.list_boards().unwrap_or_default();
            }
            KeyCode::Enter => {
                let name = self
                    .input_dialog
                    .as_ref()
                    .map(|d| d.fields[0].value.trim().to_string())
                    .unwrap_or_default();
                self.input_dialog = None;
                if !name.is_empty() {
                    let board = Board::new(name.clone());
                    let _ = self.storage.save_board(&board);
                    self.switch_board(&name);
                    self.transition_start = Some(self.tick);
                }
                self.splash_boards = self.storage.list_boards().unwrap_or_default();
                self.mode = Mode::Normal;
            }
            KeyCode::Backspace => {
                if let Some(ref mut dlg) = self.input_dialog {
                    dlg.active_field_mut().backspace();
                }
            }
            KeyCode::Char(c) => {
                if let Some(ref mut dlg) = self.input_dialog {
                    dlg.active_field_mut().insert(c);
                }
            }
            _ => {}
        }
    }

    fn handle_normal(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.running = false;
            return;
        }
        match key.code {
            KeyCode::Char('q') => self.running = false,
            KeyCode::Char('h') | KeyCode::Left => self.move_column_left(),
            KeyCode::Char('l') | KeyCode::Right => self.move_column_right(),
            KeyCode::Char('j') | KeyCode::Down => self.move_task_down(),
            KeyCode::Char('k') | KeyCode::Up => self.move_task_up(),
            KeyCode::Char('g') => self.jump_first_task(),
            KeyCode::Char('G') => self.jump_last_task(),
            KeyCode::Tab => self.cycle_column(),
            KeyCode::Char('1') => self.jump_column(0),
            KeyCode::Char('2') => self.jump_column(1),
            KeyCode::Char('3') => self.jump_column(2),
            KeyCode::Char('4') => self.jump_column(3),
            KeyCode::Char('n') | KeyCode::Char('a') => self.open_new_task(),
            KeyCode::Enter | KeyCode::Char('e') => self.open_edit_task(),
            KeyCode::Char('r') => self.open_rename_task(),
            KeyCode::Char('d') => self.open_confirm_delete(),
            KeyCode::Char(' ') | KeyCode::Char('m') => self.advance_task(),
            KeyCode::Char('M') => self.retreat_task(),
            KeyCode::Char('t') => self.open_tag_dialog(),
            KeyCode::Char('D') => self.open_due_date_dialog(),
            KeyCode::Char('/') => self.enter_search(),
            KeyCode::Char('?') => self.mode = Mode::Dialog(DialogKind::Help),
            KeyCode::Char(':') => self.enter_command(),
            KeyCode::Char('b') => self.open_board_picker(),
            _ => {}
        }
    }

    fn move_column_left(&mut self) {
        if self.selected_column > 0 {
            self.selected_column -= 1;
            self.clamp_task_cursor();
        }
    }

    fn move_column_right(&mut self) {
        if self.selected_column + 1 < self.visible_columns.len() {
            self.selected_column += 1;
            self.clamp_task_cursor();
        }
    }

    fn move_task_down(&mut self) {
        let count = self.task_count_in_current();
        let idx = self.selected_column;
        if count > 0 && self.selected_tasks[idx] + 1 < count {
            self.selected_tasks[idx] += 1;
        }
    }

    fn move_task_up(&mut self) {
        let idx = self.selected_column;
        if self.selected_tasks[idx] > 0 {
            self.selected_tasks[idx] -= 1;
        }
    }

    fn jump_first_task(&mut self) {
        self.selected_tasks[self.selected_column] = 0;
    }

    fn jump_last_task(&mut self) {
        let count = self.task_count_in_current();
        let idx = self.selected_column;
        self.selected_tasks[idx] = count.saturating_sub(1);
    }

    fn cycle_column(&mut self) {
        self.selected_column = (self.selected_column + 1) % self.visible_columns.len();
        self.clamp_task_cursor();
    }

    fn jump_column(&mut self, idx: usize) {
        if idx < self.visible_columns.len() {
            self.selected_column = idx;
            self.clamp_task_cursor();
        }
    }

    fn open_new_task(&mut self) {
        let dlg = self.make_task_dialog(
            "New Task",
            vec![
                FieldState::new("Title", ""),
                FieldState::new("Description", "")
                    .with_placeholder("optional"),
                FieldState::new("Tags", "")
                    .with_placeholder("semicolon-separated, e.g. bug;urgent"),
                FieldState::new("Due date", "")
                    .with_placeholder("YYYY-MM-DD"),
            ],
        );
        self.dialog_original_values = dlg.fields.iter().map(|f| f.value.clone()).collect();
        self.input_dialog = Some(dlg);
        self.mode = Mode::Dialog(DialogKind::NewTask);
    }

    fn open_edit_task(&mut self) {
        let Some(id) = self.selected_task_id() else {
            return;
        };
        let Some(task) = self.active_board.tasks.iter().find(|t| t.id == id) else {
            return;
        };
        let dlg = self.make_task_dialog(
            "Edit Task",
            vec![
                FieldState::new("Title", &task.title),
                FieldState::new("Description", &task.description)
                    .with_placeholder("optional"),
                FieldState::new("Tags", &task.tags.join(";"))
                    .with_placeholder("semicolon-separated"),
                FieldState::new("Due date", &task.due_date
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default())
                    .with_placeholder("YYYY-MM-DD"),
            ],
        );
        self.dialog_original_values = dlg.fields.iter().map(|f| f.value.clone()).collect();
        self.input_dialog = Some(dlg);
        self.mode = Mode::Dialog(DialogKind::EditTask(id));
    }

    fn open_rename_task(&mut self) {
        let Some(id) = self.selected_task_id() else {
            return;
        };
        let Some(task) = self.active_board.tasks.iter().find(|t| t.id == id) else {
            return;
        };
        let dlg = self.make_task_dialog(
            "Rename Task",
            vec![FieldState::new("Title", &task.title)],
        );
        self.dialog_original_values = dlg.fields.iter().map(|f| f.value.clone()).collect();
        self.input_dialog = Some(dlg);
        self.mode = Mode::Dialog(DialogKind::EditTask(id));
    }

    fn open_confirm_delete(&mut self) {
        if let Some(id) = self.selected_task_id() {
            self.mode = Mode::Dialog(DialogKind::ConfirmDelete(id));
        }
    }

    fn advance_task(&mut self) {
        let Some(id) = self.selected_task_id() else {
            return;
        };
        if let Some(task) = self.active_board.tasks.iter_mut().find(|t| t.id == id) {
            if let Some(next) = task.column.next() {
                task.move_to(next);
                self.dirty = true;
                self.clamp_task_cursor();
            }
        }
    }

    fn retreat_task(&mut self) {
        let Some(id) = self.selected_task_id() else {
            return;
        };
        if let Some(task) = self.active_board.tasks.iter_mut().find(|t| t.id == id) {
            if let Some(prev) = task.column.prev() {
                task.move_to(prev);
                self.dirty = true;
                self.clamp_task_cursor();
            }
        }
    }

    fn open_tag_dialog(&mut self) {
        let Some(id) = self.selected_task_id() else {
            return;
        };
        let Some(task) = self.active_board.tasks.iter().find(|t| t.id == id) else {
            return;
        };
        let dlg = self.make_task_dialog(
            "Set Tags",
            vec![FieldState::new("Tags", &task.tags.join(";"))
                .with_placeholder("semicolon-separated, e.g. bug;urgent")],
        );
        self.dialog_original_values = dlg.fields.iter().map(|f| f.value.clone()).collect();
        self.input_dialog = Some(dlg);
        self.mode = Mode::Dialog(DialogKind::EditTask(id));
    }

    fn open_due_date_dialog(&mut self) {
        let Some(id) = self.selected_task_id() else {
            return;
        };
        let Some(task) = self.active_board.tasks.iter().find(|t| t.id == id) else {
            return;
        };
        let dlg = self.make_task_dialog(
            "Set Due Date",
            vec![FieldState::new(
                "Due date",
                &task
                    .due_date
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default(),
            )
            .with_placeholder("YYYY-MM-DD")],
        );
        self.dialog_original_values = dlg.fields.iter().map(|f| f.value.clone()).collect();
        self.input_dialog = Some(dlg);
        self.mode = Mode::Dialog(DialogKind::EditTask(id));
    }

    fn enter_search(&mut self) {
        self.input_dialog = Some(InputDialog {
            title: "Search".into(),
            fields: vec![FieldState::new("Query", &self.search_query)],
            active_field: 0,
            vim_mode: DialogVimMode::Insert,
            tag_suggestions: Vec::new(),
        });
        self.mode = Mode::Search;
    }

    fn enter_command(&mut self) {
        self.input_dialog = Some(InputDialog {
            title: "Command".into(),
            fields: vec![FieldState::new(":", "")],
            active_field: 0,
            vim_mode: DialogVimMode::Insert,
            tag_suggestions: Vec::new(),
        });
        self.mode = Mode::Command;
    }

    fn open_board_picker(&mut self) {
        self.board_list = self.storage.list_boards().unwrap_or_default();
        self.board_picker_idx = 0;
        self.mode = Mode::Dialog(DialogKind::BoardPicker);
    }

    fn handle_help(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => self.mode = Mode::Normal,
            _ => {}
        }
    }

    fn handle_confirm_delete(&mut self, key: KeyEvent, id: Uuid) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.active_board.tasks.retain(|t| t.id != id);
                self.dirty = true;
                self.mode = Mode::Normal;
                self.clamp_task_cursor();
            }
            _ => self.mode = Mode::Normal,
        }
    }

    fn handle_confirm_unsaved(&mut self, key: KeyEvent, is_new: bool, task_id: Option<Uuid>) {
        match key.code {
            KeyCode::Char('s') | KeyCode::Char('y') | KeyCode::Enter => {
                self.submit_dialog_with_id(is_new, task_id);
            }
            KeyCode::Char('d') | KeyCode::Char('n') => {
                self.input_dialog = None;
                self.dialog_original_values.clear();
                self.mode = Mode::Normal;
            }
            KeyCode::Esc => {
                if is_new {
                    self.mode = Mode::Dialog(DialogKind::NewTask);
                } else if let Some(id) = task_id {
                    self.mode = Mode::Dialog(DialogKind::EditTask(id));
                } else {
                    self.mode = Mode::Normal;
                }
            }
            _ => {}
        }
    }

    fn handle_board_picker(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => self.mode = Mode::Normal,
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.board_list.is_empty() {
                    self.board_picker_idx =
                        (self.board_picker_idx + 1) % self.board_list.len();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !self.board_list.is_empty() {
                    self.board_picker_idx = if self.board_picker_idx == 0 {
                        self.board_list.len() - 1
                    } else {
                        self.board_picker_idx - 1
                    };
                }
            }
            KeyCode::Enter => {
                if let Some(name) = self.board_list.get(self.board_picker_idx).cloned() {
                    self.switch_board(&name);
                }
                self.mode = Mode::Normal;
            }
            _ => {}
        }
    }

    fn handle_input_dialog(&mut self, key: KeyEvent, is_new: bool) {
        let vim_mode = self
            .input_dialog
            .as_ref()
            .map(|d| d.vim_mode)
            .unwrap_or(DialogVimMode::Insert);

        let result = {
            let Some(ref mut dlg) = self.input_dialog else {
                return;
            };
            let field = dlg.active_field_mut();
            match vim_mode {
                DialogVimMode::Normal => self.vim_state.handle_normal(key, field),
                DialogVimMode::Insert => self.vim_state.handle_insert(key, field),
                DialogVimMode::Visual => self.vim_state.handle_visual(key, field),
                DialogVimMode::Replace => self.vim_state.handle_replace(key, field),
            }
        };

        if let Some(new_mode) = result.new_mode
            && let Some(ref mut dlg) = self.input_dialog
        {
            dlg.vim_mode = new_mode;
        }

        match result.action {
            Some(VimAction::Confirm) => self.submit_dialog(is_new),
            Some(VimAction::Cancel) => self.try_cancel_dialog(is_new),
            Some(VimAction::NextField) => {
                self.vim_state.clear_undo_redo();
                if let Some(ref mut dlg) = self.input_dialog {
                    dlg.next_field();
                }
            }
            Some(VimAction::PrevField) => {
                self.vim_state.clear_undo_redo();
                if let Some(ref mut dlg) = self.input_dialog {
                    dlg.prev_field();
                }
            }
            None => {}
        }
    }

    fn submit_dialog(&mut self, is_new: bool) {
        let edit_id = match &self.mode {
            Mode::Dialog(DialogKind::EditTask(id)) => Some(*id),
            _ => None,
        };
        self.submit_dialog_with_id(is_new, edit_id);
    }

    fn submit_dialog_with_id(&mut self, is_new: bool, edit_id: Option<Uuid>) {
        let Some(dlg) = self.input_dialog.take() else {
            self.mode = Mode::Normal;
            return;
        };

        if is_new {
            self.create_task_from_dialog(&dlg);
        } else if let Some(id) = edit_id {
            self.apply_edit_from_dialog(id, &dlg);
        }

        self.dialog_original_values.clear();
        self.vim_state.reset();
        self.mode = Mode::Normal;
    }

    fn create_task_from_dialog(&mut self, dlg: &InputDialog) {
        let title = field_value(dlg, 0);
        if title.is_empty() {
            return;
        }
        let mut task = Task::new(title, self.current_column());
        task.description = field_value(dlg, 1);
        task.tags = parse_tags(&field_value(dlg, 2));
        task.due_date = parse_date(&field_value(dlg, 3));
        self.active_board.tasks.push(task);
        self.dirty = true;
        self.clamp_task_cursor();
    }

    fn apply_edit_from_dialog(&mut self, id: Uuid, dlg: &InputDialog) {
        let Some(task) = self.active_board.tasks.iter_mut().find(|t| t.id == id) else {
            return;
        };

        if dlg.fields.len() == 1 {
            let val = dlg.fields[0].value.trim().to_string();
            if dlg.title.contains("Tag") {
                task.tags = parse_tags(&val);
            } else if dlg.title.contains("Due") {
                task.due_date = parse_date(&val);
            } else if !val.is_empty() {
                task.title = val;
            }
        } else {
            let title = field_value(dlg, 0);
            if !title.is_empty() {
                task.title = title;
            }
            task.description = field_value(dlg, 1);
            task.tags = parse_tags(&field_value(dlg, 2));
            task.due_date = parse_date(&field_value(dlg, 3));
        }

        task.updated_at = chrono::Utc::now();
        self.dirty = true;
    }

    fn handle_search(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.search_query.clear();
                self.input_dialog = None;
                self.mode = Mode::Normal;
                self.clamp_task_cursor();
            }
            KeyCode::Enter => {
                if let Some(ref dlg) = self.input_dialog {
                    self.search_query = dlg.fields[0].value.clone();
                }
                self.input_dialog = None;
                self.mode = Mode::Normal;
                self.clamp_task_cursor();
            }
            KeyCode::Backspace => {
                if let Some(ref mut dlg) = self.input_dialog {
                    dlg.active_field_mut().backspace();
                    self.search_query = dlg.fields[0].value.clone();
                }
            }
            KeyCode::Char(c) => {
                if let Some(ref mut dlg) = self.input_dialog {
                    dlg.active_field_mut().insert(c);
                    self.search_query = dlg.fields[0].value.clone();
                }
            }
            _ => {}
        }
    }

    fn handle_command(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_dialog = None;
                self.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                let cmd = self
                    .input_dialog
                    .as_ref()
                    .map(|d| d.fields[0].value.trim().to_string())
                    .unwrap_or_default();
                self.input_dialog = None;
                self.mode = Mode::Normal;
                self.execute_command(&cmd);
            }
            KeyCode::Backspace => {
                if let Some(ref mut dlg) = self.input_dialog {
                    dlg.active_field_mut().backspace();
                }
            }
            KeyCode::Char(c) => {
                if let Some(ref mut dlg) = self.input_dialog {
                    dlg.active_field_mut().insert(c);
                }
            }
            _ => {}
        }
    }

    fn execute_command(&mut self, cmd: &str) {
        match cmd {
            "q" | "quit" => self.running = false,
            "w" | "save" => {
                let _ = self.save();
            }
            "wq" => {
                let _ = self.save();
                self.running = false;
            }
            _ => {}
        }
    }

    fn switch_board(&mut self, name: &str) {
        let _ = self.save();
        if let Ok(board) = self.storage.load_board(name) {
            self.active_board = board;
            self.config.active_board = name.to_string();
            let _ = self.config.save();
            self.selected_column = 0;
            self.selected_tasks = vec![0; self.visible_columns.len()];
            self.search_query.clear();
        }
    }

    pub fn save(&mut self) -> Result<()> {
        self.storage.save_board(&self.active_board)?;
        self.dirty = false;
        Ok(())
    }

    pub fn auto_save(&mut self) {
        if self.dirty {
            let _ = self.save();
        }
    }
}

fn field_value(dlg: &InputDialog, idx: usize) -> String {
    dlg.fields
        .get(idx)
        .map(|f| f.value.trim().to_string())
        .unwrap_or_default()
}

fn parse_tags(s: &str) -> Vec<String> {
    if s.is_empty() {
        Vec::new()
    } else {
        s.split(';')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect()
    }
}

fn parse_date(s: &str) -> Option<NaiveDate> {
    if s.is_empty() {
        None
    } else {
        NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok()
    }
}
