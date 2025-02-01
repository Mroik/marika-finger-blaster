use std::{
    collections::HashSet,
    error::Error,
    io::{stdout, Stdout, Write},
    time::Duration,
};

use crossterm::{
    cursor::{MoveDown, MoveTo, MoveToColumn, RestorePosition, SavePosition, SetCursorStyle},
    style::{Color, Print, SetForegroundColor},
    terminal::{
        disable_raw_mode, enable_raw_mode, size, Clear, ClearType, EnterAlternateScreen,
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
    error::{TerminalTooSmallError, TyperError, WordTooLongError},
    event::{handle_input, Event},
    state::State,
};

pub const TICK_RATE: u64 = 1000 / 20;
pub const MIN_TERM_COL: u16 = 40;
pub const MIN_TERM_ROW: u16 = 10;
const MAX_QUOTE_LINE: u16 = 80;
const MIN_MARGIN: u16 = 4;

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
    mistake_count: u32,
    mistakes: HashSet<(usize, usize)>,
    raw_quote: &'a str,
    error: Option<TyperError>,
}

impl App<'_> {
    pub fn new(quote: &str) -> App {
        let (event_tx, event_rx): (Sender<Event>, Receiver<Event>) = channel(10);
        App {
            stdout: stdout(),
            quote: quote.split_whitespace().filter(|s| !s.is_empty()).collect(),
            raw_quote: quote,
            event_rx,
            event_tx,
            running: false,
            state: State::default(),
            should_render: true,
            start: None,
            completed: false,
            mistake_count: 0,
            mistakes: HashSet::new(),
            error: None,
        }
    }

    async fn format_quote(quote: &str, row_len: u16) -> Result<Vec<Vec<&str>>, WordTooLongError> {
        let max = if row_len - (MIN_MARGIN * 2) < MAX_QUOTE_LINE {
            row_len - (MIN_MARGIN * 2)
        } else {
            MAX_QUOTE_LINE
        };
        let mut counter = 0;
        let mut lines = Vec::new();
        let mut line = Vec::new();
        for w in quote.split_whitespace().filter(|s| !s.is_empty()) {
            let w_len = w.chars().count();
            if w_len > max as usize {
                return Err(WordTooLongError::new(w, max));
            }

            if w_len + counter > max as usize {
                lines.push(line);
                line = Vec::new();
                line.push(w);
                counter = w_len + 1;
            } else {
                line.push(w);
                counter += w_len + 1;
            }
        }
        lines.push(line);
        return Ok(lines);
    }

    async fn run(&mut self) -> Result<(f64, f64, String), Box<dyn Error>> {
        self.stdout
            .execute(EnterAlternateScreen)?
            .execute(SetCursorStyle::SteadyBar)?;
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
        let accuracy = total_chars * 100.0 / (total_chars + self.mistake_count as f64);
        let history = self.generate_mistake_locations().await;

        input_ks_tx.send(()).await?;
        tick_ks_tx.send(()).await?;

        disable_raw_mode()?;
        self.stdout.execute(LeaveAlternateScreen)?;
        return Ok((wpm, accuracy, history));
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn Error>> {
        let (wpm, accuracy, history) = self.run().await?;
        if self.completed {
            println!(
                "Mistake history:\n{}\n\nYour stats\nWPM: {}\nAccuracy: {}%\nMistakes: {}",
                history,
                wpm.round(),
                accuracy.round(),
                self.mistake_count
            );
        }
        if self.error.is_some() {
            println!("{}", self.error.as_ref().unwrap());
        }
        return Ok(());
    }

    async fn generate_mistake_locations(&self) -> String {
        let mut miss: Vec<(usize, usize)> = self.mistakes.iter().copied().collect();
        miss.sort();
        miss.reverse();

        let a: Vec<String> = self
            .quote
            .iter()
            .enumerate()
            .map(|(i, w)| {
                let mut ris = String::new();
                for (j, c) in w.chars().enumerate() {
                    match miss.last() {
                        Some((a, b)) if *a == i && *b == j => {
                            ris.push_str("\x1b[31m");
                            ris.push(c);
                            ris.push_str("\x1b[0m");
                            miss.pop();
                        }
                        _ => ris.push(c),
                    }
                }
                ris
            })
            .collect();
        return a.join(" ");
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
            Event::ForceRender => (),
        }

        if event != Event::Render {
            self.should_render = true;
        }
        return Ok(());
    }

    async fn handle_keypress(&mut self, k: char) -> Result<(), Box<dyn Error>> {
        self.state.buffer.push(k);
        let last_byte = self.state.buffer.char_indices().last().unwrap().0;
        let is_word_completed = self.state.buffer[..last_byte] == *self.quote[self.state.current];
        let is_text_completed = self.state.buffer == self.quote[self.state.current]
            && self.state.current == self.quote.len() - 1;
        let is_correct = self.state.buffer.chars().count()
            <= self.quote[self.state.current].chars().count()
            && self.state.buffer.chars().last().unwrap()
                == self.quote[self.state.current]
                    .chars()
                    .nth(self.state.buffer.chars().count() - 1)
                    .unwrap();
        let miss_word = self.state.current;

        if is_word_completed && k == ' ' {
            self.state.buffer.clear();
            self.state.current += 1;
        } else if is_text_completed {
            self.running = false;
            self.completed = true;
        } else if !is_correct {
            self.mistake_count += 1;
            if self.state.buffer.chars().count() <= self.quote[miss_word].chars().count() {
                self.mistakes
                    .insert((miss_word, self.state.buffer.chars().count() - 1));
            }
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

        let (cols, rows) = size()?;
        if cols < MIN_TERM_COL || rows < MIN_TERM_ROW {
            self.error = Some(TyperError::TerminalTooSmallError(TerminalTooSmallError));
            self.running = false;
            return Ok(());
        }
        let lines = match App::format_quote(self.raw_quote, cols).await {
            Ok(v) => v,
            Err(e) => {
                self.error = Some(TyperError::WordTooLongError(e));
                self.running = false;
                return Ok(());
            }
        };
        let margin = (cols
            - lines
                .iter()
                .map(|line| line.join(" ").chars().count())
                .max()
                .unwrap() as u16)
            / 2
            + 1; // Terminals index starting with 1 instead of 0
        let current_line = lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                (
                    i,
                    line.len() + lines.iter().take(i).fold(0, |a, b| a + b.len()),
                )
            })
            .filter(|(_, cumul_len)| self.state.current < *cumul_len)
            .nth(0)
            .unwrap()
            .0;

        self.stdout
            .queue(Clear(ClearType::All))
            .unwrap()
            .queue(MoveTo(margin, (rows - 3) / 2 - 2))
            .unwrap();

        // Prev line
        if current_line > 0 {
            self.stdout
                .queue(SetForegroundColor(Color::Green))?
                .queue(Print(lines[current_line - 1].join(" ")))?
                .queue(MoveDown(1))?
                .queue(MoveToColumn(margin))?;
        }

        // Cur line
        let offset = lines[..current_line]
            .iter()
            .map(|line| line.len())
            .sum::<usize>();
        for i in 0..lines[current_line].len() {
            if i + offset < self.state.current {
                self.stdout
                    .queue(SetForegroundColor(Color::Green))?
                    .queue(Print(lines[current_line][i]))?
                    .queue(Print(' '))?;
                continue;
            }

            if i + offset > self.state.current {
                self.stdout
                    .queue(SetForegroundColor(Color::Reset))?
                    .queue(Print(lines[current_line][i]))?
                    .queue(Print(' '))?;
                continue;
            }

            let cc: Vec<char> = lines[current_line][i].chars().collect();
            let vv: Vec<char> = self.state.buffer.chars().collect();
            for j in 0..lines[current_line][i].chars().count() {
                if self.state.buffer.chars().count() <= j {
                    break;
                }

                if cc[j] == vv[j] {
                    self.stdout.queue(SetForegroundColor(Color::Green))?;
                } else {
                    self.stdout.queue(SetForegroundColor(Color::Red))?;
                }
                self.stdout.queue(Print(cc[j]))?;
            }
            self.stdout.queue(SavePosition)?;

            match (cc.len(), vv.len()) {
                (ccc, vvv) if ccc < vvv => {
                    self.stdout.queue(SetForegroundColor(Color::Yellow))?;
                    let remaining = vv.iter().skip(cc.len()).fold(String::new(), |mut a, b| {
                        a.push(*b);
                        a
                    });
                    self.stdout.queue(Print(remaining))?;
                    self.stdout.queue(SavePosition)?;
                }
                (ccc, vvv) if ccc > vvv => {
                    self.stdout.queue(SetForegroundColor(Color::Reset))?;
                    let remaining = cc.iter().skip(vv.len()).fold(String::new(), |mut a, b| {
                        a.push(*b);
                        a
                    });
                    self.stdout.queue(Print(remaining))?;
                }
                (_, _) => (),
            }
            self.stdout.queue(Print(' '))?;
        }
        self.stdout
            .queue(MoveDown(1))?
            .queue(MoveToColumn(margin))?;

        // Next line
        if lines.len() > 1 && current_line < lines.len() - 1 {
            let last_rendered = if current_line == 0 && lines.len() > 2 {
                2
            } else {
                1
            };
            for line in &lines[current_line + 1..current_line + 1 + last_rendered] {
                self.stdout
                    .queue(SetForegroundColor(Color::Reset))?
                    .queue(Print(line.join(" ")))?
                    .queue(MoveDown(1))?
                    .queue(MoveToColumn(margin))?;
            }
        }
        self.stdout.queue(RestorePosition)?;

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
