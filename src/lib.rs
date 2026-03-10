pub mod input;
mod loops;
mod rv_api;
mod utils;

use crossterm::{
    cursor, execute,
    terminal::{enable_raw_mode, EnterAlternateScreen},
};

use std::{
    io::{self, stdout, Result, Stdout, Write},
    sync::{mpsc::Receiver, LazyLock},
    time::Duration,
};

pub const INPUT_TIMEOUT_SHORT: Duration = Duration::from_secs(60);
pub const INPUT_TIMEOUT_LONG: Duration = Duration::from_secs(5 * 60);

pub static DEVELOPMENT_MODE: LazyLock<bool> =
    LazyLock::new(|| std::env::var("DEVELOPMENT").is_ok());

pub struct TerminalWriter {
    stdout: Stdout,
    pub test_output: Vec<u8>,
    test: bool, // True to write into test_output instead of stdout
}

impl TerminalWriter {
    pub fn new(test: bool) -> Self {
        Self {
            stdout: stdout(),
            test_output: Vec::new(),
            test,
        }
    }
}

impl Write for TerminalWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        if self.test {
            self.test_output.write(buf)
        } else {
            self.stdout.write(buf)
        }
    }
    fn flush(&mut self) -> Result<()> {
        if self.test {
            self.test_output.flush()
        } else {
            self.stdout.flush()
        }
    }
}

pub struct TerminalIO {
    pub recv: Receiver<input::InputEvent>,
    pub writer: TerminalWriter,
}

pub fn start() -> io::Result<()> {
    utils::set_big_font();
    let mut terminal_io = TerminalIO {
        recv: input::init(),
        writer: TerminalWriter::new(false),
    };
    enable_raw_mode().expect("Enabling raw mode failed");
    execute!(
        terminal_io.writer,
        EnterAlternateScreen,
        cursor::EnableBlinking,
        cursor::MoveTo(0, 0)
    )?;
    loops::main_loop(&mut terminal_io)
}
