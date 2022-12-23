use crate::general::{Status, Thing, ThingList, Styled};
use crate::keymap::KeyMap;

use std::io::{stdout, Stdout, Write};
use std::time;
use crossterm::{
    style::{self, Stylize},
    QueueableCommand, terminal, cursor,
    event::{poll, read},
};


pub struct TermAdapter {
    pub term: Stdout,
}

impl TermAdapter {
    pub fn size(&self) -> (usize, usize) {
        let s = terminal::size().unwrap();
        (s.0 as usize, s.1 as usize)
    }

    pub fn clear(&self) {
        terminal::disable_raw_mode();
        stdout().queue(terminal::Clear(terminal::ClearType::All)).unwrap();
        stdout().queue(cursor::MoveTo(0, 0)).unwrap();
        stdout().queue(cursor::Show).unwrap();
        stdout().flush().unwrap();
    }
    
    pub fn peek_key(&self) -> Option<KeyMap> {
        match poll(time::Duration::from_millis(5)) {
            Ok(true) => {
                match read() {
                    Ok(ev) => KeyMap::from_crossterm_event(ev),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    pub fn update(&mut self, dtlist: &ThingList<Thing>, status: &Status, height: usize, width: usize) {
        let mut stdout = stdout();
        stdout.queue(cursor::MoveTo(0, 0)).unwrap();
        let offset = dtlist.selected_index as i64 - height as i64 + 2_i64;
        let start = if offset < 0 { 0 } else { offset as usize };
        let end = if dtlist.things.len() > start + height - 1 { start + height - 1 } else { dtlist.things.len() };
        for (index, thing) in dtlist.things[start..end].iter().enumerate() {
            stdout.queue(cursor::MoveToColumn(0)).unwrap();
            stdout.queue(terminal::Clear(terminal::ClearType::CurrentLine)).unwrap();
            let mut sc = thing.crossterm_styles(width);
            if start + index == dtlist.selected_index {
                sc = sc.blue();
            }
            stdout.queue(style::PrintStyledContent(sc)).unwrap();
            stdout.queue(cursor::MoveDown(1)).unwrap();
            stdout.flush().unwrap();
            if index >= height - 2 {
                break;
            }
        }
        if let Ok((_, mut row)) = cursor::position() {
            while (row as usize) < height - 1 {
                stdout.queue(cursor::MoveTo(0, row)).unwrap();
                stdout.queue(terminal::Clear(terminal::ClearType::CurrentLine)).unwrap();
                row += 1;
            }
            stdout.flush().unwrap();
        }
        stdout.flush().unwrap();
    }

    pub fn update_status(&mut self, dtlist: &ThingList<Thing>, status: &Status, height: usize, width: usize) {
        let mut stdout = stdout();
        stdout.queue(cursor::MoveTo(0, (height - 1) as u16)).unwrap();
        let mut s = status.0.clone();
        let s = match s.char_indices().nth(width) {
            None => s,
            Some((idx, _)) => s[..idx].to_string(),
        };
        stdout.queue(style::PrintStyledContent(format!("{:width$}", s, width=width).black().on_white())).unwrap();
        stdout.flush().unwrap();
    }
}

pub fn get_term() -> TermAdapter {
    let ta = TermAdapter { term: stdout() };
    terminal::enable_raw_mode();
    stdout().queue(terminal::Clear(terminal::ClearType::All)).unwrap();
    stdout().queue(cursor::Hide).unwrap();
    stdout().flush().unwrap();
    /*

    redraw().unwrap();

    stdout.queue(cursor::MoveTo(2, 2)).unwrap();
    stdout.queue(style::PrintStyledContent( "test".blue())).unwrap();
    stdout.flush();
    */
    ta
}