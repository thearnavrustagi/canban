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

    pub fn all_tags(&self) -> Vec<String> {
        let mut tags: Vec<String> = self
            .tasks
            .iter()
            .flat_map(|t| t.tags.iter().cloned())
            .collect();
        tags.sort();
        tags.dedup();
        tags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_board_new() {
        let board = Board::new("My Board".into());
        assert_eq!(board.name, "My Board");
        assert_eq!(board.tasks.len(), 1); // Welcome task
        assert_eq!(board.tasks[0].title, "Welcome to canban!");
        assert_eq!(board.tasks[0].column, ColumnKind::Ready);
    }

    #[test]
    fn test_tasks_in_column() {
        let mut board = Board::new("My Board".into());
        board.tasks.push(Task::new("Doing Task".into(), ColumnKind::Doing));
        
        assert_eq!(board.tasks_in_column(ColumnKind::Ready).len(), 1);
        assert_eq!(board.tasks_in_column(ColumnKind::Doing).len(), 1);
        assert_eq!(board.column_count(ColumnKind::Doing), 1);
    }

    #[test]
    fn test_all_tags() {
        let mut board = Board::new("My Board".into());
        board.tasks.clear(); // remove welcome task
        
        let mut t1 = Task::new("T1".into(), ColumnKind::Ready);
        t1.tags = vec!["bug".into(), "ui".into()];
        
        let mut t2 = Task::new("T2".into(), ColumnKind::Ready);
        t2.tags = vec!["ui".into(), "feature".into()];
        
        board.tasks.push(t1);
        board.tasks.push(t2);
        
        let tags = board.all_tags();
        assert_eq!(tags, vec!["bug".to_string(), "feature".to_string(), "ui".to_string()]);
    }
}
