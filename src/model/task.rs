use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::column::ColumnKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub due_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub column: ColumnKind,
    pub time_in_doing_secs: u64,
    pub doing_since: Option<DateTime<Utc>>,
}

impl Task {
    pub fn new(title: String, column: ColumnKind) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title,
            description: String::new(),
            tags: Vec::new(),
            due_date: None,
            created_at: now,
            updated_at: now,
            column,
            time_in_doing_secs: 0,
            doing_since: None,
        }
    }

    pub fn move_to(&mut self, target: ColumnKind) {
        self.finalize_doing_time();
        if target == ColumnKind::Doing {
            self.doing_since = Some(Utc::now());
        }
        self.column = target;
        self.updated_at = Utc::now();
    }

    pub fn finalize_doing_time(&mut self) {
        if let Some(since) = self.doing_since.take() {
            let elapsed = Utc::now().signed_duration_since(since);
            self.time_in_doing_secs += elapsed.num_seconds().max(0) as u64;
        }
    }

    pub fn effective_doing_secs(&self) -> u64 {
        let extra = self
            .doing_since
            .map(|s| Utc::now().signed_duration_since(s).num_seconds().max(0) as u64)
            .unwrap_or(0);
        self.time_in_doing_secs + extra
    }

    pub fn is_overdue(&self) -> bool {
        self.due_date
            .map(|d| d < Utc::now().date_naive())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new("Test Task".into(), ColumnKind::Ready);
        assert_eq!(task.title, "Test Task");
        assert_eq!(task.column, ColumnKind::Ready);
        assert_eq!(task.time_in_doing_secs, 0);
        assert!(task.doing_since.is_none());
    }

    #[test]
    fn test_task_move_to_doing() {
        let mut task = Task::new("Test".into(), ColumnKind::Ready);
        task.move_to(ColumnKind::Doing);
        
        assert_eq!(task.column, ColumnKind::Doing);
        assert!(task.doing_since.is_some());
    }

    #[test]
    fn test_task_move_out_of_doing() {
        let mut task = Task::new("Test".into(), ColumnKind::Doing);
        task.doing_since = Some(Utc::now() - chrono::Duration::seconds(10));
        
        task.move_to(ColumnKind::Done);
        assert_eq!(task.column, ColumnKind::Done);
        assert!(task.doing_since.is_none());
        assert!(task.time_in_doing_secs >= 10);
    }
}
