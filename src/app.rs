use std::{
    error::Error,
    io::{stdout, Stdout, Write},
    time::Duration,
};

use crossterm::{
    cursor::MoveTo,
    style::{Color, Print, SetForegroundColor},
    terminal::{
        self, disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
    ExecutableCommand, QueueableCommand,
};
use tokio::{
    spawn,
    sync::mpsc::{channel, Receiver, Sender},
    time::Instant,
};

use crate::{
    event::{handle_input, Event},
    state::State,
};

pub const TICK_RATE: u64 = 1000 / 20;

trait Substringable<'a> {
    fn substring(&'a self, start: usize, end: usize) -> Option<&'a str>;
}

impl<'a> Substringable<'a> for str {
    fn substring(&'a self, start: usize, end: usize) -> Option<&'a str> {
        let s = self.char_indices().nth(start);
        let e = self.char_indices().nth(end);
        match (s, e) {
            (None, _) => return None,
            (Some(v), None) => Some(&self[v.0..]),
            (Some(v1), Some(v2)) => Some(&self[v1.0..v2.0]),
        }
    }
}

pub struct App<'a> {
    stdout: Stdout,
    pub event_tx: Sender<Event>,
    event_rx: Receiver<Event>,
    running: bool,
    quote: Vec<&'a str>,
    state: State,
    should_render: bool,
    start: Option<Instant>,
    completed: bool,
}

impl<'a> App<'a> {
    pub fn new(quote: &str) -> App {
        let (event_tx, event_rx): (Sender<Event>, Receiver<Event>) = channel(10);
        App {
            stdout: stdout(),
            quote: quote.split_whitespace().filter(|s| !s.is_empty()).collect(),
            event_rx,
            event_tx,
            running: false,
            state: State::default(),
            should_render: true,
            start: None,
            completed: false,
        }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        self.stdout.execute(EnterAlternateScreen)?;
        enable_raw_mode()?;

        let (input_ks_tx, input_ks_rx): (Sender<()>, Receiver<()>) = channel(1);
        let (tick_ks_tx, tick_ks_rx): (Sender<()>, Receiver<()>) = channel(1);
        spawn(start_input_handler(self.event_tx.clone(), input_ks_rx));
        spawn(start_tick_generator(self.event_tx.clone(), tick_ks_rx));

        self.running = true;
        self.start = Some(Instant::now());
        while self.running {
            self.process().await?;
        }
        let time = self.start.unwrap().elapsed().as_millis();
        let total_chars = self
            .quote
            .iter()
            .map(|w| w.chars().count() as f64)
            .sum::<f64>()
            + self.quote.len() as f64
            - 1.0;
        let wpm = total_chars / 5.0 * 60000.0 / time as f64;

        input_ks_tx.send(()).await?;
        tick_ks_tx.send(()).await?;

        disable_raw_mode()?;
        self.stdout.execute(LeaveAlternateScreen)?;
        if self.completed {
            println!("Your WPM: {}", wpm.round());
        }
        return Ok(());
    }

    async fn process(&mut self) -> Result<(), Box<dyn Error>> {
        let event = self.event_rx.recv().await.unwrap();
        match event {
            Event::Terminate => {
                self.running = false;
            }
            Event::KeyPress(k) => self.handle_keypress(k).await?,
            Event::Backspace => self.handle_backspace().await,
            Event::Render => self.render().await?,
            Event::ForceRender => {
                self.should_render = true;
                self.render().await?;
            }
        }

        if event != Event::Render {
            self.should_render = true;
        }
        return Ok(());
    }

    async fn handle_keypress(&mut self, k: char) -> Result<(), Box<dyn Error>> {
        self.state.buffer.push(k);
        let is_word_completed = self
            .state
            .buffer
            .substring(0, self.state.buffer.chars().count() - 1)
            .unwrap()
            == self.quote[self.state.current];
        let is_text_completed = self.state.buffer == self.quote[self.state.current]
            && self.state.current == self.quote.len() - 1;

        if is_word_completed && k == ' ' {
            self.state.buffer.clear();
            self.state.current += 1;
        } else if is_text_completed {
            self.running = false;
            self.completed = true;
        }
        return Ok(());
    }

    async fn handle_backspace(&mut self) {
        if !self.state.buffer.is_empty() {
            self.state.buffer.pop();
        }
    }

    async fn render(&mut self) -> Result<(), Box<dyn Error>> {
        if !self.should_render {
            return Ok(());
        }

        let buf_size = self.state.buffer.chars().count();
        let cur_word_size = self.quote[self.state.current].chars().count();
        let (col, _) = terminal::size()?;
        self.stdout
            .queue(Clear(ClearType::All))
            .unwrap()
            .queue(SetForegroundColor(Color::Green))
            .unwrap()
            .queue(MoveTo(0, 0))
            .unwrap();
        let done = self.quote[..self.state.current].join(" ");
        self.stdout.queue(Print(&done))?;

        if done.chars().count() > 0 {
            self.stdout.queue(Print(" "))?;
        }

        let mut cur_loc = done.chars().count() + buf_size;
        if self.state.current > 0 {
            cur_loc += 1;
        }

        for i in 0..self.state.buffer.chars().count() {
            if i >= cur_word_size {
                break;
            }

            let c = self.quote[self.state.current].chars().nth(i).unwrap();
            if self.state.buffer.chars().nth(i).unwrap() == c {
                self.stdout.queue(SetForegroundColor(Color::Green)).unwrap();
            } else {
                self.stdout.queue(SetForegroundColor(Color::Red)).unwrap();
            }
            self.stdout.queue(Print(&c))?;
        }

        match (buf_size, cur_word_size) {
            (a, b) if a < b => {
                self.stdout.queue(SetForegroundColor(Color::Reset)).unwrap();
                let v = &self.quote[self.state.current]
                    .substring(buf_size, self.quote[self.state.current].chars().count())
                    .unwrap();
                self.stdout.queue(Print(v))?;
            }
            (a, b) if a > b => {
                self.stdout
                    .queue(SetForegroundColor(Color::Yellow))
                    .unwrap();
                let v = &self
                    .state
                    .buffer
                    .substring(cur_word_size, self.state.buffer.chars().count())
                    .unwrap();
                self.stdout.queue(Print(v))?;
            }
            _ => (),
        }

        self.stdout.queue(Print(" "))?;

        self.stdout.queue(SetForegroundColor(Color::Reset)).unwrap();
        let to_do = self.quote[self.state.current + 1..].join(" ");
        self.stdout.queue(Print(&to_do))?.queue(Print("\n"))?;

        self.stdout
            .queue(MoveTo(cur_loc as u16 % col, cur_loc as u16 / col))?;

        self.stdout.flush()?;
        self.should_render = false;
        return Ok(());
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
