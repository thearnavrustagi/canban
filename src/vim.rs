use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::ui::dialog::{DialogVimMode, FieldState};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Delete,
    Change,
    Yank,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindDirection {
    Forward,
    Backward,
    TillForward,
    TillBackward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextObject {
    InnerWord,
    AWord,
    InnerQuoted(char),
    AQuoted(char),
    InnerParen,
    AParen,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VimAction {
    Confirm,
    Cancel,
    NextField,
    PrevField,
}

pub struct VimResult {
    pub new_mode: Option<DialogVimMode>,
    pub action: Option<VimAction>,
}

impl VimResult {
    fn handled() -> Self {
        Self { new_mode: None, action: None }
    }
    fn mode(m: DialogVimMode) -> Self {
        Self { new_mode: Some(m), action: None }
    }
    fn action(a: VimAction) -> Self {
        Self { new_mode: None, action: Some(a) }
    }
}

#[derive(Debug, Clone)]
pub struct RecordedEdit {
    pub count: Option<usize>,
    pub keys: Vec<KeyEvent>,
}

// ---------------------------------------------------------------------------
// VimState
// ---------------------------------------------------------------------------

pub struct VimState {
    pub register: String,
    pub count_accum: Option<usize>,
    op_count: Option<usize>,
    pending_op: Option<Operator>,
    awaiting_find: Option<FindDirection>,
    awaiting_text_obj: Option<bool>,
    awaiting_replace: bool,
    pub last_find: Option<(FindDirection, char)>,
    undo_stack: Vec<(String, usize)>,
    redo_stack: Vec<(String, usize)>,
    last_edit: Option<RecordedEdit>,
    recording: Option<Vec<KeyEvent>>,
    recording_count: Option<usize>,
    pub visual_anchor: Option<usize>,
    replace_backup: Vec<char>,
    replaying: bool,
}

impl VimState {
    pub fn new() -> Self {
        Self {
            register: String::new(),
            count_accum: None,
            op_count: None,
            pending_op: None,
            awaiting_find: None,
            awaiting_text_obj: None,
            awaiting_replace: false,
            last_find: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            last_edit: None,
            recording: None,
            recording_count: None,
            visual_anchor: None,
            replace_backup: Vec::new(),
            replaying: false,
        }
    }

    pub fn reset(&mut self) {
        self.count_accum = None;
        self.op_count = None;
        self.pending_op = None;
        self.awaiting_find = None;
        self.awaiting_text_obj = None;
        self.awaiting_replace = false;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.recording = None;
        self.recording_count = None;
        self.visual_anchor = None;
        self.replace_backup.clear();
        self.replaying = false;
    }

    pub fn clear_undo_redo(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    fn clear_pending(&mut self) {
        self.count_accum = None;
        self.op_count = None;
        self.pending_op = None;
        self.awaiting_find = None;
        self.awaiting_text_obj = None;
        self.awaiting_replace = false;
    }

    fn effective_count(&mut self) -> usize {
        let c1 = self.op_count.take().unwrap_or(1);
        let c2 = self.count_accum.take().unwrap_or(1);
        c1 * c2
    }

    fn push_undo(&mut self, field: &FieldState) {
        if self.replaying {
            return;
        }
        self.undo_stack.push((field.value.clone(), field.cursor));
        self.redo_stack.clear();
    }

    fn undo(&mut self, field: &mut FieldState) {
        if let Some((val, cur)) = self.undo_stack.pop() {
            self.redo_stack.push((field.value.clone(), field.cursor));
            field.value = val;
            field.cursor = cur;
        }
    }

    fn redo(&mut self, field: &mut FieldState) {
        if let Some((val, cur)) = self.redo_stack.pop() {
            self.undo_stack.push((field.value.clone(), field.cursor));
            field.value = val;
            field.cursor = cur;
        }
    }

    fn start_recording(&mut self, key: KeyEvent) {
        if self.replaying {
            return;
        }
        if self.recording.is_none() {
            self.recording_count = self.count_accum;
            self.recording = Some(vec![key]);
        } else if let Some(ref mut rec) = self.recording {
            rec.push(key);
        }
    }

    fn record_key_if_active(&mut self, key: KeyEvent) {
        if !self.replaying {
            if let Some(ref mut rec) = self.recording {
                rec.push(key);
            }
        }
    }

    fn finish_recording(&mut self) {
        if self.replaying {
            return;
        }
        if let Some(keys) = self.recording.take() {
            if !keys.is_empty() {
                self.last_edit = Some(RecordedEdit {
                    count: self.recording_count.take(),
                    keys,
                });
            }
        }
        self.recording_count = None;
    }

    fn cancel_recording(&mut self) {
        if !self.replaying {
            self.recording = None;
            self.recording_count = None;
        }
    }

    // -----------------------------------------------------------------------
    // Normal mode
    // -----------------------------------------------------------------------

    pub fn handle_normal(&mut self, key: KeyEvent, field: &mut FieldState) -> VimResult {
        self.record_key_if_active(key);

        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return self.handle_ctrl_normal(key, field);
        }

        if let Some(r) = self.try_awaiting_find(key, field) {
            return r;
        }
        if let Some(r) = self.try_awaiting_replace(key, field) {
            return r;
        }
        if let Some(r) = self.try_awaiting_text_obj(key, field) {
            return r;
        }

        if let KeyCode::Char(ch) = key.code {
            if ch.is_ascii_digit() && (ch != '0' || self.count_accum.is_some()) {
                let d = ch.to_digit(10).unwrap() as usize;
                self.count_accum = Some(self.count_accum.unwrap_or(0) * 10 + d);
                return VimResult::handled();
            }
        }

        if let Some(op) = self.pending_op {
            return self.handle_operator_pending(key, field, op);
        }

        self.handle_normal_command(key, field)
    }

    fn handle_ctrl_normal(&mut self, key: KeyEvent, field: &mut FieldState) -> VimResult {
        match key.code {
            KeyCode::Char('c') => {
                self.clear_pending();
                self.cancel_recording();
                VimResult::action(VimAction::Cancel)
            }
            KeyCode::Char('r') => {
                self.redo(field);
                VimResult::handled()
            }
            _ => VimResult::handled(),
        }
    }

    // -- awaiting helpers ---------------------------------------------------

    fn try_awaiting_find(
        &mut self,
        key: KeyEvent,
        field: &mut FieldState,
    ) -> Option<VimResult> {
        let dir = self.awaiting_find.take()?;
        if let KeyCode::Char(ch) = key.code {
            self.last_find = Some((dir, ch));
            let count = self.effective_count();
            if let Some(op) = self.pending_op.take() {
                if let Some(target) = find_char_repeated(&field.value, field.cursor, dir, ch, count) {
                    let (lo, hi) = find_operator_range(&field.value, field.cursor, target);
                    self.push_undo(field);
                    let m = apply_operator(op, field, lo, hi, self);
                    self.finish_recording();
                    if let Some(m) = m {
                        return Some(VimResult::mode(m));
                    }
                }
                self.clear_pending();
            } else if let Some(target) =
                find_char_repeated(&field.value, field.cursor, dir, ch, count)
            {
                field.cursor = target;
            }
        } else {
            self.pending_op = None;
            self.cancel_recording();
            self.clear_pending();
        }
        Some(VimResult::handled())
    }

    fn try_awaiting_replace(
        &mut self,
        key: KeyEvent,
        field: &mut FieldState,
    ) -> Option<VimResult> {
        if !self.awaiting_replace {
            return None;
        }
        self.awaiting_replace = false;
        if let KeyCode::Char(ch) = key.code {
            let count = self.effective_count();
            self.push_undo(field);
            let start = field.cursor;
            for _ in 0..count {
                if field.cursor < field.value.len() {
                    replace_char_at(field, field.cursor, ch);
                    let clen = ch.len_utf8();
                    if field.cursor + clen < field.value.len() {
                        field.cursor += clen;
                    }
                }
            }
            if count > 1 && field.cursor > start {
                field.cursor = prev_char_boundary(&field.value, field.cursor);
            }
            self.finish_recording();
        } else {
            self.cancel_recording();
        }
        Some(VimResult::handled())
    }

    fn try_awaiting_text_obj(
        &mut self,
        key: KeyEvent,
        field: &mut FieldState,
    ) -> Option<VimResult> {
        let inner = self.awaiting_text_obj.take()?;
        let op = self.pending_op.take()?;
        let obj = match key.code {
            KeyCode::Char('w') => {
                Some(if inner { TextObject::InnerWord } else { TextObject::AWord })
            }
            KeyCode::Char('"') => Some(if inner {
                TextObject::InnerQuoted('"')
            } else {
                TextObject::AQuoted('"')
            }),
            KeyCode::Char('\'') => Some(if inner {
                TextObject::InnerQuoted('\'')
            } else {
                TextObject::AQuoted('\'')
            }),
            KeyCode::Char('(') | KeyCode::Char(')') | KeyCode::Char('b') => {
                Some(if inner { TextObject::InnerParen } else { TextObject::AParen })
            }
            _ => None,
        };
        if let Some(obj) = obj {
            if let Some((lo, hi)) = text_object_range(&field.value, field.cursor, obj) {
                self.push_undo(field);
                let m = apply_operator(op, field, lo, hi, self);
                self.finish_recording();
                if let Some(m) = m {
                    return Some(VimResult::mode(m));
                }
                return Some(VimResult::handled());
            }
        }
        self.clear_pending();
        self.cancel_recording();
        Some(VimResult::handled())
    }

    // -- operator-pending ---------------------------------------------------

    fn handle_operator_pending(
        &mut self,
        key: KeyEvent,
        field: &mut FieldState,
        op: Operator,
    ) -> VimResult {
        let is_double = matches!(
            (op, key.code),
            (Operator::Delete, KeyCode::Char('d'))
                | (Operator::Change, KeyCode::Char('c'))
                | (Operator::Yank, KeyCode::Char('y'))
        );

        if is_double {
            self.pending_op = None;
            let count = self.effective_count();
            let _ = count; // count is meaningless for single-line dd/cc/yy
            self.push_undo(field);
            self.register = field.value.clone();
            if op == Operator::Yank {
                field.cursor = 0;
                self.finish_recording();
                return VimResult::handled();
            }
            field.clear();
            if op == Operator::Change {
                return VimResult::mode(DialogVimMode::Insert);
            }
            self.finish_recording();
            return VimResult::handled();
        }

        match key.code {
            KeyCode::Char('i') => {
                self.awaiting_text_obj = Some(true);
                self.pending_op = Some(op);
                VimResult::handled()
            }
            KeyCode::Char('a') => {
                self.awaiting_text_obj = Some(false);
                self.pending_op = Some(op);
                VimResult::handled()
            }
            KeyCode::Char('f') => {
                self.awaiting_find = Some(FindDirection::Forward);
                self.pending_op = Some(op);
                VimResult::handled()
            }
            KeyCode::Char('F') => {
                self.awaiting_find = Some(FindDirection::Backward);
                self.pending_op = Some(op);
                VimResult::handled()
            }
            KeyCode::Char('t') => {
                self.awaiting_find = Some(FindDirection::TillForward);
                self.pending_op = Some(op);
                VimResult::handled()
            }
            KeyCode::Char('T') => {
                self.awaiting_find = Some(FindDirection::TillBackward);
                self.pending_op = Some(op);
                VimResult::handled()
            }
            KeyCode::Char(';') => {
                self.pending_op = None;
                let count = self.effective_count();
                if let Some((dir, ch)) = self.last_find {
                    if let Some(target) =
                        find_char_repeated(&field.value, field.cursor, dir, ch, count)
                    {
                        let (lo, hi) = find_operator_range(&field.value, field.cursor, target);
                        self.push_undo(field);
                        let m = apply_operator(op, field, lo, hi, self);
                        self.finish_recording();
                        if let Some(m) = m {
                            return VimResult::mode(m);
                        }
                    }
                }
                VimResult::handled()
            }
            KeyCode::Char(',') => {
                self.pending_op = None;
                let count = self.effective_count();
                if let Some((dir, ch)) = self.last_find {
                    let rev = reverse_find_dir(dir);
                    if let Some(target) =
                        find_char_repeated(&field.value, field.cursor, rev, ch, count)
                    {
                        let (lo, hi) = find_operator_range(&field.value, field.cursor, target);
                        self.push_undo(field);
                        let m = apply_operator(op, field, lo, hi, self);
                        self.finish_recording();
                        if let Some(m) = m {
                            return VimResult::mode(m);
                        }
                    }
                }
                VimResult::handled()
            }
            KeyCode::Esc => {
                self.pending_op = None;
                self.cancel_recording();
                self.clear_pending();
                VimResult::handled()
            }
            _ => {
                self.pending_op = None;
                let count = self.effective_count();
                if let Some((lo, hi)) = resolve_motion_range(field, key.code, count) {
                    self.push_undo(field);
                    let m = apply_operator(op, field, lo, hi, self);
                    self.finish_recording();
                    if let Some(m) = m {
                        return VimResult::mode(m);
                    }
                } else {
                    self.cancel_recording();
                }
                self.clear_pending();
                VimResult::handled()
            }
        }
    }

    // -- normal commands (no pending op) ------------------------------------

    fn handle_normal_command(
        &mut self,
        key: KeyEvent,
        field: &mut FieldState,
    ) -> VimResult {
        match key.code {
            // --- Operators ---
            KeyCode::Char('d') => {
                let c = self.effective_count();
                self.start_recording(key);
                self.op_count = if c > 1 { Some(c) } else { None };
                self.pending_op = Some(Operator::Delete);
                VimResult::handled()
            }
            KeyCode::Char('c') => {
                let c = self.effective_count();
                self.start_recording(key);
                self.op_count = if c > 1 { Some(c) } else { None };
                self.pending_op = Some(Operator::Change);
                VimResult::handled()
            }
            KeyCode::Char('y') => {
                let c = self.effective_count();
                self.start_recording(key);
                self.op_count = if c > 1 { Some(c) } else { None };
                self.pending_op = Some(Operator::Yank);
                VimResult::handled()
            }

            // --- Mode switches ---
            KeyCode::Char('i') => {
                self.start_recording(key);
                VimResult::mode(DialogVimMode::Insert)
            }
            KeyCode::Char('a') => {
                self.start_recording(key);
                if field.cursor < field.value.len() {
                    field.move_right();
                }
                VimResult::mode(DialogVimMode::Insert)
            }
            KeyCode::Char('A') => {
                self.start_recording(key);
                field.move_end();
                VimResult::mode(DialogVimMode::Insert)
            }
            KeyCode::Char('I') => {
                self.start_recording(key);
                field.move_to_first_nonblank();
                VimResult::mode(DialogVimMode::Insert)
            }
            KeyCode::Char('v') => {
                self.visual_anchor = Some(field.cursor);
                VimResult::mode(DialogVimMode::Visual)
            }
            KeyCode::Char('R') => {
                self.start_recording(key);
                self.replace_backup.clear();
                VimResult::mode(DialogVimMode::Replace)
            }

            // --- Immediate edits ---
            KeyCode::Char('x') | KeyCode::Delete => {
                let count = self.effective_count();
                if !field.value.is_empty() && field.cursor < field.value.len() {
                    self.start_recording(key);
                    self.push_undo(field);
                    let mut deleted = String::new();
                    for _ in 0..count {
                        if field.cursor < field.value.len() {
                            let cl = next_char_len(&field.value, field.cursor);
                            deleted.push_str(&field.value[field.cursor..field.cursor + cl]);
                            field.value.drain(field.cursor..field.cursor + cl);
                        }
                    }
                    self.register = deleted;
                    clamp_cursor_normal(field);
                    self.finish_recording();
                }
                VimResult::handled()
            }
            KeyCode::Char('X') => {
                let count = self.effective_count();
                if field.cursor > 0 {
                    self.start_recording(key);
                    self.push_undo(field);
                    let mut deleted = String::new();
                    for _ in 0..count {
                        if field.cursor > 0 {
                            let prev = prev_char_boundary(&field.value, field.cursor);
                            deleted.insert_str(0, &field.value[prev..field.cursor]);
                            field.value.drain(prev..field.cursor);
                            field.cursor = prev;
                        }
                    }
                    self.register = deleted;
                    self.finish_recording();
                }
                VimResult::handled()
            }
            KeyCode::Char('s') => {
                let count = self.effective_count();
                self.start_recording(key);
                self.push_undo(field);
                let mut deleted = String::new();
                for _ in 0..count {
                    if field.cursor < field.value.len() {
                        let cl = next_char_len(&field.value, field.cursor);
                        deleted.push_str(&field.value[field.cursor..field.cursor + cl]);
                        field.value.drain(field.cursor..field.cursor + cl);
                    }
                }
                self.register = deleted;
                VimResult::mode(DialogVimMode::Insert)
            }
            KeyCode::Char('S') => {
                self.effective_count(); // consume
                self.start_recording(key);
                self.push_undo(field);
                self.register = field.value.clone();
                field.clear();
                VimResult::mode(DialogVimMode::Insert)
            }
            KeyCode::Char('C') => {
                self.effective_count();
                self.start_recording(key);
                self.push_undo(field);
                let pos = field.cursor;
                self.register = field.value[pos..].to_string();
                field.value.truncate(pos);
                VimResult::mode(DialogVimMode::Insert)
            }
            KeyCode::Char('D') => {
                self.effective_count();
                if field.cursor < field.value.len() {
                    self.start_recording(key);
                    self.push_undo(field);
                    let pos = field.cursor;
                    self.register = field.value[pos..].to_string();
                    field.value.truncate(pos);
                    clamp_cursor_normal(field);
                    self.finish_recording();
                }
                VimResult::handled()
            }
            KeyCode::Char('r') => {
                // count is consumed later in try_awaiting_replace
                self.start_recording(key);
                self.awaiting_replace = true;
                VimResult::handled()
            }
            KeyCode::Char('p') => {
                let count = self.effective_count();
                if !self.register.is_empty() {
                    self.start_recording(key);
                    self.push_undo(field);
                    let insert_pos = if field.value.is_empty() {
                        0
                    } else if field.cursor < field.value.len() {
                        field.cursor + next_char_len(&field.value, field.cursor)
                    } else {
                        field.value.len()
                    };
                    let text = self.register.repeat(count);
                    field.value.insert_str(insert_pos, &text);
                    let end = insert_pos + text.len();
                    field.cursor =
                        if end > 0 { prev_char_boundary(&field.value, end) } else { 0 };
                    self.finish_recording();
                }
                VimResult::handled()
            }
            KeyCode::Char('P') => {
                let count = self.effective_count();
                if !self.register.is_empty() {
                    self.start_recording(key);
                    self.push_undo(field);
                    let text = self.register.repeat(count);
                    field.value.insert_str(field.cursor, &text);
                    let end = field.cursor + text.len();
                    field.cursor =
                        if end > 0 { prev_char_boundary(&field.value, end) } else { 0 };
                    self.finish_recording();
                }
                VimResult::handled()
            }
            KeyCode::Char('~') => {
                let count = self.effective_count();
                if field.cursor < field.value.len() {
                    self.start_recording(key);
                    self.push_undo(field);
                    for _ in 0..count {
                        if field.cursor < field.value.len() {
                            toggle_case_at(field, field.cursor);
                            let cl = next_char_len(&field.value, field.cursor);
                            if field.cursor + cl < field.value.len() {
                                field.cursor += cl;
                            }
                        }
                    }
                    self.finish_recording();
                }
                VimResult::handled()
            }
            KeyCode::Char('.') => self.handle_dot_repeat(field),
            KeyCode::Char('u') => {
                self.undo(field);
                VimResult::handled()
            }

            // --- Motions ---
            KeyCode::Char('h') | KeyCode::Left => {
                let n = self.effective_count();
                for _ in 0..n { field.move_left(); }
                VimResult::handled()
            }
            KeyCode::Char('l') | KeyCode::Right => {
                let n = self.effective_count();
                for _ in 0..n { field.move_right(); }
                VimResult::handled()
            }
            KeyCode::Char('w') | KeyCode::Char('W') => {
                let n = self.effective_count();
                for _ in 0..n { field.move_word_forward(); }
                VimResult::handled()
            }
            KeyCode::Char('b') | KeyCode::Char('B') => {
                let n = self.effective_count();
                for _ in 0..n { field.move_word_backward(); }
                VimResult::handled()
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                let n = self.effective_count();
                for _ in 0..n { field.move_word_end(); }
                VimResult::handled()
            }
            KeyCode::Char('0') | KeyCode::Home => {
                field.move_home();
                VimResult::handled()
            }
            KeyCode::Char('^') => {
                field.move_to_first_nonblank();
                VimResult::handled()
            }
            KeyCode::Char('$') | KeyCode::End => {
                if !field.value.is_empty() {
                    field.cursor = prev_char_boundary(&field.value, field.value.len());
                }
                VimResult::handled()
            }

            // --- Find motions ---
            KeyCode::Char('f') => {
                self.awaiting_find = Some(FindDirection::Forward);
                VimResult::handled()
            }
            KeyCode::Char('F') => {
                self.awaiting_find = Some(FindDirection::Backward);
                VimResult::handled()
            }
            KeyCode::Char('t') => {
                self.awaiting_find = Some(FindDirection::TillForward);
                VimResult::handled()
            }
            KeyCode::Char('T') => {
                self.awaiting_find = Some(FindDirection::TillBackward);
                VimResult::handled()
            }
            KeyCode::Char(';') => {
                let n = self.effective_count();
                if let Some((dir, ch)) = self.last_find {
                    for _ in 0..n {
                        if let Some(t) = find_char_in(&field.value, field.cursor, dir, ch) {
                            field.cursor = t;
                        }
                    }
                }
                VimResult::handled()
            }
            KeyCode::Char(',') => {
                let n = self.effective_count();
                if let Some((dir, ch)) = self.last_find {
                    let rev = reverse_find_dir(dir);
                    for _ in 0..n {
                        if let Some(t) = find_char_in(&field.value, field.cursor, rev, ch) {
                            field.cursor = t;
                        }
                    }
                }
                VimResult::handled()
            }

            // --- Field navigation ---
            KeyCode::Char('j') | KeyCode::Down | KeyCode::Tab => {
                VimResult::action(VimAction::NextField)
            }
            KeyCode::Char('k') | KeyCode::Up | KeyCode::BackTab => {
                VimResult::action(VimAction::PrevField)
            }

            // --- Dialog actions ---
            KeyCode::Enter => VimResult::action(VimAction::Confirm),
            KeyCode::Esc | KeyCode::Char('q') => {
                self.clear_pending();
                self.cancel_recording();
                VimResult::action(VimAction::Cancel)
            }

            _ => VimResult::handled(),
        }
    }

    // -----------------------------------------------------------------------
    // Insert mode
    // -----------------------------------------------------------------------

    pub fn handle_insert(&mut self, key: KeyEvent, field: &mut FieldState) -> VimResult {
        self.record_key_if_active(key);

        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                KeyCode::Char('c') => {
                    self.cancel_recording();
                    VimResult::action(VimAction::Cancel)
                }
                KeyCode::Char('w') => {
                    field.delete_word_backward();
                    VimResult::handled()
                }
                KeyCode::Char('u') => {
                    field.clear();
                    VimResult::handled()
                }
                _ => VimResult::handled(),
            };
        }

        match key.code {
            KeyCode::Esc => {
                if field.cursor > 0 {
                    field.move_left();
                }
                self.finish_recording();
                VimResult::mode(DialogVimMode::Normal)
            }
            KeyCode::Enter => VimResult::action(VimAction::Confirm),
            KeyCode::Tab => VimResult::action(VimAction::NextField),
            KeyCode::BackTab => VimResult::action(VimAction::PrevField),
            KeyCode::Backspace => {
                field.backspace();
                VimResult::handled()
            }
            KeyCode::Delete => {
                field.delete();
                VimResult::handled()
            }
            KeyCode::Left => {
                field.move_left();
                VimResult::handled()
            }
            KeyCode::Right => {
                field.move_right();
                VimResult::handled()
            }
            KeyCode::Home => {
                field.move_home();
                VimResult::handled()
            }
            KeyCode::End => {
                field.move_end();
                VimResult::handled()
            }
            KeyCode::Char(c) => {
                field.insert(c);
                VimResult::handled()
            }
            _ => VimResult::handled(),
        }
    }

    // -----------------------------------------------------------------------
    // Visual mode
    // -----------------------------------------------------------------------

    pub fn handle_visual(&mut self, key: KeyEvent, field: &mut FieldState) -> VimResult {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.visual_anchor = None;
            return VimResult {
                new_mode: Some(DialogVimMode::Normal),
                action: Some(VimAction::Cancel),
            };
        }

        if let Some(dir) = self.awaiting_find.take() {
            if let KeyCode::Char(ch) = key.code {
                self.last_find = Some((dir, ch));
                if let Some(t) = find_char_in(&field.value, field.cursor, dir, ch) {
                    field.cursor = t;
                }
            }
            return VimResult::handled();
        }

        if self.awaiting_replace {
            self.awaiting_replace = false;
            if let KeyCode::Char(ch) = key.code {
                let anchor = self.visual_anchor.unwrap_or(field.cursor);
                let (lo, hi) = visual_range(anchor, field.cursor, &field.value);
                self.push_undo(field);
                let mut pos = lo;
                while pos < hi && pos < field.value.len() {
                    replace_char_at(field, pos, ch);
                    pos += next_char_len(&field.value, pos);
                }
                field.cursor = lo;
                self.visual_anchor = None;
                return VimResult::mode(DialogVimMode::Normal);
            }
            // non-char key: fall through to normal visual handling
        }

        let anchor = self.visual_anchor.unwrap_or(field.cursor);

        match key.code {
            // -- motions --
            KeyCode::Char('h') | KeyCode::Left => field.move_left(),
            KeyCode::Char('l') | KeyCode::Right => field.move_right(),
            KeyCode::Char('w') | KeyCode::Char('W') => field.move_word_forward(),
            KeyCode::Char('b') | KeyCode::Char('B') => field.move_word_backward(),
            KeyCode::Char('e') | KeyCode::Char('E') => field.move_word_end(),
            KeyCode::Char('0') | KeyCode::Home => field.move_home(),
            KeyCode::Char('$') | KeyCode::End => {
                if !field.value.is_empty() {
                    field.cursor = prev_char_boundary(&field.value, field.value.len());
                }
            }
            KeyCode::Char('^') => field.move_to_first_nonblank(),
            KeyCode::Char('f') => {
                self.awaiting_find = Some(FindDirection::Forward);
                return VimResult::handled();
            }
            KeyCode::Char('F') => {
                self.awaiting_find = Some(FindDirection::Backward);
                return VimResult::handled();
            }
            KeyCode::Char('t') => {
                self.awaiting_find = Some(FindDirection::TillForward);
                return VimResult::handled();
            }
            KeyCode::Char('T') => {
                self.awaiting_find = Some(FindDirection::TillBackward);
                return VimResult::handled();
            }

            // -- swap anchor/cursor --
            KeyCode::Char('o') => {
                self.visual_anchor = Some(field.cursor);
                field.cursor = anchor;
                return VimResult::handled();
            }

            // -- operations --
            KeyCode::Char('d') | KeyCode::Char('x') => {
                let (lo, hi) = visual_range(anchor, field.cursor, &field.value);
                self.push_undo(field);
                self.register = field.value[lo..hi].to_string();
                field.value.drain(lo..hi);
                field.cursor = lo;
                clamp_cursor_normal(field);
                self.visual_anchor = None;
                return VimResult::mode(DialogVimMode::Normal);
            }
            KeyCode::Char('c') | KeyCode::Char('s') => {
                let (lo, hi) = visual_range(anchor, field.cursor, &field.value);
                self.push_undo(field);
                self.register = field.value[lo..hi].to_string();
                field.value.drain(lo..hi);
                field.cursor = lo;
                self.visual_anchor = None;
                return VimResult::mode(DialogVimMode::Insert);
            }
            KeyCode::Char('y') => {
                let (lo, hi) = visual_range(anchor, field.cursor, &field.value);
                self.register = field.value[lo..hi].to_string();
                field.cursor = lo;
                self.visual_anchor = None;
                return VimResult::mode(DialogVimMode::Normal);
            }
            KeyCode::Char('r') => {
                self.awaiting_replace = true;
                return VimResult::handled();
            }
            KeyCode::Char('~') => {
                let (lo, hi) = visual_range(anchor, field.cursor, &field.value);
                self.push_undo(field);
                let mut pos = lo;
                while pos < hi && pos < field.value.len() {
                    toggle_case_at(field, pos);
                    pos += next_char_len(&field.value, pos);
                }
                field.cursor = lo;
                self.visual_anchor = None;
                return VimResult::mode(DialogVimMode::Normal);
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                if !self.register.is_empty() {
                    let (lo, hi) = visual_range(anchor, field.cursor, &field.value);
                    self.push_undo(field);
                    let old_sel = field.value[lo..hi].to_string();
                    field.value.drain(lo..hi);
                    let reg = self.register.clone();
                    field.value.insert_str(lo, &reg);
                    field.cursor = lo;
                    clamp_cursor_normal(field);
                    self.register = old_sel;
                }
                self.visual_anchor = None;
                return VimResult::mode(DialogVimMode::Normal);
            }

            // -- cancel --
            KeyCode::Esc | KeyCode::Char('v') => {
                self.visual_anchor = None;
                return VimResult::mode(DialogVimMode::Normal);
            }

            _ => return VimResult::handled(),
        }

        // Motion was applied, stay in visual mode
        VimResult::handled()
    }

    // -----------------------------------------------------------------------
    // Replace mode (R)
    // -----------------------------------------------------------------------

    pub fn handle_replace(&mut self, key: KeyEvent, field: &mut FieldState) -> VimResult {
        self.record_key_if_active(key);

        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.replace_backup.clear();
            self.cancel_recording();
            return VimResult::action(VimAction::Cancel);
        }

        match key.code {
            KeyCode::Esc => {
                if field.cursor > 0 {
                    field.move_left();
                }
                self.replace_backup.clear();
                self.finish_recording();
                VimResult::mode(DialogVimMode::Normal)
            }
            KeyCode::Backspace => {
                if let Some(original) = self.replace_backup.pop() {
                    if field.cursor > 0 {
                        field.move_left();
                        if original != '\0' {
                            replace_char_at(field, field.cursor, original);
                        }
                    }
                }
                VimResult::handled()
            }
            KeyCode::Char(c) => {
                if field.cursor < field.value.len() {
                    let orig = field.value[field.cursor..].chars().next().unwrap_or(' ');
                    self.replace_backup.push(orig);
                    replace_char_at(field, field.cursor, c);
                    let cl = c.len_utf8();
                    if field.cursor + cl <= field.value.len() {
                        field.cursor += cl;
                    }
                } else {
                    field.insert(c);
                    self.replace_backup.push('\0');
                }
                VimResult::handled()
            }
            _ => VimResult::handled(),
        }
    }

    // -----------------------------------------------------------------------
    // Dot-repeat
    // -----------------------------------------------------------------------

    fn handle_dot_repeat(&mut self, field: &mut FieldState) -> VimResult {
        let Some(edit) = self.last_edit.clone() else {
            return VimResult::handled();
        };
        let override_count = self.count_accum.take();
        self.push_undo(field);
        self.replaying = true;
        self.count_accum = override_count.or(edit.count);
        self.op_count = None;

        let mut current_mode = DialogVimMode::Normal;
        for key in &edit.keys {
            let result = match current_mode {
                DialogVimMode::Normal => self.handle_normal(*key, field),
                DialogVimMode::Insert => self.handle_insert(*key, field),
                DialogVimMode::Replace => self.handle_replace(*key, field),
                DialogVimMode::Visual => self.handle_visual(*key, field),
            };
            if let Some(m) = result.new_mode {
                current_mode = m;
            }
        }

        self.replaying = false;
        VimResult::mode(DialogVimMode::Normal)
    }
}

// ---------------------------------------------------------------------------
// Free helper functions
// ---------------------------------------------------------------------------

fn prev_char_boundary(s: &str, pos: usize) -> usize {
    if pos == 0 {
        return 0;
    }
    s[..pos]
        .char_indices()
        .last()
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn next_char_len(s: &str, pos: usize) -> usize {
    s[pos..].chars().next().map(|c| c.len_utf8()).unwrap_or(1)
}

fn clamp_cursor_normal(field: &mut FieldState) {
    if field.value.is_empty() {
        field.cursor = 0;
    } else if field.cursor >= field.value.len() {
        field.cursor = prev_char_boundary(&field.value, field.value.len());
    }
}

fn replace_char_at(field: &mut FieldState, pos: usize, ch: char) {
    if pos < field.value.len() {
        let old_len = next_char_len(&field.value, pos);
        let new_str = ch.to_string();
        field.value.replace_range(pos..pos + old_len, &new_str);
    }
}

fn toggle_case_at(field: &mut FieldState, pos: usize) {
    if pos >= field.value.len() {
        return;
    }
    let cl = next_char_len(&field.value, pos);
    let ch = field.value[pos..pos + cl].chars().next().unwrap();
    let toggled: String = if ch.is_uppercase() {
        ch.to_lowercase().to_string()
    } else {
        ch.to_uppercase().to_string()
    };
    field.value.replace_range(pos..pos + cl, &toggled);
}

// -- motion resolution for operators ----------------------------------------

fn resolve_motion_range(field: &FieldState, key: KeyCode, count: usize) -> Option<(usize, usize)> {
    let cursor = field.cursor;
    let value = &field.value;

    match key {
        KeyCode::Char('h') | KeyCode::Left => {
            let mut t = cursor;
            for _ in 0..count {
                if t > 0 { t = prev_char_boundary(value, t); }
            }
            if t < cursor { Some((t, cursor)) } else { None }
        }
        KeyCode::Char('l') | KeyCode::Right => {
            let mut t = cursor;
            for _ in 0..count {
                if t < value.len() { t += next_char_len(value, t); }
            }
            if t > cursor { Some((cursor, t)) } else { None }
        }
        KeyCode::Char('w') | KeyCode::Char('W') => {
            let mut t = cursor;
            for _ in 0..count { t = pos_word_forward(value, t); }
            if t > cursor { Some((cursor, t)) } else { None }
        }
        KeyCode::Char('b') | KeyCode::Char('B') => {
            let mut t = cursor;
            for _ in 0..count { t = pos_word_backward(value, t); }
            if t < cursor { Some((t, cursor)) } else { None }
        }
        KeyCode::Char('e') | KeyCode::Char('E') => {
            let mut t = cursor;
            for _ in 0..count { t = pos_word_end(value, t); }
            if t >= cursor && t < value.len() {
                Some((cursor, t + next_char_len(value, t)))
            } else {
                None
            }
        }
        KeyCode::Char('0') | KeyCode::Home => {
            if cursor > 0 { Some((0, cursor)) } else { None }
        }
        KeyCode::Char('^') => {
            let t = pos_first_nonblank(value);
            if t < cursor {
                Some((t, cursor))
            } else if t > cursor {
                Some((cursor, t))
            } else {
                None
            }
        }
        KeyCode::Char('$') | KeyCode::End => {
            if cursor < value.len() { Some((cursor, value.len())) } else { None }
        }
        _ => None,
    }
}

// -- find helpers -----------------------------------------------------------

fn find_char_in(value: &str, cursor: usize, dir: FindDirection, ch: char) -> Option<usize> {
    match dir {
        FindDirection::Forward => {
            if cursor >= value.len() { return None; }
            let skip = next_char_len(value, cursor);
            let start = cursor + skip;
            if start >= value.len() { return None; }
            value[start..].find(ch).map(|i| start + i)
        }
        FindDirection::Backward => {
            if cursor == 0 { return None; }
            value[..cursor].rfind(ch)
        }
        FindDirection::TillForward => {
            if cursor >= value.len() { return None; }
            let skip = next_char_len(value, cursor);
            let start = cursor + skip;
            if start >= value.len() { return None; }
            value[start..].find(ch).map(|i| {
                let found = start + i;
                if found > 0 { prev_char_boundary(value, found) } else { 0 }
            })
        }
        FindDirection::TillBackward => {
            if cursor == 0 { return None; }
            value[..cursor].rfind(ch).map(|found| {
                let cl = next_char_len(value, found);
                (found + cl).min(cursor)
            })
        }
    }
}

fn find_char_repeated(
    value: &str,
    cursor: usize,
    dir: FindDirection,
    ch: char,
    count: usize,
) -> Option<usize> {
    let raw_dir = match dir {
        FindDirection::TillForward => FindDirection::Forward,
        FindDirection::TillBackward => FindDirection::Backward,
        d => d,
    };
    let mut pos = cursor;
    for _ in 0..count {
        pos = find_char_in(value, pos, raw_dir, ch)?;
    }
    match dir {
        FindDirection::TillForward => {
            if pos > 0 { Some(prev_char_boundary(value, pos)) } else { Some(0) }
        }
        FindDirection::TillBackward => {
            let cl = next_char_len(value, pos);
            Some((pos + cl).min(value.len()))
        }
        _ => Some(pos),
    }
}

fn find_operator_range(value: &str, cursor: usize, target: usize) -> (usize, usize) {
    if cursor <= target {
        let end = if target < value.len() {
            target + next_char_len(value, target)
        } else {
            value.len()
        };
        (cursor, end)
    } else {
        let end = if cursor < value.len() {
            cursor + next_char_len(value, cursor)
        } else {
            value.len()
        };
        (target, end)
    }
}

fn reverse_find_dir(dir: FindDirection) -> FindDirection {
    match dir {
        FindDirection::Forward => FindDirection::Backward,
        FindDirection::Backward => FindDirection::Forward,
        FindDirection::TillForward => FindDirection::TillBackward,
        FindDirection::TillBackward => FindDirection::TillForward,
    }
}

// -- word position helpers --------------------------------------------------

fn pos_word_forward(value: &str, from: usize) -> usize {
    let bytes = value.as_bytes();
    let mut i = from;
    while i < bytes.len() && !bytes[i].is_ascii_whitespace() { i += 1; }
    while i < bytes.len() && bytes[i].is_ascii_whitespace() { i += 1; }
    i
}

fn pos_word_backward(value: &str, from: usize) -> usize {
    let bytes = value.as_bytes();
    let mut i = from;
    if i > 0 { i -= 1; }
    while i > 0 && bytes[i].is_ascii_whitespace() { i -= 1; }
    while i > 0 && !bytes[i - 1].is_ascii_whitespace() { i -= 1; }
    i
}

fn pos_word_end(value: &str, from: usize) -> usize {
    let bytes = value.as_bytes();
    let len = bytes.len();
    if from >= len { return from; }
    let mut i = from + 1;
    while i < len && bytes[i].is_ascii_whitespace() { i += 1; }
    while i < len && !bytes[i].is_ascii_whitespace() { i += 1; }
    if i > from + 1 { i - 1 } else { i.min(len.saturating_sub(1)) }
}

fn pos_first_nonblank(value: &str) -> usize {
    let bytes = value.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() { i += 1; }
    i
}

// -- text objects -----------------------------------------------------------

fn text_object_range(
    value: &str,
    cursor: usize,
    obj: TextObject,
) -> Option<(usize, usize)> {
    match obj {
        TextObject::InnerWord => {
            let bytes = value.as_bytes();
            if cursor >= bytes.len() { return None; }
            let on_ws = bytes[cursor].is_ascii_whitespace();
            let mut start = cursor;
            let mut end = cursor;
            if on_ws {
                while start > 0 && bytes[start - 1].is_ascii_whitespace() { start -= 1; }
                while end < bytes.len() && bytes[end].is_ascii_whitespace() { end += 1; }
            } else {
                while start > 0 && !bytes[start - 1].is_ascii_whitespace() { start -= 1; }
                while end < bytes.len() && !bytes[end].is_ascii_whitespace() { end += 1; }
            }
            Some((start, end))
        }
        TextObject::AWord => {
            let (start, end) = text_object_range(value, cursor, TextObject::InnerWord)?;
            let bytes = value.as_bytes();
            let mut new_end = end;
            while new_end < bytes.len() && bytes[new_end].is_ascii_whitespace() { new_end += 1; }
            if new_end > end {
                Some((start, new_end))
            } else {
                let mut new_start = start;
                while new_start > 0 && bytes[new_start - 1].is_ascii_whitespace() {
                    new_start -= 1;
                }
                Some((new_start, end))
            }
        }
        TextObject::InnerQuoted(q) => {
            let positions: Vec<usize> = value.match_indices(q).map(|(i, _)| i).collect();
            for pair in positions.chunks(2) {
                if pair.len() == 2 {
                    let (open, close) = (pair[0], pair[1]);
                    if cursor >= open && cursor <= close {
                        return Some((open + q.len_utf8(), close));
                    }
                }
            }
            None
        }
        TextObject::AQuoted(q) => {
            let (inner_start, inner_end) =
                text_object_range(value, cursor, TextObject::InnerQuoted(q))?;
            let open = inner_start.saturating_sub(q.len_utf8());
            let close = (inner_end + q.len_utf8()).min(value.len());
            Some((open, close))
        }
        TextObject::InnerParen => {
            let bytes = value.as_bytes();
            let mut depth: i32 = 0;
            let mut open_pos = None;
            for (i, &b) in bytes.iter().enumerate() {
                if b == b'(' {
                    if depth == 0 && i <= cursor {
                        open_pos = Some(i);
                    }
                    depth += 1;
                } else if b == b')' {
                    depth -= 1;
                    if depth == 0 {
                        if let Some(o) = open_pos {
                            if cursor >= o && cursor <= i {
                                return Some((o + 1, i));
                            }
                        }
                        open_pos = None;
                    }
                }
            }
            None
        }
        TextObject::AParen => {
            let (inner_start, inner_end) =
                text_object_range(value, cursor, TextObject::InnerParen)?;
            Some((inner_start.saturating_sub(1), (inner_end + 1).min(value.len())))
        }
    }
}

// -- operator application ---------------------------------------------------

fn apply_operator(
    op: Operator,
    field: &mut FieldState,
    lo: usize,
    hi: usize,
    state: &mut VimState,
) -> Option<DialogVimMode> {
    let hi = hi.min(field.value.len());
    if lo >= hi {
        return None;
    }

    let text = field.value[lo..hi].to_string();

    match op {
        Operator::Delete => {
            state.register = text;
            field.value.drain(lo..hi);
            field.cursor = lo;
            clamp_cursor_normal(field);
            None
        }
        Operator::Change => {
            state.register = text;
            field.value.drain(lo..hi);
            field.cursor = lo;
            Some(DialogVimMode::Insert)
        }
        Operator::Yank => {
            state.register = text;
            None
        }
    }
}

// -- public helper for rendering --------------------------------------------

pub fn visual_selection_range(anchor: usize, cursor: usize, value: &str) -> (usize, usize) {
    let lo = anchor.min(cursor);
    let hi = anchor.max(cursor);
    let hi_end = if hi < value.len() {
        hi + next_char_len(value, hi)
    } else {
        value.len()
    };
    (lo, hi_end)
}

fn visual_range(anchor: usize, cursor: usize, value: &str) -> (usize, usize) {
    visual_selection_range(anchor, cursor, value)
}
