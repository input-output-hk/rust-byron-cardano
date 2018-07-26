extern crate pbr;

use super::Term;

use std::{ops::{Deref, DerefMut}, io::{Stdout}};

pub use self::pbr::Units;

enum ProgressType {
    Ticker,
    Bar,
    Quiet
}

pub struct Progress<'a> {
    _term: &'a mut Term,
    bar: pbr::ProgressBar<Stdout>,
    progress_type: ProgressType
}
impl<'a> Progress<'a> {
    pub fn quiet(term: &'a mut Term) -> Self {
        let mut bar = pbr::ProgressBar::new(0);
        bar.show_bar = false;
        bar.show_counter = false;
        bar.show_speed = false;
        bar.show_percent = false;
        bar.show_tick = false;
        bar.show_message = false;
        Progress {
            _term: term,
            bar: bar,
            progress_type: ProgressType::Quiet,
        }
    }

    pub fn new_bar(term: &'a mut Term, count: u64) -> Self {
        let mut bar = pbr::ProgressBar::new(count);
        bar.set_width(Some(term.width));
        bar.format("╢▌▌░╟");
        bar.show_speed = false;
        bar.show_tick = true;
        Progress {
            _term: term,
            bar: bar,
            progress_type: ProgressType::Bar
        }
    }

    pub fn new_tick(term: &'a mut Term, count: u64) -> Self {
        let mut bar = pbr::ProgressBar::new(count);
        bar.set_width(Some(term.width));
        bar.tick_format("▀▐▄▌");
        bar.show_bar = false;
        bar.show_counter = false;
        bar.show_speed = false;
        bar.show_percent = false;
        Progress {
            _term: term,
            bar: bar,
            progress_type: ProgressType::Ticker
        }
    }

    pub fn advance(&mut self, count: u64) {
        match self.progress_type {
            ProgressType::Bar    => { self.bar.add(count);  },
            ProgressType::Ticker => { self.bar.tick(); },
            ProgressType::Quiet  => { },
        }
    }

    pub fn end(mut self) {
        match self.progress_type {
            ProgressType::Bar    => { self.bar.finish_println(""); }
            ProgressType::Ticker => { self.bar.finish_println(""); },
            ProgressType::Quiet  => { self.bar.finish() },
        }
    }
}
impl<'a> Deref for Progress<'a> {
    type Target = pbr::ProgressBar<Stdout>;

    fn deref(&self) -> &Self::Target { &self.bar }
}
impl<'a> DerefMut for Progress<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.bar }
}
