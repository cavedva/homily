use crate::general::Message;

use futures_channel::mpsc::UnboundedSender;
use log::{Record, Level, Metadata};
use std::sync::Mutex;


pub struct StringLogger {
    pub utx: Mutex<UnboundedSender<Message>>,
}

impl log::Log for StringLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let utx = self.utx.lock().unwrap();
            utx.unbounded_send(Message::LogMessage(format!("{} -{}", record.level(), record.args()))).unwrap();
        }
    }

    fn flush(&self) {}
}