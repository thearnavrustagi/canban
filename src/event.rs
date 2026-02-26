use color_eyre::eyre::Result;
use crossterm::event::{self, Event as CtEvent, KeyEvent};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub enum Event {
    Key(KeyEvent),
    Resize,
    Tick,
}

pub struct EventHandler {
    rx: mpsc::Receiver<Event>,
    _handle: thread::JoinHandle<()>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || loop {
            if event::poll(tick_rate).unwrap_or(false) {
                let evt = match event::read() {
                    Ok(CtEvent::Key(k)) => Event::Key(k),
                    Ok(CtEvent::Resize(_, _)) => Event::Resize,
                    _ => continue,
                };
                if tx.send(evt).is_err() {
                    break;
                }
            } else if tx.send(Event::Tick).is_err() {
                break;
            }
        });
        Self {
            rx,
            _handle: handle,
        }
    }

    pub fn next(&self) -> Result<Event> {
        Ok(self.rx.recv()?)
    }
}
