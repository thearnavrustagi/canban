use color_eyre::eyre::{Result, WrapErr};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::boards_dir;
use crate::model::Board;

use super::csv_backend;
use super::StorageBackend;

pub struct JsonBackend {
    base: PathBuf,
}

impl JsonBackend {
    pub fn new() -> Self {
        Self { base: boards_dir() }
    }

    fn board_path(&self, name: &str) -> PathBuf {
        self.base.join(name).join("tasks.json")
    }
}

impl StorageBackend for JsonBackend {
    fn load_board(&self, name: &str) -> Result<Board> {
        let path = self.board_path(name);
        if !path.exists() {
            let board = Board::new(name.to_string());
            self.save_board(&board)?;
            return Ok(board);
        }
        let data =
            fs::read_to_string(&path).wrap_err_with(|| format!("reading {}", path.display()))?;
        let board: Board =
            serde_json::from_str(&data).wrap_err_with(|| format!("parsing {}", path.display()))?;
        Ok(board)
    }

    fn save_board(&self, board: &Board) -> Result<()> {
        let path = self.board_path(&board.name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(board)?;
        fs::write(&path, data)?;
        Ok(())
    }

    fn list_boards(&self) -> Result<Vec<String>> {
        if !self.base.exists() {
            return Ok(Vec::new());
        }
        let mut boards = Vec::new();
        for entry in fs::read_dir(&self.base)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    boards.push(name.to_string());
                }
            }
        }
        boards.sort();
        Ok(boards)
    }

    fn delete_board(&self, name: &str) -> Result<()> {
        let dir = self.base.join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }

    fn export_csv(&self, board: &Board, path: &Path) -> Result<()> {
        csv_backend::export(board, path)
    }

    fn import_csv(&self, path: &Path) -> Result<Board> {
        csv_backend::import(path)
    }
}
