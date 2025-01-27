use std::io::{stdout, Stdout};

use rand::Rng;
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::event::Event;

pub struct App {
    stdout: Stdout,
    quote: Vec<String>,
    event_tx: Sender<Event>,
    event_rx: Receiver<Event>,
}

impl App {
    pub fn new(quotes: &[Vec<String>]) -> App {
        let mut rng = rand::thread_rng();
        let chosen = rng.gen_range(0..quotes.len());
        let (event_tx, event_rx): (Sender<Event>, Receiver<Event>) = channel(10);
        App {
            stdout: stdout(),
            quote: quotes[chosen].clone(),
            event_rx,
            event_tx,
        }
    }

    pub fn run() {
        todo!()
    }
}
