use clap::Clap;
use crossterm::{event, ExecutableCommand};
use std::sync::mpsc;
use std::thread;
use std::{error::Error, io, time::Duration};
use terminal_fonts::to_block_string;
use tui::{
    backend::CrosstermBackend,
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
            Status::Break => Color::Green,
        }
    }
}

/// A tomato timer
#[derive(Clap)]
#[clap(author, about, version)]
struct Opts {
    /// Work timer in minutes
    #[clap(short, long, default_value = "25")]
    work_time: u64,
    /// Break timer in minutes
    #[clap(short, long, default_value = "5")]
    break_time: u64,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opts: Opts = Opts::parse();
    let mut status = Status::Work;
    let mut left_seconds = opts.work_time * 60;
    let mut finish = false;
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(crossterm::terminal::EnterAlternateScreen)?;
    stdout.execute(crossterm::cursor::Hide)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let (tx, rx) = mpsc::channel();

    let tx_key_event = tx.clone();
    thread::spawn(move || loop {
        if let event::Event::Key(key) = event::read().unwrap() {
            tx_key_event.send(Event::Input(key)).unwrap();
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
            let y = (size.height - text_height) / 2;
            let rect = Rect::new(0, y, size.width, text_height);
            f.render_widget(paragraph, rect);
        })?;

        match rx.recv()? {
            Event::Input(input) => {
                if input.code == event::KeyCode::Char('q') {
                    quit(0)?;
                }
            }
            Event::Tick => {
                if !finish {
                    if left_seconds == 0 {
                        match status {
                            Status::Work => {
                                status = Status::Break;
                                left_seconds = opts.break_time * 60;
                                notify("Your work time is up, take a break!");
                            }
                            Status::Break => {
                                notify("Your break time is up!!");
                                finish = true;
                            }
                        }
                    }
                    if left_seconds > 0 {
                        left_seconds -= 1;
                    }
                }
            }
        }
    }
}

enum Event<I> {
    Input(I),
    Tick,
}

#[cfg(not(target_os = "windows"))]
fn notify(msg: &str) {
    let msg = msg.to_string();
    std::thread::spawn(move || {
        let _ = notify_rust::Notification::new()
            .summary("Tomato Timer")
            .body(msg.as_str())
            .show();
    });
}

#[cfg(target_os = "windows")]
fn notify(msg: &str) {
    let msg = msg.to_string();
    std::thread::spawn(move || {
        let _ = std::process::Command::new("powershell")
            .args(&[
                "-WindowStyle",
                "Hidden",
                "-NonInteractive",
                "-Command",
                format!(
                    "New-BurntToastNotification -Text \"Tomato Timer\",\"{}\"",
                    msg
                )
                .as_str(),
            ])
            .status();
    });
}

fn quit(code: i32) -> Result<(), Box<dyn Error>> {
    let mut stdout = io::stdout();
    stdout.execute(crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;
    stdout.execute(crossterm::cursor::Show)?;
    std::process::exit(code);
}
