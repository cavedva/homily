use crate::general::{Status, Thing, ThingList};
use crate::keymap::KeyMap;

use std::time;
use tuikit::prelude::{VSplit, Win};
use tuikit::term::{Term, TermHeight};
use tuikit::widget::Size;

pub struct TermAdapter {
    pub term: Term,
}

impl TermAdapter {
    pub fn size(&self) -> (usize, usize) {
        self.term.term_size().unwrap()
    }

    pub fn clear(&self) {
        self.term.clear_on_exit(false).unwrap();
    }
    
    pub fn peek_key(&self) -> Option<KeyMap> {
        match self.term.peek_event(time::Duration::from_millis(5)) {
            Ok(ev) => KeyMap::from_tuikit_event(ev),
            _ => None,
        }
    }

    pub fn update(&self, dtlist: &ThingList<Thing>, status: &Status, height: usize, width: usize) {
        let main_win = Win::new(&dtlist);
        let hsplit = VSplit::default()
            .split(main_win.border(true).basis(Size::Fixed(height - 1)))
            .split(Win::new(&status).basis(Size::Fixed(1)));

        let _ = self.term.clear();
        let _ = self.term.draw(&hsplit);
        let _ = self.term.present();
    }

    pub fn update_status(&self, dtlist: &ThingList<Thing>, status: &Status, height: usize, width: usize) {
        self.update(dtlist, status, height, width);
    }
}

pub fn get_term() -> TermAdapter {
    TermAdapter { term: Term::with_height(TermHeight::Percent(100)).unwrap() }
}