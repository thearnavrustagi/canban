use chrono::NaiveDate;
use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use uuid::Uuid;

use crate::config::Config;
use crate::model::{Board, ColumnKind, Task};
use crate::storage::StorageBackend;
use crate::ui::dialog::{FieldState, InputDialog};

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
    BoardPicker,
    NewBoard,
    Help,
}

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
    pub splash_selected: usize,
    pub tick: u64,
    pub running: bool,
    pub dirty: bool,
    storage: Box<dyn StorageBackend>,
    config: Config,
}

impl App {
    pub fn new(storage: Box<dyn StorageBackend>, config: Config) -> Result<Self> {
        let board = storage.load_board(&config.active_board)?;
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
            splash_selected: 0,
            tick: 0,
            running: true,
            dirty: false,
            storage,
            config,
        })
    }

    pub fn mode_label(&self) -> String {
        match &self.mode {
            Mode::Splash => "MENU".into(),
            Mode::Normal => "NORMAL".into(),
            Mode::Dialog(DialogKind::Help) => "HELP".into(),
            Mode::Dialog(DialogKind::NewTask) => "NEW TASK".into(),
            Mode::Dialog(DialogKind::EditTask(_)) => "EDIT TASK".into(),
            Mode::Dialog(DialogKind::ConfirmDelete(_)) => "CONFIRM".into(),
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
            Mode::Splash => vec![
                ("j/k", "navigate"),
                ("Enter", "select"),
                ("n", "new board"),
                ("q", "quit"),
            ],
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
            Mode::Dialog(DialogKind::BoardPicker) => {
                vec![("j/k", "navigate"), ("Enter", "select"), ("Esc", "cancel")]
            }
            Mode::Dialog(DialogKind::NewTask)
            | Mode::Dialog(DialogKind::EditTask(_))
            | Mode::Dialog(DialogKind::NewBoard) => {
                vec![("Tab", "next field"), ("Enter", "confirm"), ("Esc", "cancel")]
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

    pub fn handle_key(&mut self, key: KeyEvent) {
        match &self.mode {
            Mode::Splash => self.handle_splash(key),
            Mode::Normal => self.handle_normal(key),
            Mode::Dialog(DialogKind::Help) => self.handle_help(key),
            Mode::Dialog(DialogKind::ConfirmDelete(id)) => {
                let id = *id;
                self.handle_confirm_delete(key, id);
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
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.running = false;
            return;
        }
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.splash_selected = (self.splash_selected + 1) % 3;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.splash_selected = if self.splash_selected == 0 {
                    2
                } else {
                    self.splash_selected - 1
                };
            }
            KeyCode::Char('q') => self.running = false,
            KeyCode::Char('n') => self.open_new_board_dialog(),
            KeyCode::Enter => match self.splash_selected {
                0 => self.splash_open_board(),
                1 => self.open_new_board_dialog(),
                2 => self.running = false,
                _ => {}
            },
            _ => {}
        }
    }

    fn splash_open_board(&mut self) {
        let boards = self.storage.list_boards().unwrap_or_default();
        if boards.len() <= 1 {
            self.mode = Mode::Normal;
        } else {
            self.board_list = boards;
            self.board_picker_idx = 0;
            self.mode = Mode::Dialog(DialogKind::BoardPicker);
        }
    }

    fn open_new_board_dialog(&mut self) {
        self.input_dialog = Some(InputDialog {
            title: "New Board".into(),
            fields: vec![FieldState::new("Board name", "")],
            active_field: 0,
        });
        self.mode = Mode::Dialog(DialogKind::NewBoard);
    }

    fn handle_new_board_dialog(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_dialog = None;
                self.mode = Mode::Splash;
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
                }
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
        self.input_dialog = Some(InputDialog {
            title: "New Task".into(),
            fields: vec![
                FieldState::new("Title", ""),
                FieldState::new("Description", ""),
                FieldState::new("Tags (;-separated)", ""),
                FieldState::new("Due date (YYYY-MM-DD)", ""),
            ],
            active_field: 0,
        });
        self.mode = Mode::Dialog(DialogKind::NewTask);
    }

    fn open_edit_task(&mut self) {
        let Some(id) = self.selected_task_id() else {
            return;
        };
        let Some(task) = self.active_board.tasks.iter().find(|t| t.id == id) else {
            return;
        };
        self.input_dialog = Some(InputDialog {
            title: "Edit Task".into(),
            fields: vec![
                FieldState::new("Title", &task.title),
                FieldState::new("Description", &task.description),
                FieldState::new("Tags (;-separated)", &task.tags.join(";")),
                FieldState::new(
                    "Due date (YYYY-MM-DD)",
                    &task
                        .due_date
                        .map(|d| d.format("%Y-%m-%d").to_string())
                        .unwrap_or_default(),
                ),
            ],
            active_field: 0,
        });
        self.mode = Mode::Dialog(DialogKind::EditTask(id));
    }

    fn open_rename_task(&mut self) {
        let Some(id) = self.selected_task_id() else {
            return;
        };
        let Some(task) = self.active_board.tasks.iter().find(|t| t.id == id) else {
            return;
        };
        self.input_dialog = Some(InputDialog {
            title: "Rename Task".into(),
            fields: vec![FieldState::new("Title", &task.title)],
            active_field: 0,
        });
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
        self.input_dialog = Some(InputDialog {
            title: "Set Tags".into(),
            fields: vec![FieldState::new("Tags (;-separated)", &task.tags.join(";"))],
            active_field: 0,
        });
        self.mode = Mode::Dialog(DialogKind::EditTask(id));
    }

    fn open_due_date_dialog(&mut self) {
        let Some(id) = self.selected_task_id() else {
            return;
        };
        let Some(task) = self.active_board.tasks.iter().find(|t| t.id == id) else {
            return;
        };
        self.input_dialog = Some(InputDialog {
            title: "Set Due Date".into(),
            fields: vec![FieldState::new(
                "Due date (YYYY-MM-DD)",
                &task
                    .due_date
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default(),
            )],
            active_field: 0,
        });
        self.mode = Mode::Dialog(DialogKind::EditTask(id));
    }

    fn enter_search(&mut self) {
        self.input_dialog = Some(InputDialog {
            title: "Search".into(),
            fields: vec![FieldState::new("Query", &self.search_query)],
            active_field: 0,
        });
        self.mode = Mode::Search;
    }

    fn enter_command(&mut self) {
        self.input_dialog = Some(InputDialog {
            title: "Command".into(),
            fields: vec![FieldState::new(":", "")],
            active_field: 0,
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
        match key.code {
            KeyCode::Esc => {
                self.input_dialog = None;
                self.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                self.submit_dialog(is_new);
            }
            KeyCode::Tab => {
                if let Some(ref mut dlg) = self.input_dialog {
                    dlg.next_field();
                }
            }
            KeyCode::BackTab => {
                if let Some(ref mut dlg) = self.input_dialog {
                    dlg.prev_field();
                }
            }
            KeyCode::Backspace => {
                if let Some(ref mut dlg) = self.input_dialog {
                    dlg.active_field_mut().backspace();
                }
            }
            KeyCode::Delete => {
                if let Some(ref mut dlg) = self.input_dialog {
                    dlg.active_field_mut().delete();
                }
            }
            KeyCode::Left => {
                if let Some(ref mut dlg) = self.input_dialog {
                    dlg.active_field_mut().move_left();
                }
            }
            KeyCode::Right => {
                if let Some(ref mut dlg) = self.input_dialog {
                    dlg.active_field_mut().move_right();
                }
            }
            KeyCode::Home => {
                if let Some(ref mut dlg) = self.input_dialog {
                    dlg.active_field_mut().move_home();
                }
            }
            KeyCode::End => {
                if let Some(ref mut dlg) = self.input_dialog {
                    dlg.active_field_mut().move_end();
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

    fn submit_dialog(&mut self, is_new: bool) {
        let Some(dlg) = self.input_dialog.take() else {
            self.mode = Mode::Normal;
            return;
        };

        if is_new {
            self.create_task_from_dialog(&dlg);
        } else if let Mode::Dialog(DialogKind::EditTask(id)) = &self.mode {
            self.apply_edit_from_dialog(*id, &dlg);
        }

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
            } else {
                if !val.is_empty() {
                    task.title = val;
                }
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
