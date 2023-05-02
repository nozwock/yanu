use std::sync::mpsc;

pub mod gui;
pub mod utils;

#[derive(Debug)]
pub struct MpscChannel<T> {
    pub tx: mpsc::Sender<T>,
    pub rx: mpsc::Receiver<T>,
}

impl<T> Default for MpscChannel<T> {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self { tx, rx }
    }
}
