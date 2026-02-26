use chrono::{DateTime, NaiveDate, Utc};
use color_eyre::eyre::{Result, WrapErr};
use std::path::Path;
use uuid::Uuid;

use crate::model::{Board, ColumnKind, Task};

#[derive(serde::Serialize, serde::Deserialize)]
struct CsvRow {
    id: String,
    title: String,
    description: String,
    tags: String,
    due_date: String,
    created_at: String,
    updated_at: String,
    column: String,
    time_in_doing_secs: u64,
}

fn task_to_row(t: &Task) -> CsvRow {
    CsvRow {
        id: t.id.to_string(),
        title: t.title.clone(),
        description: t.description.clone(),
        tags: t.tags.join(";"),
        due_date: t
            .due_date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
        created_at: t.created_at.to_rfc3339(),
        updated_at: t.updated_at.to_rfc3339(),
        column: t.column.to_string(),
        time_in_doing_secs: t.time_in_doing_secs,
    }
}

fn row_to_task(r: CsvRow) -> Result<Task> {
    let column = match r.column.as_str() {
        "Ready" => ColumnKind::Ready,
        "Doing" => ColumnKind::Doing,
        "Done" => ColumnKind::Done,
        "Archived" => ColumnKind::Archived,
        other => color_eyre::eyre::bail!("unknown column: {other}"),
    };

    let due_date = if r.due_date.is_empty() {
        None
    } else {
        Some(NaiveDate::parse_from_str(&r.due_date, "%Y-%m-%d")?)
    };

    let tags: Vec<String> = if r.tags.is_empty() {
        Vec::new()
    } else {
        r.tags.split(';').map(String::from).collect()
    };

    Ok(Task {
        id: Uuid::parse_str(&r.id)?,
        title: r.title,
        description: r.description,
        tags,
        due_date,
        created_at: r.created_at.parse::<DateTime<Utc>>()?,
        updated_at: r.updated_at.parse::<DateTime<Utc>>()?,
        column,
        time_in_doing_secs: r.time_in_doing_secs,
        doing_since: None,
    })
}

pub fn export(board: &Board, path: &Path) -> Result<()> {
    let mut wtr = csv::Writer::from_path(path)
        .wrap_err_with(|| format!("creating {}", path.display()))?;
    for task in &board.tasks {
        wtr.serialize(task_to_row(task))?;
    }
    wtr.flush()?;
    Ok(())
}

pub fn import(path: &Path) -> Result<Board> {
    let mut rdr =
        csv::Reader::from_path(path).wrap_err_with(|| format!("reading {}", path.display()))?;
    let mut tasks = Vec::new();
    for result in rdr.deserialize() {
        let row: CsvRow = result?;
        tasks.push(row_to_task(row)?);
    }
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("imported")
        .to_string();
    Ok(Board { name, tasks })
}
