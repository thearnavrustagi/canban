mod app;
mod cli;
mod config;
mod event;
mod model;
mod storage;
mod ui;
mod vim;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use chrono::NaiveDate;
use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::config::{boards_dir, config_path, Config};
use crate::event::{Event, EventHandler};
use crate::model::{Board, ColumnKind, Task};
use crate::storage::json_backend::JsonBackend;
use crate::storage::StorageBackend;

#[derive(Parser)]
#[command(
    name = "canban",
    version,
    about = "A terminal Kanban board with vim bindings",
    styles = cli::styles(),
    after_help = "Run 'canban' with no arguments to launch the interactive TUI."
)]
struct Cli {
    /// Open a board directly (last active board if omitted)
    #[arg(short = 'o', long = "open", value_name = "BOARD", num_args = 0..=1, default_missing_value = "")]
    open: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new empty board
    New {
        /// Board name
        name: String,
    },
    /// Delete a board permanently
    Delete {
        /// Board name
        name: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
    /// Rename a board
    Rename {
        /// Current name
        from: String,
        /// New name
        to: String,
    },
    /// Show board summary with task counts
    Show {
        /// Board name (defaults to active board)
        name: Option<String>,
    },
    /// Quick-add a task to a board
    Add {
        /// Task title
        title: String,
        /// Target board (defaults to active board)
        #[arg(short, long)]
        board: Option<String>,
        /// Column: ready, doing, done (default: ready)
        #[arg(short, long, default_value = "ready")]
        column: String,
        /// Comma-separated tags
        #[arg(short, long)]
        tags: Option<String>,
        /// Due date (YYYY-MM-DD)
        #[arg(short, long)]
        due: Option<String>,
    },
    /// Export the active board to CSV
    Export {
        /// Output file path
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Import a board from CSV
    Import {
        /// Input CSV file path
        #[arg(short, long)]
        input: PathBuf,
    },
    /// List all boards
    Boards,
    /// Show configuration paths and values
    Config,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let cli_args = Cli::parse();
    let mut config = Config::load()?;
    let storage = JsonBackend::new();

    match (cli_args.open, cli_args.command) {
        (Some(_), Some(_)) => cli::fail("Cannot use --open together with a subcommand."),
        (Some(board), None) => cmd_open(storage, config, &board),
        (None, Some(cmd)) => dispatch(cmd, &storage, &mut config),
        (None, None) => run_tui(Box::new(storage), config, false),
    }
}

fn dispatch(cmd: Commands, storage: &JsonBackend, config: &mut Config) -> Result<()> {
    match cmd {
        Commands::New { name } => cmd_new(storage, &name),
        Commands::Delete { name, force } => cmd_delete(storage, config, &name, force),
        Commands::Rename { from, to } => cmd_rename(storage, config, &from, &to),
        Commands::Show { name } => cmd_show(storage, config, name.as_deref()),
        Commands::Add { title, board, column, tags, due } => {
            cmd_add(storage, config, &title, board.as_deref(), &column, tags.as_deref(), due.as_deref())
        }
        Commands::Export { output } => cmd_export(storage, config, &output),
        Commands::Import { input } => cmd_import(storage, &input),
        Commands::Boards => cmd_boards(storage, config),
        Commands::Config => cmd_config(config),
    }
}

// ── open ────────────────────────────────────────────────────────────

fn cmd_open(storage: JsonBackend, mut config: Config, board: &str) -> Result<()> {
    let target = resolve_open_target(&storage, &config, board)?;
    config.active_board = target;
    config.save()?;
    run_tui(Box::new(storage), config, true)
}

fn resolve_open_target(storage: &JsonBackend, config: &Config, board: &str) -> Result<String> {
    let boards = storage.list_boards()?;
    if boards.is_empty() { cli::fail("No boards found. Run `canban` to create one."); }
    let target = if board.is_empty() { config.active_board.as_str() } else { board };
    if !boards.contains(&target.to_string()) {
        cli::fail(&format!("Board '{}' not found. Available: {}", target, boards.join(", ")));
    }
    Ok(target.to_string())
}

// ── new ─────────────────────────────────────────────────────────────

fn cmd_new(storage: &JsonBackend, name: &str) -> Result<()> {
    if storage.list_boards()?.contains(&name.to_string()) {
        cli::fail(&format!("Board '{name}' already exists."));
    }
    storage.save_board(&Board::new(name.to_string()))?;
    cli::header("new board");
    cli::success(&format!("Created board '{name}'"));
    println!();
    Ok(())
}

// ── delete ──────────────────────────────────────────────────────────

fn cmd_delete(storage: &JsonBackend, config: &mut Config, name: &str, force: bool) -> Result<()> {
    require_board(storage, name)?;
    if !force && !cli::confirm(&format!("Delete board '{name}' permanently?")) {
        cli::hint("Cancelled.");
        return Ok(());
    }
    storage.delete_board(name)?;
    reset_active_if_needed(config, name)?;
    cli::header("delete board");
    cli::success(&format!("Deleted board '{name}'"));
    println!();
    Ok(())
}

fn reset_active_if_needed(config: &mut Config, deleted: &str) -> Result<()> {
    if config.active_board == deleted {
        config.active_board = "default".into();
        config.save()?;
    }
    Ok(())
}

// ── rename ──────────────────────────────────────────────────────────

fn cmd_rename(storage: &JsonBackend, config: &mut Config, from: &str, to: &str) -> Result<()> {
    storage.rename_board(from, to)?;
    if config.active_board == from {
        config.active_board = to.to_string();
        config.save()?;
    }
    cli::header("rename board");
    cli::success(&format!("Renamed '{from}' → '{to}'"));
    println!();
    Ok(())
}

// ── show ────────────────────────────────────────────────────────────

fn cmd_show(storage: &JsonBackend, config: &Config, name: Option<&str>) -> Result<()> {
    let target = name.unwrap_or(&config.active_board);
    require_board(storage, target)?;
    let board = storage.load_board(target)?;
    cli::header("board summary");
    cli::board_title(&board.name);
    cli::separator();
    for col in ColumnKind::ALL {
        cli::column_row(&col.to_string(), board.column_count(col));
    }
    cli::separator();
    let noun = if board.tasks.len() == 1 { "task" } else { "tasks" };
    cli::count_line(board.tasks.len(), &format!("{noun} total"));
    println!();
    Ok(())
}

// ── add ─────────────────────────────────────────────────────────────

fn cmd_add(
    storage: &JsonBackend,
    config: &Config,
    title: &str,
    board: Option<&str>,
    column: &str,
    tags: Option<&str>,
    due: Option<&str>,
) -> Result<()> {
    let board_name = board.unwrap_or(&config.active_board);
    require_board(storage, board_name)?;
    let col = parse_column(column);
    let tag_vec = parse_tags(tags);
    let due_date = parse_due(due);
    save_task(storage, board_name, build_task(title, col, &tag_vec, due_date))?;
    print_task_added(board_name, &col.to_string(), title, &tag_vec, due);
    Ok(())
}

fn build_task(title: &str, col: ColumnKind, tags: &[String], due: Option<NaiveDate>) -> Task {
    let mut task = Task::new(title.to_string(), col);
    task.tags = tags.to_vec();
    task.due_date = due;
    task
}

fn save_task(storage: &JsonBackend, board_name: &str, task: Task) -> Result<()> {
    let mut board = storage.load_board(board_name)?;
    board.tasks.push(task);
    storage.save_board(&board)
}

fn print_task_added(board: &str, col: &str, title: &str, tags: &[String], due: Option<&str>) {
    let color = cli::col_color(col);
    let icon = cli::col_icon(col);
    let r = cli::R;
    cli::header("add task");
    cli::success(&format!("Added '{title}' to {board} {color}{icon} {col}{r}"));
    cli::tag_list(tags);
    if let Some(d) = due { cli::due_line(d); }
    println!();
}

// ── export / import ─────────────────────────────────────────────────

fn cmd_export(storage: &JsonBackend, config: &Config, output: &PathBuf) -> Result<()> {
    let board = storage.load_board(&config.active_board)?;
    storage.export_csv(&board, output)?;
    cli::header("export");
    cli::success(&format!("Exported '{}' → {}", board.name, output.display()));
    println!();
    Ok(())
}

fn cmd_import(storage: &JsonBackend, input: &PathBuf) -> Result<()> {
    let board = storage.import_csv(input)?;
    let name = board.name.clone();
    storage.save_board(&board)?;
    cli::header("import");
    cli::success(&format!("Imported '{name}' from {}", input.display()));
    println!();
    Ok(())
}

// ── boards ──────────────────────────────────────────────────────────

fn cmd_boards(storage: &JsonBackend, config: &Config) -> Result<()> {
    let boards = storage.list_boards()?;
    cli::header("boards");
    if boards.is_empty() {
        cli::hint("No boards yet. Run `canban` to create one.");
    } else {
        for b in &boards {
            cli::board_line(b, *b == config.active_board);
        }
        println!();
        cli::count_line(boards.len(), "boards");
    }
    println!();
    Ok(())
}

// ── config ──────────────────────────────────────────────────────────

fn cmd_config(config: &Config) -> Result<()> {
    cli::header("configuration");
    print_config_paths();
    print_config_values(config);
    println!();
    Ok(())
}

fn print_config_paths() {
    cli::kv("Config file", &shorten_home(&config_path()));
    cli::kv("Data directory", &shorten_home(&boards_dir()));
    println!();
}

fn print_config_values(config: &Config) {
    cli::kv("Active board", &config.active_board);
    cli::kv("Done limit", &config.done_limit.to_string());
    cli::kv("Visible columns", &config.columns.visible.join(", "));
    cli::kv("Column min width", &config.display.column_min_width.to_string());
    cli::kv("Show footer", &config.display.show_footer.to_string());
}

// ── helpers ─────────────────────────────────────────────────────────

fn require_board(storage: &JsonBackend, name: &str) -> Result<()> {
    if !storage.list_boards()?.contains(&name.to_string()) {
        cli::fail(&format!("Board '{name}' not found"));
    }
    Ok(())
}

fn parse_column(s: &str) -> ColumnKind {
    match s.to_lowercase().as_str() {
        "ready" => ColumnKind::Ready,
        "doing" => ColumnKind::Doing,
        "done" => ColumnKind::Done,
        "archived" => ColumnKind::Archived,
        _ => cli::fail(&format!("Unknown column '{s}'. Use: ready, doing, done, archived")),
    }
}

fn parse_tags(s: Option<&str>) -> Vec<String> {
    s.map(|t| t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        .unwrap_or_default()
}

fn parse_due(s: Option<&str>) -> Option<NaiveDate> {
    s.map(|d| {
        NaiveDate::parse_from_str(d, "%Y-%m-%d")
            .unwrap_or_else(|_| cli::fail(&format!("Invalid date '{d}'. Use YYYY-MM-DD format")))
    })
}

fn shorten_home(path: &std::path::Path) -> String {
    dirs::home_dir()
        .and_then(|h| path.strip_prefix(&h).ok().map(|r| format!("~/{}", r.display())))
        .unwrap_or_else(|| path.display().to_string())
}

// ── TUI ─────────────────────────────────────────────────────────────

fn run_tui(storage: Box<dyn StorageBackend>, config: Config, skip_splash: bool) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = app::App::new(storage, config)?;
    if skip_splash {
        app.mode = app::Mode::Normal;
    }
    let events = EventHandler::new(Duration::from_millis(80));

    while app.running {
        terminal.draw(|f| ui::render(f, &app))?;

        app.tick = app.tick.wrapping_add(1);

        match events.next()? {
            Event::Key(key) => app.handle_key(key),
            Event::Tick => app.auto_save(),
            Event::Resize => {}
        }
    }

    app.auto_save();

    disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }

    #[test]
    fn test_cli_open() {
        let args = Cli::try_parse_from(&["canban", "-o", "myboard"]).unwrap();
        assert_eq!(args.open, Some("myboard".to_string()));
        assert!(args.command.is_none());
    }

    #[test]
    fn test_cli_add() {
        let args = Cli::try_parse_from(&["canban", "add", "my task", "-b", "board1", "-c", "doing"]).unwrap();
        if let Some(Commands::Add { title, board, column, tags, due }) = args.command {
            assert_eq!(title, "my task");
            assert_eq!(board, Some("board1".to_string()));
            assert_eq!(column, "doing");
            assert_eq!(tags, None);
            assert_eq!(due, None);
        } else {
            panic!("Expected Add command");
        }
    }

    #[test]
    fn test_cli_new() {
        let args = Cli::try_parse_from(&["canban", "new", "Project X"]).unwrap();
        if let Some(Commands::New { name }) = args.command {
            assert_eq!(name, "Project X");
        } else {
            panic!("Expected New command");
        }
    }
}

