use std::{
    error::Error,
    io::{stdout, Stdout},
    time::Duration,
};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    style::{Color, SetForegroundColor},
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
    ExecutableCommand,
};
use tokio::{
    spawn,
    sync::mpsc::{channel, Receiver, Sender},
};

use crate::{
    event::{handle_input, Event},
    state::State,
};

pub const TICK_RATE: u64 = 1000 / 20;

pub struct App {
    stdout: Stdout,
    pub event_tx: Sender<Event>,
    event_rx: Receiver<Event>,
    running: bool,
    quote: Vec<String>,
    state: State,
    should_render: bool,
}

impl App {
    pub fn new(quote: String) -> App {
        let (event_tx, event_rx): (Sender<Event>, Receiver<Event>) = channel(10);
        App {
            stdout: stdout(),
            quote: quote
                .split_whitespace()
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect(),
            event_rx,
            event_tx,
            running: false,
            state: State::default(),
            should_render: true,
        }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        self.stdout.execute(EnterAlternateScreen)?.execute(Hide)?;
        enable_raw_mode()?;

        let (input_ks_tx, input_ks_rx): (Sender<()>, Receiver<()>) = channel(1);
        let ev = self.event_tx.clone();
        spawn(async {
            start_input_handler(ev, input_ks_rx).await;
        });

        let (tick_ks_tx, tick_ks_rx): (Sender<()>, Receiver<()>) = channel(1);
        let ev = self.event_tx.clone();
        spawn(async {
            start_tick_generator(ev, tick_ks_rx).await;
        });

        self.running = true;
        while self.running {
            self.process().await?;
        }
        input_ks_tx.send(()).await?;
        tick_ks_tx.send(()).await?;

        disable_raw_mode()?;
        self.stdout.execute(Show)?.execute(LeaveAlternateScreen)?;
        return Ok(());
    }

    async fn process(&mut self) -> Result<(), Box<dyn Error>> {
        let event = self.event_rx.recv().await.unwrap();
        match event {
            Event::Terminate => {
                self.running = false;
            }
            Event::KeyPress(k) => self.handle_keypress(k).await,
            Event::Backspace => self.handle_backspace().await,
            Event::Render => self.render().await,
        }

        if event != Event::Render {
            self.should_render = true;
        }
        return Ok(());
    }

    async fn handle_keypress(&mut self, k: char) {
        if self.state.buffer == self.quote[self.state.current] && k == ' ' {
            self.state.buffer.clear();
            self.state.current += 1;
            if self.state.current == self.quote.len() {
                // TODO Remove running set and set state to StateEnum::Result instead
                self.running = false;
            }
        } else {
            self.state.buffer.push(k);
        }
    }

    async fn handle_backspace(&mut self) {
        if !self.state.buffer.is_empty() {
            self.state.buffer.pop();
        }
    }

    async fn render(&mut self) {
        if !self.should_render {
            return;
        }

        self.stdout
            .execute(Clear(ClearType::All))
            .unwrap()
            .execute(SetForegroundColor(Color::Green))
            .unwrap()
            .execute(MoveTo(0, 0))
            .unwrap();
        let done = self.quote[..self.state.current].join(" ");
        print!("{}", done);

        if !done.trim().is_empty() {
            print!(" ");
        }

        for i in 0..self.state.buffer.len() {
            if i >= self.quote[self.state.current].len() {
                break;
            }

            let c = self.quote[self.state.current].chars().nth(i).unwrap();
            if self.state.buffer.chars().nth(i).unwrap() == c {
                self.stdout
                    .execute(SetForegroundColor(Color::Green))
                    .unwrap();
            } else {
                self.stdout.execute(SetForegroundColor(Color::Red)).unwrap();
            }
            print!("{}", c);
        }

        if self.state.buffer.len() < self.quote[self.state.current].len() {
            self.stdout
                .execute(SetForegroundColor(Color::Reset))
                .unwrap();
            let v = &self.quote[self.state.current][self.state.buffer.len()..];
            print!("{}", v);
        } else if self.state.buffer.len() > self.quote[self.state.current].len() {
            self.stdout
                .execute(SetForegroundColor(Color::Yellow))
                .unwrap();
            let v = &self.state.buffer[self.quote[self.state.current].len()..];
            print!("{}", v);
        }

        print!(" ");

        self.stdout
            .execute(SetForegroundColor(Color::Reset))
            .unwrap();
        let to_do = self.quote[self.state.current + 1..].join(" ");
        println!("{}", to_do);
        self.should_render = false;
    }
}

async fn start_tick_generator(ev: Sender<Event>, mut kill_switch: Receiver<()>) {
    loop {
        tokio::select! {
            _ = async {
                tokio::time::sleep(Duration::from_millis(TICK_RATE)).await;
                ev.send(Event::Render).await
            } => (),
            _ = kill_switch.recv() => return,
        }
    }
}

async fn start_input_handler(ev: Sender<Event>, mut kill_switch: Receiver<()>) {
    loop {
        tokio::select! {
            _ = handle_input(&ev) => (),
            _ = kill_switch.recv() => return,
        }
    }
}
