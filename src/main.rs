use clap::Clap;
use std::sync::mpsc;
use std::thread;
use std::{error::Error, io, time::Duration};
use terminal_fonts::to_block_string;
use termion::{
    event::Key,
    input::{MouseTerminal, TermRead},
    raw::IntoRawMode,
    screen::AlternateScreen,
};
use tui::{
    backend::TermionBackend,
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Paragraph, Text},
    Terminal,
};

#[derive(Debug, Eq, PartialEq)]
enum Status {
    Work,
    Break,
}

impl Status {
    pub fn color(&self) -> Color {
        match self {
            Status::Work => Color::Red,
            Status::Break => Color::Red,
        }
    }
}

/// A tomato timer
#[derive(Clap)]
struct Opts {
    /// Work timer in minutes
    #[clap(short, long, default_value = "25")]
    work_timer: u64,
    /// Break timer in minutes
    #[clap(short, long, default_value = "5")]
    break_timer: u64,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opts: Opts = Opts::parse();
    let mut status = Status::Work;
    let mut left_seconds = opts.work_timer * 60;

    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;
    let (tx, rx) = mpsc::channel();

    let tx_key_event = tx.clone();
    thread::spawn(move || {
        let stdin = io::stdin();
        for evt in stdin.keys() {
            if let Ok(key) = evt {
                if let Err(err) = tx_key_event.send(Event::Input(key)) {
                    eprintln!("{}", err);
                    return;
                }
            }
        }
    });
    let tx_tick_event = tx.clone();
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(1));
        tx_tick_event.send(Event::Tick).unwrap();
    });

    loop {
        terminal.draw(|mut f| {
            let minutes = left_seconds / 60;
            let seconds = left_seconds % 60;
            let block_string = to_block_string(&format!("{:02}:{:02}", minutes, seconds));
            let texts: Vec<Text> = block_string
                .split("\n")
                .map(|v| format!("{}\n", v))
                .map(|v| Text::raw(v))
                .collect();
            let text_height = texts.len() as u16;
            let style = Style::new().fg(status.color());
            let paragraph = Paragraph::new(texts.iter())
                .alignment(Alignment::Center)
                .style(style);
            let size = f.size();
            let y = (size.height - text_height) /2;
            let rect = Rect::new(0, y, size.width, text_height);
            f.render_widget(paragraph, rect);
        })?;

        match rx.recv()? {
            Event::Input(input) => {
                if input == Key::Char('q') {
                    break;
                }
            }
            Event::Tick => {
                if left_seconds == 0 {
                    match status {
                        Status::Work => {
                            status = Status::Break;
                            left_seconds = opts.break_timer * 60;
                            // TODO desktop notify
                        }
                        Status::Break => {}
                    }
                }
                left_seconds -= 1;
            }
        }
    }

    Ok(())
}

enum Event<I> {
    Input(I),
    Tick,
}
