extern crate pbr;

use super::Term;

use std::{ops::{Deref, DerefMut}, io::{Stdout}};

pub use self::pbr::Units;

pub struct Progress<'a> {
    _term: &'a mut Term,
    bar: pbr::ProgressBar<Stdout>,
}
impl<'a> Progress<'a> {

    pub fn new_bar(term: &'a mut Term, count: u64) -> Self {
        let mut bar = pbr::ProgressBar::new(count);
        bar.set_width(Some(term.width));
        bar.format("╢▌▌░╟");
        Progress {
            _term: term,
            bar: bar
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
            bar: bar
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
