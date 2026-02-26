use serde::{Deserialize, Serialize};

use super::column::ColumnKind;
use super::task::Task;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    pub name: String,
    pub tasks: Vec<Task>,
}

impl Board {
    pub fn new(name: String) -> Self {
        let mut welcome = Task::new("Welcome to canban!".into(), ColumnKind::Ready);
        welcome.description = "Use 'n' to create tasks, h/j/k/l to navigate".into();
        welcome.tags = vec!["getting-started".into()];
        Self {
            name,
            tasks: vec![welcome],
        }
    }

    pub fn tasks_in_column(&self, col: ColumnKind) -> Vec<&Task> {
        self.tasks.iter().filter(|t| t.column == col).collect()
    }

    #[allow(dead_code)]
    pub fn tasks_in_column_mut(&mut self, col: ColumnKind) -> Vec<&mut Task> {
        self.tasks.iter_mut().filter(|t| t.column == col).collect()
    }

    pub fn column_count(&self, col: ColumnKind) -> usize {
        self.tasks.iter().filter(|t| t.column == col).count()
    }
}
