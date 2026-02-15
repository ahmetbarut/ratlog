//! CLI: version, help, and argument parsing.

use std::path::PathBuf;

use crate::constants::MAX_LINES;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn print_version() {
    println!("ratlog {}", VERSION);
}

pub fn print_help() {
    println!(
        r#"ratlog {} — Terminal log viewer with live filtering and tail-style follow

USAGE:
    ratlog [OPTIONS] [LOG_FILE]

ARGUMENTS:
    LOG_FILE    Log file to open (last {} lines shown). If omitted, sample logs are used.

OPTIONS:
    -h, --help      Show this message and exit
    -V, --version   Show version and exit

CONTROLS (in app):
    / or Tab or Ctrl+F   Focus filter
    S                    Settings (colours)
    L or F               Toggle live mode (when viewing a file)
    g / G                Go to first / last line
    q or Ctrl+C          Quit

https://github.com/ahmetbarut/ratlog
"#,
        VERSION,
        MAX_LINES
    );
}

/// Parse args: exits with 0 for -h/--version; otherwise returns optional log file path (None = use sample logs).
pub fn parse_args(args: &[String]) -> Option<PathBuf> {
    if args.iter().skip(1).any(|a| a == "-h" || a == "--help") {
        print_help();
        std::process::exit(0);
    }
    if args.iter().skip(1).any(|a| a == "-V" || a == "--version") {
        print_version();
        std::process::exit(0);
    }
    args.iter()
        .skip(1)
        .find(|a| *a != "-h" && *a != "--help" && *a != "-V" && *a != "--version")
        .map(|s| PathBuf::from(s.as_str()))
}
