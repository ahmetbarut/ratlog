# ratlog — Log Viewer and Filtering TUI

A [Ratatui] TUI application to open log files in the terminal with **live text filtering**, **live follow** (tail -f style), and **memory usage** display.

## Installation

**From source (recommended):**

```bash
# Clone the repository
git clone https://github.com/yourusername/ratlog.git
cd ratlog

# Option 1: Use the install script
./install.sh

# Option 2: Manual install
cargo install --path .
```

After installation, run from anywhere:

```bash
ratlog                    # Sample logs
ratlog /var/log/app.log   # Open a log file
```

**From the project directory (without installing):**

```bash
cargo build --release
./target/release/ratlog -- log.log
```

## Features

- **Log viewing:** Run with a file or with sample logs
- **Live filter:** Case-insensitive instant text filter
- **Live mode (L/F):** Automatically show new lines appended to the file
- **RAM display:** Current process memory usage (MiB/KiB) in the status bar
- **Memory limit:** At most 150 lines kept; last 150 lines used for file and filter
- **Settings (S):** Colours and text style: accent, text colour, text style (Normal/Bold/Dim), border colour, status bar colour

## Requirements

- [Rust](https://www.rust-lang.org/) (1.70+ recommended; install via `rustup`)

## Build (development)

```bash
cargo build
cargo build --release
```

## Usage

```bash
# Run with sample logs (no file)
ratlog
# or: cargo run

# Run with a log file (last 150 lines are loaded)
ratlog log.log
# or: cargo run -- log.log
```

**Example scenario (live log):**

```bash
# Terminal 1: Application that produces logs (e.g. php log.php)
php log.php

# Terminal 2: Open the log file with this app, press L to enable live mode
ratlog log.log
# After it opens, press L or F
```

## Controls

| Key | Action |
|-----|--------|
| **Tab** / **/** / **Ctrl+F** | Focus filter field |
| **S** | Open Settings (theme and accent colour) |
| **L** / **F** | Toggle live mode (only when loaded from file) |
| **Esc** (in filter) | Clear filter; quit when empty |
| **q** / **Ctrl+C** | Quit |
| **j** / **↓** | Next line |
| **k** / **↑** | Previous line |
| **Page Up** / **Page Down** | Page scroll |
| **Home** / **g** | Go to first line (top) |
| **End** / **G** | Go to last line (bottom) |

**In Settings:** **↑/↓** or **j/k** to move, **←/→** to change the selected option, **Enter** on “Back” or **Esc** to close.

- While in the filter field, typed text filters the list instantly; the **last 150 matches** are shown.
- With live mode on, new lines appended to the file appear automatically and the list scrolls to the end.
- Each log line is shown with its **file line number** on the left (e.g. `   324 │ [2025-02-15 10:00:00] INFO ...`).

## Settings (colours and text style)

Press **S** to open the settings panel.

- **Accent:** **Cyan**, **Green**, **Yellow**, **Magenta**, **Blue** — filter field when focused and selected log line highlight.
- **Text colour:** **White**, **Gray**, **Cyan**, **Green**, **Yellow** — colour of log lines.
- **Text style:** **Normal**, **Bold**, **Dim** — style of log line text.
- **Border colour:** **White**, **Gray**, **Dark** — colour of block borders (Filter, Logs).
- **Status bar colour:** **Gray**, **Dark**, **White** — colour of the bottom status bar text.
- **Back** — close settings.

Use **←/→** on a row to change the value; **Enter** on “Back” or **Esc** to close.

**Font (typeface and size):** This is a terminal (TUI) app. The **font family and font size** are chosen in your **terminal emulator** (e.g. Terminal.app, iTerm2, Alacritty). Use your terminal’s preferences to pick a system font (e.g. Fira Code, JetBrains Mono) and size; the app cannot list or change fonts itself.

## Memory (RAM) behaviour

- At most **150 lines** are kept in memory (`MAX_LINES`).
- When opening a file, only the **last 150 lines** are loaded.
- When filtering, the **last 150 matching lines** are listed.
- The status bar shows **RAM: X.X MiB** for the current process memory usage.

## Technologies and libraries used

| Component | Version | Description |
|-----------|---------|-------------|
| **Rust** | 2024 edition | Programming language |
| **[ratatui]** | 0.30 | Terminal UI (TUI); widgets, layout, styling |
| **[crossterm]** | 0.28 | Terminal input (keyboard/mouse), event stream |
| **[tokio]** | 1.40 | Async runtime; timer and event loop in live mode |
| **[futures]** | 0.3 | Async stream (EventStream) support |
| **[sysinfo]** | 0.31 | Process info; memory (RSS) measurement |
| **[color-eyre]** | 0.6 | Error reporting (coloured, detailed) |

The whole UI (list, filter box, status bar) is drawn with **ratatui**; keyboard events come from **crossterm**, and **tokio** drives the async event loop and periodic file reads in live mode.

## Project structure

```
ratlog/
├── Cargo.toml      # Dependencies and build settings
├── install.sh      # Install script (./install.sh)
├── README.md       # This file
├── LICENSE        # MIT
└── src/
    └── main.rs    # Application code (single file)
```

## License

Copyright (c) Ahmet Barut <ahmetbarut588@gmail.com>

This project is licensed under the MIT license ([LICENSE](./LICENSE) or <http://opensource.org/licenses/MIT>).

---

[Ratatui]: https://ratatui.rs  
[ratatui]: https://github.com/ratatui-org/ratatui  
[crossterm]: https://github.com/crossterm-rs/crossterm  
[tokio]: https://tokio.rs  
[futures]: https://github.com/rust-lang/futures-rs  
[sysinfo]: https://github.com/GuillaumeGomez/sysinfo  
[color-eyre]: https://github.com/eyreists/color-eyre  
