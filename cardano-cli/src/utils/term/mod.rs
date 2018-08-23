//! terminal helpers
//!

mod config;
pub mod emoji;

use console;
use indicatif;
use dialoguer;

pub use self::config::{Config, ColorChoice};

use std::{io::{self, Write}};

pub const DEFAULT_TERM_WIDTH : usize = 80;
pub const DEFAULT_TERM_HEIGHT: usize = 24;

pub struct Style {
    pub error:   console::Style,
    pub warning: console::Style,
    pub success: console::Style,
    pub info:    console::Style,
}
impl Style {
    fn new(color_choice: &ColorChoice) -> Self {
        match color_choice {
            ColorChoice::Auto   => { /* do nothing */ },
            ColorChoice::Never  => { console::set_colors_enabled(false) },
            ColorChoice::Always => { console::set_colors_enabled(true) },
        };

        Style {
            error:   console::Style::new().red().bold(),
            warning: console::Style::new().red(),
            success: console::Style::new().green(),
            info:    console::Style::new().cyan().italic(),
        }
    }
}

pub struct Term {
    /// the user's configuration of the terminal
    pub config: Config,

    pub style: Style,

    pub term: console::Term
}
impl Term {
    pub fn new(config: Config) -> Self {
        // make sure we are using a terminal for now
        if ! console::user_attended() {
            panic!(
                "We only support terminal"
            );
        }

        let term = console::Term::stdout();
        let style  = Style::new(&config.color);

        Term { config, term, style }
    }

    pub fn progress_bar(&self, count: u64) -> indicatif::ProgressBar {
        let pb = indicatif::ProgressBar::new(count);
        pb.enable_steady_tick(100);
        pb.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                .progress_chars("#>-")
        );
        pb
    }

    pub fn prompt(&mut self, prompt: &str) -> io::Result<String> {
        dialoguer::Input::new(prompt).interact()
    }

    pub fn password(&mut self, prompt: &str) -> io::Result<String> {
        #[cfg(windows)]
        {
            // TODO: there seems to be an issue with rust crate: console
            //       the password read line is not working or not returning
            //       at all on windows 10 's `cmd` or `PowerShell`
            let line = dialoguer::Input::new(prompt).default("").interact()?;
            self.term.move_cursor_up(1)?;
            self.term.clear_line()?;
            Ok(line)
        }
        #[cfg(not(windows))]
        {
            dialoguer::PasswordInput::new(prompt).allow_empty_password(true).interact()
        }
    }

    pub fn new_password(&mut self, prompt: &str, confirmation: &str, mismatch_err: &str) -> io::Result<String> {
        #[cfg(windows)]
        {
            loop {
                // TODO: there seems to be an issue with rust crate: console
                //       the password read line is not working or not returning
                //       at all on windows 10 's `cmd` or `PowerShell`
                let line = dialoguer::Input::new(prompt).default("").interact()?;
                self.term.move_cursor_up(1)?;
                self.term.clear_line()?;
                let line2 = dialoguer::Input::new(confirmation).default("").interact()?;
                self.term.move_cursor_up(1)?;
                self.term.clear_line()?;
                if line == line2 {
                    return Ok(line)
                }
                self.error(mismatch_err)?;
            }
        }
        #[cfg(not(windows))]
        {
            dialoguer::PasswordInput::new(prompt)
                .allow_empty_password(true)
                .confirm(confirmation, mismatch_err)
                .interact()
        }
    }

    pub fn simply(&mut self, msg: &str) -> io::Result<()> {
        write!(self, "{}", msg)
    }
    pub fn success(&mut self, msg: &str) -> io::Result<()> {
        write!(&mut self.term, "{}", self.style.success.apply_to(msg))
    }
    pub fn info(&mut self, msg: &str) -> io::Result<()> {
        write!(&mut self.term, "{}", self.style.info.apply_to(msg))
    }
    pub fn warn(&mut self, msg: &str) -> io::Result<()> {
        write!(&mut self.term, "{}", self.style.warning.apply_to(msg))
    }
    pub fn error(&mut self, msg: &str) -> io::Result<()> {
        write!(&mut self.term, "{}", self.style.error.apply_to(msg))
    }
}
impl ::std::ops::Deref for Term {
    type Target = console::Term;
    fn deref(&self) -> &Self::Target { &self.term }
}
impl ::std::ops::DerefMut for Term {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.term }
}
impl io::Write for Term {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        io::Write::write(&mut self.term, buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        io::Write::flush(&mut self.term)
    }
}
impl io::Read for Term {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        io::Read::read(&mut self.term, buf)
    }
}
