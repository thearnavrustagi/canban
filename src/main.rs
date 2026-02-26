mod app;
mod config;
mod event;
mod model;
mod storage;
mod ui;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::config::Config;
use crate::event::{Event, EventHandler};
use crate::storage::json_backend::JsonBackend;
use crate::storage::StorageBackend;

#[derive(Parser)]
#[command(name = "canban", version, about = "A terminal Kanban board with vim bindings")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
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
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();
    let config = Config::load()?;
    let storage = JsonBackend::new();

    match cli.command {
        Some(Commands::Export { output }) => cmd_export(&storage, &config, &output),
        Some(Commands::Import { input }) => cmd_import(&storage, &input),
        Some(Commands::Boards) => cmd_boards(&storage),
        None => run_tui(Box::new(storage), config),
    }
}

fn cmd_export(storage: &JsonBackend, config: &Config, output: &PathBuf) -> Result<()> {
    let board = storage.load_board(&config.active_board)?;
    storage.export_csv(&board, output)?;
    println!("Exported board '{}' to {}", board.name, output.display());
    Ok(())
}

fn cmd_import(storage: &JsonBackend, input: &PathBuf) -> Result<()> {
    let board = storage.import_csv(input)?;
    let name = board.name.clone();
    storage.save_board(&board)?;
    println!("Imported board '{}' from {}", name, input.display());
    Ok(())
}

fn cmd_boards(storage: &JsonBackend) -> Result<()> {
    let boards = storage.list_boards()?;
    if boards.is_empty() {
        println!("No boards found. Run `canban` to create one.");
    } else {
        println!("Boards:");
        for b in boards {
            println!("  - {b}");
        }
    }
    Ok(())
}

fn run_tui(storage: Box<dyn StorageBackend>, config: Config) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = app::App::new(storage, config)?;
    let events = EventHandler::new(Duration::from_millis(250));

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
