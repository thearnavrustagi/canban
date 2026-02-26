use color_eyre::eyre::{Result, WrapErr};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub active_board: String,
    pub done_limit: usize,

    #[serde(default)]
    pub columns: ColumnsConfig,

    #[serde(default)]
    pub display: DisplayConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnsConfig {
    pub visible: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub column_min_width: u16,
    pub show_footer: bool,
}

impl Default for ColumnsConfig {
    fn default() -> Self {
        Self {
            visible: vec![
                "Ready".into(),
                "Doing".into(),
                "Done".into(),
                "Archived".into(),
            ],
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            column_min_width: 30,
            show_footer: true,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            active_board: "default".into(),
            done_limit: 20,
            columns: ColumnsConfig::default(),
            display: DisplayConfig::default(),
        }
    }
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("canban")
}

pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("canban")
}

pub fn boards_dir() -> PathBuf {
    data_dir().join("boards")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path();
        if !path.exists() {
            let cfg = Config::default();
            cfg.save()?;
            return Ok(cfg);
        }
        let contents =
            fs::read_to_string(&path).wrap_err_with(|| format!("reading {}", path.display()))?;
        let cfg: Config =
            toml::from_str(&contents).wrap_err_with(|| format!("parsing {}", path.display()))?;
        Ok(cfg)
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self)?;
        fs::write(&path, contents)?;
        Ok(())
    }
}
