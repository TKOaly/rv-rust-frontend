pub mod input;
mod rv_api;
mod user_loop;
mod utils;
use lazy_static::lazy_static;

use crossterm::{
    cursor,
    event::{Event, KeyCode},
    execute,
    style::Print,
    terminal::{self, enable_raw_mode, EnterAlternateScreen},
};

use rv_api::{login_rfid, ApiResultValue};
use std::{
    io::{self, stdout, Result, Stdout, Write},
    sync::mpsc::{Receiver, RecvTimeoutError},
    time::Duration,
};
use utils::{printline, ConfirmResult, TimeoutResult};

pub const INPUT_TIMEOUT_SHORT: Duration = Duration::from_secs(60);
pub const INPUT_TIMEOUT_LONG: Duration = Duration::from_secs(5 * 60);

lazy_static! {
    static ref DEVELOPMENT_MODE: bool = match std::env::var("DEVELOPMENT") {
        Ok(v) => true,
        Err(_) => false,
    };
}

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

fn register(username: &str, terminal_io: &mut TerminalIO) -> TimeoutResult<()> {
    utils::printline(
        terminal_io,
        &format!("\r\nuser {username} does not exist, create a new user? [yN]"),
    );
    match utils::confirm(terminal_io).unwrap() {
        ConfirmResult::YES => (),
        ConfirmResult::NO => {
            utils::printline(terminal_io, "Aborting!");
            std::thread::sleep(std::time::Duration::from_millis(2000));
            return TimeoutResult::RESULT(());
        }
        ConfirmResult::TIMEOUT => {
            utils::printline(terminal_io, "Timed out!");
            std::thread::sleep(std::time::Duration::from_millis(2000));
            return TimeoutResult::TIMEOUT;
        }
    }

    execute!(
                terminal_io.writer,
                Print(&format!(
                        "\r\n\
                        I am a member of TKO-äly ry and I understand that this service is intended\r\n\
                        ONLY for the use of the members of TKO-äly ry. [yN]\r\n\
                        "
                ))
            ).unwrap();
    match utils::confirm(terminal_io).unwrap() {
        ConfirmResult::YES => (),
        ConfirmResult::NO => {
            utils::printline(terminal_io, "Aborting!");
            std::thread::sleep(std::time::Duration::from_millis(2000));
            return TimeoutResult::RESULT(());
        }
        ConfirmResult::TIMEOUT => {
            utils::printline(terminal_io, "Timed out!");
            std::thread::sleep(std::time::Duration::from_millis(2000));
            return TimeoutResult::TIMEOUT;
        }
    }

    execute!(
        terminal_io.writer,
        Print(&format!(
            "\r\ncreating a new user: {username}\r\nenter password:"
        ))
    )
    .unwrap();

    let password1 = match utils::readpasswd(terminal_io, INPUT_TIMEOUT_LONG) {
        TimeoutResult::TIMEOUT => {
            utils::printline(terminal_io, "Timed out!");
            std::thread::sleep(std::time::Duration::from_millis(2000));
            return TimeoutResult::TIMEOUT;
        }
        TimeoutResult::RESULT(s) => s,
    };

    execute!(terminal_io.writer, Print("\r\nenter password again:")).unwrap();
    let password2 = match utils::readpasswd(terminal_io, INPUT_TIMEOUT_LONG) {
        TimeoutResult::TIMEOUT => {
            utils::printline(terminal_io, "Timed out!");
            std::thread::sleep(std::time::Duration::from_millis(2000));
            return TimeoutResult::TIMEOUT;
        }
        TimeoutResult::RESULT(s) => s,
    };

    if password1 != password2 {
        printline(terminal_io, "Given passwords do not match, aborting.");
        std::thread::sleep(std::time::Duration::from_millis(2000));
        return TimeoutResult::RESULT(());
    }

    execute!(terminal_io.writer, Print("\r\nEnter your FULL name:")).unwrap();

    let full_name = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
        TimeoutResult::TIMEOUT => {
            utils::printline(terminal_io, "Timed out!");
            std::thread::sleep(std::time::Duration::from_millis(2000));
            return TimeoutResult::TIMEOUT;
        }
        TimeoutResult::RESULT(s) => s,
    };

    execute!(terminal_io.writer, Print("\r\nEnter your email address:")).unwrap();

    let email = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
        TimeoutResult::TIMEOUT => {
            utils::printline(terminal_io, "Timed out!");
            std::thread::sleep(std::time::Duration::from_millis(2000));
            return TimeoutResult::TIMEOUT;
        }
        TimeoutResult::RESULT(s) => s,
    };
    printline(terminal_io, "");

    match rv_api::register(&username, &password1, &full_name, &email).unwrap() {
        rv_api::ApiResult::Success => {
            printline(terminal_io, &format!("{username} registered successfully"));
            utils::confirm_enter_to_continue(terminal_io);
        }
        rv_api::ApiResult::Fail(msg) => {
            printline(terminal_io, &format!("registration failed: {msg}"));
            utils::confirm_enter_to_continue(terminal_io);
        }
    }
    TimeoutResult::RESULT(())
}

pub fn main_loop(terminal_io: &mut TerminalIO) -> io::Result<()> {
    'main: loop {
        execute!(
            terminal_io.writer,
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, terminal::size()?.1)
        )?;
        execute!(terminal_io.writer, Print("enter username: "))?;
        let mut username = String::new();
        loop {
            match &terminal_io.recv.recv().unwrap() {
                input::InputEvent::Terminal(Event::Key(ev)) => match ev.code {
                    KeyCode::Char(c) => {
                        username.push(c);
                        execute!(terminal_io.writer, Print(c))?;
                    }
                    KeyCode::Backspace => {
                        if !username.is_empty() {
                            execute!(
                                terminal_io.writer,
                                cursor::MoveLeft(1),
                                Print(" "),
                                cursor::MoveLeft(1)
                            )
                            .expect("fail");
                            username.pop();
                        }
                    }
                    KeyCode::Enter => {
                        if !username.is_empty() {
                            break;
                        }
                    }
                    _ => (),
                },
                input::InputEvent::Rfid(rfid) => match login_rfid(&rfid) {
                    Some(credentials) => {
                        user_loop::user_loop(&credentials, terminal_io);
                        continue 'main;
                    }
                    None => {
                        utils::printline(terminal_io, "No matching users found for rfid");
                        std::thread::sleep(std::time::Duration::from_millis(2000));
                        continue 'main;
                    }
                },
                _ => (),
            }
        }

        if username == "quit" && *DEVELOPMENT_MODE {
            return Ok(());
        }

        if !rv_api::user_exists(&username).unwrap() {
            register(&username, terminal_io);
            continue 'main;
        }

        execute!(terminal_io.writer, Print("\r\nenter password: ")).expect("fail");
        let mut password = String::new();
        loop {
            match &terminal_io.recv.recv_timeout(INPUT_TIMEOUT_SHORT) {
                Err(RecvTimeoutError::Timeout) => {
                    utils::printline(terminal_io, "Timed out!");
                    std::thread::sleep(std::time::Duration::from_millis(2000));
                    continue 'main;
                }
                Ok(input::InputEvent::Terminal(Event::Key(ev))) => match ev.code {
                    KeyCode::Char(c) => {
                        password.push(c);
                    }
                    KeyCode::Backspace => {
                        if !username.is_empty() {
                            password.pop();
                        }
                    }
                    KeyCode::Enter => {
                        break;
                    }
                    _ => (),
                },
                Ok(input::InputEvent::Rfid(rfid)) => match login_rfid(&rfid) {
                    Some(credentials) => {
                        user_loop::user_loop(&credentials, terminal_io);
                        continue 'main;
                    }
                    None => {
                        utils::printline(terminal_io, "no matching users found for rfid");
                        std::thread::sleep(std::time::Duration::from_millis(2000));
                        continue 'main;
                    }
                },
                _ => (),
            }
        }
        let credentials = match rv_api::login(&username, &password) {
            ApiResultValue::Success(v) => v,
            ApiResultValue::Fail(_) => {
                utils::printline(terminal_io, "error: invalid username or password!");
                std::thread::sleep(std::time::Duration::from_millis(2000));
                continue;
            }
        };
        user_loop::user_loop(&credentials, terminal_io);
    }
}

pub fn start() -> io::Result<()> {
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
    main_loop(&mut terminal_io)
}
