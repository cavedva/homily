use crossterm::event::{Event as CTEvent, KeyEvent as CTKeyEvent, KeyCode as CTKey};
use tuikit::{event::Event as TKEvent, key::Key as TKKey};

pub enum KeyMap {
    Episodes,
    Feeds,
    Downloads,
    Log,
    Headers,
    Download,
    Refresh,
    Quit,
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
    Right,
    Left,
    Enter,
    Resize(usize, usize),
}

impl KeyMap {
    pub fn from_tuikit_event(event: TKEvent) -> Option<KeyMap> {
        match event {
            TKEvent::Key(TKKey::Char('e')) => Some(KeyMap::Episodes),
            TKEvent::Key(TKKey::Char('f')) => Some(KeyMap::Feeds),
            TKEvent::Key(TKKey::Char('o')) => Some(KeyMap::Downloads),
            TKEvent::Key(TKKey::Char('l')) => Some(KeyMap::Log),
            TKEvent::Key(TKKey::Char('h')) => Some(KeyMap::Headers),
            TKEvent::Key(TKKey::Char('d')) => Some(KeyMap::Download),
            TKEvent::Key(TKKey::Char('r')) => Some(KeyMap::Refresh),
            TKEvent::Key(TKKey::Char('q')) => Some(KeyMap::Quit),
            TKEvent::Key(TKKey::Up) => Some(KeyMap::Up),
            TKEvent::Key(TKKey::Down) => Some(KeyMap::Down),
            TKEvent::Key(TKKey::PageUp) => Some(KeyMap::PageUp),
            TKEvent::Key(TKKey::PageDown) => Some(KeyMap::PageDown),
            TKEvent::Key(TKKey::Home) => Some(KeyMap::Home),
            TKEvent::Key(TKKey::End) => Some(KeyMap::End),
            TKEvent::Key(TKKey::Right) => Some(KeyMap::Right),
            TKEvent::Key(TKKey::Left) => Some(KeyMap::Left),
            TKEvent::Key(TKKey::Enter) => Some(KeyMap::Enter),
            TKEvent::Key(TKKey::ESC) => Some(KeyMap::Quit),
            TKEvent::Resize { width, height } => Some(KeyMap::Resize(width, height)),
            _ => None,
        }
    }

    pub fn from_crossterm_event(event: CTEvent) -> Option<KeyMap> {
        match event {
            CTEvent::Key(CTKeyEvent { code: CTKey::Char('e'), .. }) => Some(KeyMap::Episodes),
            CTEvent::Key(CTKeyEvent { code: CTKey::Char('f'), .. }) => Some(KeyMap::Feeds),
            CTEvent::Key(CTKeyEvent { code: CTKey::Char('o'), .. }) => Some(KeyMap::Downloads),
            CTEvent::Key(CTKeyEvent { code: CTKey::Char('l'), .. }) => Some(KeyMap::Log),
            CTEvent::Key(CTKeyEvent { code: CTKey::Char('h'), .. }) => Some(KeyMap::Headers),
            CTEvent::Key(CTKeyEvent { code: CTKey::Char('d'), .. }) => Some(KeyMap::Download),
            CTEvent::Key(CTKeyEvent { code: CTKey::Char('r'), .. }) => Some(KeyMap::Refresh),
            CTEvent::Key(CTKeyEvent { code: CTKey::Char('q'), .. }) => Some(KeyMap::Quit),
            CTEvent::Key(CTKeyEvent { code: CTKey::Up, .. }) => Some(KeyMap::Up),
            CTEvent::Key(CTKeyEvent { code: CTKey::Down, .. }) => Some(KeyMap::Down),
            CTEvent::Key(CTKeyEvent { code: CTKey::PageUp, .. }) => Some(KeyMap::PageUp),
            CTEvent::Key(CTKeyEvent { code: CTKey::PageDown, .. }) => Some(KeyMap::PageDown),
            CTEvent::Key(CTKeyEvent { code: CTKey::Home, .. }) => Some(KeyMap::Home),
            CTEvent::Key(CTKeyEvent { code: CTKey::End, .. }) => Some(KeyMap::End),
            CTEvent::Key(CTKeyEvent { code: CTKey::Right, .. }) => Some(KeyMap::Right),
            CTEvent::Key(CTKeyEvent { code: CTKey::Left, .. }) => Some(KeyMap::Left),
            CTEvent::Key(CTKeyEvent { code: CTKey::Enter, .. }) => Some(KeyMap::Enter),
            CTEvent::Key(CTKeyEvent { code: CTKey::Esc, .. }) => Some(KeyMap::Quit),
            CTEvent::Resize(width, height) => Some(KeyMap::Resize(width as usize, height as usize)),
            _ => None,
        }
    }
}

/*
impl From<TKEvent> for Option<KeyMap> {
    fn from(event: TKEvent) -> Option<KeyMap> {
        match event {
            TKKey::Char('e') => Some(KeyMap::Episodes),
            _ => None,
        }
    }
}
impl Into<Option<KeyMap>> for TKEvent {
    fn into(self) -> Option<KeyMap> {
        match event {
            TKKey::Char('e') => Some(KeyMap::Episodes),
            _ => None,
        }
    }
}
*/