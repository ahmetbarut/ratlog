//! CLI: version, help, and argument parsing.

use std::path::PathBuf;

use crate::constants::MAX_LINES;

const VERSION: &str = match option_env!("RATLOG_VERSION") {
    Some(v) => v,
    None => env!("CARGO_PKG_VERSION"),
};

#[derive(Debug)]
pub enum CliAction {
    Run(Option<PathBuf>),
    Login,
}

pub fn print_version() {
    println!("ratlog {}", VERSION);
}

pub fn print_help() {
    println!(
        r#"ratlog {} — Terminal log viewer with live filtering and tail-style follow

USAGE:
    ratlog [OPTIONS] [LOG_FILE]
    ratlog login

ARGUMENTS:
    LOG_FILE    Log file to open (last {} lines shown). If omitted, sample logs are used.

COMMANDS:
    login       Log in to Ratlog Web (opens browser, saves token for log sharing)

OPTIONS:
    -h, --help      Show this message and exit
    -V, --version   Show version and exit

CONTROLS (in app):
    / or Tab or Ctrl+F   Focus filter
    S                    Settings (colours)
    L or F               Toggle live mode (when viewing a file)
    P                    Share logs to Ratlog Web (requires login)
    g / G                Go to first / last line
    q or Ctrl+C          Quit

https://github.com/ahmetbarut/ratlog
"#,
        VERSION, MAX_LINES
    );
}

/// Parse args: exits with 0 for -h/--version; otherwise returns CliAction.
pub fn parse_args(args: &[String]) -> CliAction {
    if args.iter().skip(1).any(|a| a == "-h" || a == "--help") {
        print_help();
        std::process::exit(0);
    }
    if args.iter().skip(1).any(|a| a == "-V" || a == "--version") {
        print_version();
        std::process::exit(0);
    }
    let positional: Vec<&String> = args
        .iter()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .collect();
    if positional.first().map(|s| s.as_str()) == Some("login") {
        return CliAction::Login;
    }
    let file_arg = positional.first().map(|s| PathBuf::from(s.as_str()));
    CliAction::Run(file_arg)
}
