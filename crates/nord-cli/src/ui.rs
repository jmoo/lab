//! Presentation gating: colour, unicode, and which stream a line belongs on.
//!
//! Three rules, all of them load-bearing rather than cosmetic:
//!
//! - **Data on stdout, chatter on stderr.** `nord program get 7:4 | grep transpose` must
//!   see the summary and nothing else, so every progress line, warning and pre-flight
//!   description goes to stderr.
//! - **Colour and unicode only on a TTY.** The cross-platform check compares the bytes a
//!   Wine-hosted `nord.exe` and the native Linux binary print for the same input; an
//!   escape sequence or a box-drawing character that survives a pipe would put that
//!   comparison at the mercy of Wine's console codepage.
//! - **A non-TTY is non-interactive.** Never read a stdin nobody is attached to, and
//!   never auto-proceed because nobody is there to say no.

use std::fmt::Display;
use std::io::{BufRead, IsTerminal, Write};

/// When to emit ANSI colour.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum ColorChoice {
    /// Colour when stdout is a terminal and `NO_COLOR` is unset.
    #[default]
    Auto,
    Always,
    Never,
}

/// What this invocation may print, and to whom.
#[derive(Copy, Clone, Debug)]
pub struct Ui {
    color: bool,
    unicode: bool,
    interactive: bool,
}

impl Ui {
    pub fn new(choice: ColorChoice) -> Self {
        let tty = std::io::stdout().is_terminal();
        let color = match choice {
            ColorChoice::Always => true,
            ColorChoice::Never => false,
            // `NO_COLOR` is honoured whatever its value; the convention is that the
            // variable being present at all is the signal.
            ColorChoice::Auto => {
                tty && std::env::var_os("NO_COLOR").is_none()
                    && std::env::var("TERM").as_deref() != Ok("dumb")
            }
        };
        Ui {
            color,
            // Deliberately not tied to `--color`: someone forcing colour into a file
            // still gets a file, and `nord.exe`'s console is the case that breaks.
            unicode: tty,
            // stderr as well as stdin: the prompt is written to stderr, so a redirected
            // stderr means the question would never be seen.
            interactive: std::io::stdin().is_terminal() && std::io::stderr().is_terminal(),
        }
    }

    /// Data. Goes to stdout, and is the only thing that does.
    ///
    /// ⚠️ Not `println!`, which **panics** when the reader goes away: `nord program edit
    /// --fields | head` would print a Rust backtrace over the user's terminal. A closed
    /// pipe is the reader saying it has enough, so it exits successfully and silently,
    /// the way every other line-oriented tool does.
    pub fn out(&self, line: impl Display) {
        let mut stdout = std::io::stdout().lock();
        if let Err(e) = writeln!(stdout, "{line}") {
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                std::process::exit(0);
            }
            eprintln!("writing to stdout: {e}");
            std::process::exit(1);
        }
    }

    /// Chatter — progress, pre-flight descriptions, anything a pipe should not see.
    pub fn note(&self, line: impl Display) {
        eprintln!("{line}");
    }

    pub fn warn(&self, line: impl Display) {
        eprintln!("{}{line}", self.style("warning: ", YELLOW));
    }

    /// An em dash on a terminal, a hyphen everywhere else.
    pub fn dash(&self) -> &'static str {
        if self.unicode {
            "—"
        } else {
            "-"
        }
    }

    /// Whether box-drawing and block glyphs may be used. Callers that substitute a glyph
    /// for data must keep the plain form byte-identical, so a pipe stays as informative
    /// as it was.
    pub fn unicode(&self) -> bool {
        self.unicode
    }

    /// A section heading inside a summary. Red, for the obvious reason.
    ///
    /// Colour carries exactly three meanings in this CLI and no more: a heading here, a
    /// dimmed label or inactive value, and [`Ui::danger`] for something about to be
    /// destroyed. Adding a fourth is how output stops reading as a system.
    ///
    /// ⚠️ A heading and a danger are both red, separated only by weight. They stay
    /// distinguishable because they never share a stream — headings are data on stdout,
    /// dangers are chatter on stderr immediately above a prompt. Putting a heading on
    /// stderr, or a danger inside a summary, collapses that distinction.
    pub fn heading(&self, s: impl Display) -> String {
        self.style(s, BOLD_RED)
    }

    pub fn bold(&self, s: impl Display) -> String {
        self.style(s, BOLD)
    }

    pub fn dim(&self, s: impl Display) -> String {
        self.style(s, DIM)
    }

    pub fn danger(&self, s: impl Display) -> String {
        self.style(s, RED)
    }

    fn style(&self, s: impl Display, code: &str) -> String {
        if self.color {
            format!("\x1b[{code}m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }

    /// Settle a destructive action: `--yes` given, or the operator says so at a prompt.
    ///
    /// ⚠️ The caller must already have described what will be touched. This asks the
    /// question; it does not state the stakes.
    ///
    /// On a non-TTY the missing `--yes` stays a hard error rather than becoming a block
    /// on a stdin that may never produce a line. That is what makes a `--no-interaction`
    /// flag unnecessary: a pipe *is* non-interactive.
    pub fn confirm(&self, already: bool) -> Result<(), String> {
        if already {
            return Ok(());
        }
        if !self.interactive {
            return Err(format!(
                "refusing to proceed without {}",
                self.bold("--yes")
            ));
        }
        eprint!("{} [y/N] ", self.danger("proceed?"));
        std::io::stderr().flush().ok();
        let mut answer = String::new();
        std::io::stdin()
            .lock()
            .read_line(&mut answer)
            .map_err(|e| format!("reading the answer: {e}"))?;
        match answer.trim() {
            "y" | "Y" | "yes" => Ok(()),
            _ => Err("cancelled".into()),
        }
    }
}

const BOLD: &str = "1";
const BOLD_RED: &str = "1;31";
const DIM: &str = "2";
const RED: &str = "31";
const YELLOW: &str = "33";

#[cfg(test)]
mod tests {
    use super::*;

    /// A `Ui` built for a pipe styles nothing, whatever it is asked to style.
    #[test]
    fn without_a_tty_nothing_is_decorated() {
        let ui = Ui {
            color: false,
            unicode: false,
            interactive: false,
        };
        assert_eq!(ui.bold("x"), "x");
        assert_eq!(ui.danger("x"), "x");
        assert_eq!(ui.heading("x"), "x");
        assert_eq!(ui.dim("x"), "x");
        assert_eq!(ui.dash(), "-");
        assert!(!ui.unicode());
    }

    #[test]
    fn with_colour_the_reset_always_closes_the_sequence() {
        let ui = Ui {
            color: true,
            unicode: true,
            interactive: false,
        };
        assert_eq!(ui.bold("x"), "\x1b[1mx\x1b[0m");
        assert_eq!(ui.heading("x"), "\x1b[1;31mx\x1b[0m");
        assert_eq!(ui.dash(), "—");
        assert!(ui.unicode());
    }

    /// The non-interactive refusal is the whole safety story for scripts, so it must not
    /// depend on reaching a prompt.
    #[test]
    fn a_pipe_without_yes_is_refused_rather_than_asked() {
        let ui = Ui {
            color: false,
            unicode: false,
            interactive: false,
        };
        assert!(ui.confirm(true).is_ok());
        let err = ui.confirm(false).unwrap_err();
        assert!(err.contains("--yes"), "{err}");
    }
}
