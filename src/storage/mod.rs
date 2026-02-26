pub mod csv_backend;
pub mod json_backend;

use color_eyre::eyre::Result;
use std::path::Path;

use crate::model::Board;

#[allow(dead_code)]
pub trait StorageBackend {
    fn load_board(&self, name: &str) -> Result<Board>;
    fn save_board(&self, board: &Board) -> Result<()>;
    fn list_boards(&self) -> Result<Vec<String>>;
    fn delete_board(&self, name: &str) -> Result<()>;
    fn export_csv(&self, board: &Board, path: &Path) -> Result<()>;
    fn import_csv(&self, path: &Path) -> Result<Board>;
}
