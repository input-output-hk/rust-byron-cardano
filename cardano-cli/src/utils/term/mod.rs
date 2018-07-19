//! terminal helpers
//!

extern crate termcolor;
extern crate term_size;

mod config;
mod progress_bar;

pub use self::config::{Config};
pub use self::progress_bar::{Progress, Units};

use std::{io::{self, Write}};
use self::termcolor::{StandardStream, Color, ColorSpec, WriteColor};

pub use self::termcolor::ColorChoice;

pub const DEFAULT_TERM_WIDTH : usize = 80;
pub const DEFAULT_TERM_HEIGHT: usize = 24;

pub struct Term {
    /// the terminal's width
    pub width: usize,
    /// the terminal's height
    pub height: usize,
    /// the user's configuration of the terminal
    pub config: Config,

    stdout: StandardStream,
    stderr: StandardStream,
}
impl Term {
    pub fn new(config: Config) -> Self {
        let (width, height) = if let Some((w, h)) = term_size::dimensions() {
            (w, h) } else { (DEFAULT_TERM_WIDTH, DEFAULT_TERM_HEIGHT)
        };
        let stdout = StandardStream::stdout(config.color);
        let stderr = StandardStream::stderr(config.color);
        Term { width, height, config, stdout, stderr }
    }

    /// create a progress bar configured for byte streaming progress.
    ///
    pub fn progress_download<'a>(&'a mut self, count: u64) -> Progress<'a> {
        if self.config.quiet { return Progress::quiet(self); }
        let mut b = Progress::new_bar(self, count);
        b.set_units(Units::Bytes);
        b.show_speed = true;
        b
    }
    /// create a progress bar configured not to display bytes related data
    ///
    pub fn progress_bar<'a>(&'a mut self, count: u64) -> Progress<'a> {
        if self.config.quiet { return Progress::quiet(self); }
        Progress::new_bar(self, count)
    }
    /// configure a spinning progress display, no progress bar, only
    /// a ticking display.
    pub fn progress_tick<'a>(&'a mut self) -> Progress<'a> {
        if self.config.quiet { return Progress::quiet(self); }
        Progress::new_tick(self, 0)
    }

    pub fn success(&mut self, msg: &str) -> io::Result<()> {
        if self.config.quiet { return Ok(()); }
        let mut out = self.stdout.lock();

        out.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
        write!(&mut out, "{}", msg)?;
        out.reset()
    }
    pub fn info(&mut self, msg: &str) -> io::Result<()> {
        if self.config.quiet { return Ok(()); }
        let mut out = self.stdout.lock();

        out.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)))?;
        write!(&mut out, "{}", msg)?;
        out.reset()
    }
    pub fn error(&mut self, msg: &str) -> io::Result<()> {
        if self.config.quiet { return Ok(()); }
        let mut out = self.stderr.lock();

        out.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
        write!(&mut out, "{}", msg)?;
        out.reset()
    }
}
