use std::io::Write;

use clap::builder::styling::{AnsiColor, Color, Style, Styles};

pub const CYAN: &str = "\x1b[38;2;80;200;220m";
pub const YELLOW: &str = "\x1b[38;2;240;200;80m";
pub const GREEN: &str = "\x1b[38;2;80;220;140m";
pub const RED: &str = "\x1b[38;2;240;90;90m";
pub const ARCHIVE: &str = "\x1b[38;2;140;130;170m";
pub const DIM: &str = "\x1b[38;2;140;140;160m";
pub const BRIGHT: &str = "\x1b[38;2;230;230;240m";
pub const BOLD: &str = "\x1b[1m";
pub const R: &str = "\x1b[0m";

pub fn styles() -> Styles {
    Styles::styled()
        .header(ansi(AnsiColor::Cyan, true))
        .usage(ansi(AnsiColor::Cyan, true))
        .literal(ansi(AnsiColor::Green, false))
        .placeholder(ansi(AnsiColor::Yellow, false))
        .valid(ansi(AnsiColor::Cyan, false))
}

fn ansi(color: AnsiColor, bold: bool) -> Style {
    let s = Style::new().fg_color(Some(Color::Ansi(color)));
    if bold { s.bold() } else { s }
}

pub fn header(section: &str) {
    println!("\n  {BOLD}{CYAN}canban{R} {DIM}·{R} {BRIGHT}{section}{R}\n");
}

pub fn success(msg: &str) {
    println!("  {GREEN}✓{R} {msg}");
}

pub fn error(msg: &str) {
    eprintln!("\n  {RED}✗{R} {msg}\n");
}

pub fn fail(msg: &str) -> ! {
    error(msg);
    std::process::exit(1);
}

pub fn kv(key: &str, val: &str) {
    println!("  {DIM}{key:<18}{R} {BRIGHT}{val}{R}");
}

pub fn board_line(name: &str, active: bool) {
    if active {
        println!("  {CYAN}{BOLD}▸ {name}{R}  {DIM}(active){R}");
    } else {
        println!("    {BRIGHT}{name}{R}");
    }
}

pub fn board_title(name: &str) {
    println!("  {BOLD}{CYAN}{name}{R}");
}

pub fn column_row(kind: &str, count: usize) {
    let icon = col_icon(kind);
    let color = col_color(kind);
    let noun = if count == 1 { "task" } else { "tasks" };
    println!("  {color}{icon}{R} {BRIGHT}{kind:<10}{R} {DIM}{count} {noun}{R}");
}

pub fn separator() {
    println!("  {DIM}─────────────────────────────{R}");
}

pub fn count_line(n: usize, label: &str) {
    println!("  {DIM}{n} {label}{R}");
}

pub fn tag_list(tags: &[String]) {
    if !tags.is_empty() {
        println!("    {DIM}Tags:{R} {CYAN}{}{R}", tags.join(", "));
    }
}

pub fn due_line(date: &str) {
    println!("    {DIM}Due:{R}  {YELLOW}{date}{R}");
}

pub fn hint(msg: &str) {
    println!("  {DIM}{msg}{R}");
}

pub fn confirm(prompt: &str) -> bool {
    print!("  {prompt} {DIM}[y/N]{R} ");
    std::io::stdout().flush().ok();
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf).ok();
    buf.trim().eq_ignore_ascii_case("y")
}

pub fn col_icon(kind: &str) -> &'static str {
    match kind {
        "Ready" => "◇",
        "Doing" => "▸",
        "Done" => "✓",
        "Archived" => "◌",
        _ => "·",
    }
}

pub fn col_color(kind: &str) -> &'static str {
    match kind {
        "Ready" => CYAN,
        "Doing" => YELLOW,
        "Done" => GREEN,
        "Archived" => ARCHIVE,
        _ => DIM,
    }
}
