use crate::{input::InputEvent, TerminalIO, INPUT_TIMEOUT_SHORT};

use super::input;
use crossterm::{
    cursor::{self, RestorePosition, SavePosition},
    event::{Event, KeyCode, KeyEvent},
    execute,
    style::{Print, PrintStyledContent, Stylize},
    terminal,
};

pub enum TimeoutResult<T> {
    RESULT(T),
    TIMEOUT,
}

use core::str;
use regex::Regex;
use std::{
    fs,
    process::{Command, ExitStatus},
    sync::{mpsc::RecvTimeoutError, LazyLock},
    thread::{self, sleep},
    time::Duration,
};

macro_rules! load_ascii {
    ($name:expr) => {
        LazyLock::new(|| include_str!($name).replace("\n", "\r\n"))
    };
}
pub(crate) use load_ascii;

pub fn format_money(cents: &i32) -> String {
    format!(
        "{}{}.{:02}",
        if *cents < 0 { "-" } else { "" },
        cents.abs() / 100,
        cents.abs() % 100
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn money_works() {
        assert_eq!(format_money(&42), "0.42");
        assert_eq!(format_money(&100), "1.00");
        assert_eq!(format_money(&142), "1.42");
        assert_eq!(format_money(&0), "0.00");
        assert_eq!(format_money(&-42), "-0.42");
        assert_eq!(format_money(&-100), "-1.00");
        assert_eq!(format_money(&-142), "-1.42");
        assert_eq!(format_money(&12342), "123.42");
        assert_eq!(format_money(&-12342), "-123.42");
    }

    #[test]
    fn is_barcode_works() {
        assert!(is_barcode("38588901797050"));
        assert!(is_barcode("3858890179705"));
        assert!(is_barcode("4901234567894"));
        assert!(is_barcode("700941359952"));
        assert!(is_barcode("80111351"));
        assert!(is_barcode("01234565"));
        assert!(!is_barcode("user"));
    }
}

pub fn set_small_font() {
    let output = Command::new("setfont")
        .arg("Uni2-VGA16.psf.gz")
        .arg("-C")
        .arg("/dev/tty1")
        .output()
        .unwrap();
    if !ExitStatus::success(&output.status) {
        eprintln!(
            "Setfont exit code {}\n stderr: {}",
            output.status,
            &str::from_utf8(&output.stderr).unwrap()
        );
    }
}
pub fn set_big_font() {
    let output = Command::new("setfont")
        .arg("Uni2-VGA28x16.psf.gz")
        .arg("-C")
        .arg("/dev/tty1")
        .output()
        .unwrap();
    if !ExitStatus::success(&output.status) {
        eprintln!(
            "Setfont exit code {}\n stderr: {}",
            output.status,
            &str::from_utf8(&output.stderr).unwrap()
        );
    }
}

pub fn purchase_fail_bell() {
    thread::spawn(|| {
        let mut tty = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")
            .unwrap();
        execute!(tty, Print("\u{0007}")).unwrap();
        sleep(Duration::from_millis(400));
        execute!(tty, Print("\u{0007}")).unwrap();
        sleep(Duration::from_millis(400));
        execute!(tty, Print("\u{0007}")).unwrap();
        sleep(Duration::from_millis(400));
        execute!(tty, Print("\u{0007}")).unwrap();
    });
}

pub fn printline(terminal_io: &mut TerminalIO, s: &str) {
    execute!(terminal_io.writer, Print(s), Print("\r\n")).unwrap();
}

pub fn print_title(terminal_io: &mut TerminalIO, s: &str) {
    execute!(
        terminal_io.writer,
        PrintStyledContent(format!("=={}==\r\n", s).dark_magenta())
    )
    .unwrap();
}

pub fn print_error_line(terminal_io: &mut TerminalIO, s: &str) {
    execute!(
        terminal_io.writer,
        PrintStyledContent("ERROR".red()),
        Print(": "),
        Print(s),
        Print("\r\n")
    )
    .unwrap();
}

pub fn print_rv_logo(terminal_io: &mut TerminalIO) {
    static RV_LOGO: LazyLock<String> = load_ascii!("../ascii/logo.txt");
    execute!(
        terminal_io.writer,
        SavePosition,
        cursor::MoveTo(0, 3),
        PrintStyledContent(RV_LOGO.to_string().yellow()),
        RestorePosition
    );
}

pub fn readpasswd(terminal_io: &mut TerminalIO, timeout: Duration) -> TimeoutResult<String> {
    readline_internal(false, timeout, terminal_io).unwrap()
}

pub fn readline(terminal_io: &mut TerminalIO, timeout: Duration) -> TimeoutResult<String> {
    readline_internal(true, timeout, terminal_io).unwrap()
}

pub enum ConfirmResult {
    YES,
    NO,
    TIMEOUT,
}
pub fn confirm(terminal_io: &mut TerminalIO) -> Result<ConfirmResult, std::io::Error> {
    loop {
        match terminal_io.recv.recv_timeout(INPUT_TIMEOUT_SHORT) {
            Ok(InputEvent::Terminal(Event::Key(KeyEvent {
                code: KeyCode::Char(c),
                ..
            }))) => match c {
                'Y' | 'y' => return Ok(ConfirmResult::YES),
                'N' | 'n' => return Ok(ConfirmResult::NO),
                _ => (),
            },
            Err(RecvTimeoutError::Timeout) => return Ok(ConfirmResult::TIMEOUT),
            Err(RecvTimeoutError::Disconnected) => panic!(),
            _ => (),
        }
    }
}

// Returns default if enter is pressed
pub fn confirm_with_default(
    terminal_io: &mut TerminalIO,
    default: ConfirmResult,
) -> Result<ConfirmResult, std::io::Error> {
    loop {
        match terminal_io.recv.recv_timeout(INPUT_TIMEOUT_SHORT) {
            Ok(InputEvent::Terminal(Event::Key(KeyEvent {
                code: KeyCode::Char(c),
                ..
            }))) => match c {
                'Y' | 'y' => return Ok(ConfirmResult::YES),
                'N' | 'n' => return Ok(ConfirmResult::NO),
                _ => (),
            },
            Ok(InputEvent::Terminal(Event::Key(KeyEvent {
                code: KeyCode::Enter,
                ..
            }))) => return Ok(default),
            Err(RecvTimeoutError::Timeout) => return Ok(ConfirmResult::TIMEOUT),
            Err(RecvTimeoutError::Disconnected) => panic!(),
            _ => (),
        }
    }
}

pub fn clear_terminal(terminal_io: &mut TerminalIO) {
    execute!(
        terminal_io.writer,
        terminal::Clear(terminal::ClearType::All)
    )
    .unwrap()
}

pub fn clear_line(terminal_io: &mut TerminalIO) {
    execute!(
        terminal_io.writer,
        terminal::Clear(terminal::ClearType::CurrentLine),
        cursor::MoveUp(1),
    )
    .unwrap();
}

pub fn confirm_enter_to_continue(terminal_io: &mut TerminalIO) -> ConfirmResult {
    printline(terminal_io, "Press ENTER to continue");
    loop {
        match terminal_io.recv.recv_timeout(INPUT_TIMEOUT_SHORT) {
            Ok(InputEvent::Terminal(Event::Key(KeyEvent {
                code: KeyCode::Enter,
                ..
            }))) => return ConfirmResult::YES,
            Err(RecvTimeoutError::Timeout) => return ConfirmResult::TIMEOUT,
            Err(RecvTimeoutError::Disconnected) => panic!(),
            _ => (),
        }
    }
}

fn readline_internal(
    echo: bool,
    timeout: Duration,
    terminal_io: &mut TerminalIO,
) -> Result<TimeoutResult<String>, std::io::Error> {
    let mut ret = String::new();
    loop {
        match terminal_io.recv.recv_timeout(timeout) {
            Err(RecvTimeoutError::Timeout) => {
                printline(terminal_io, "");
                return Ok(TimeoutResult::TIMEOUT);
            }
            Ok(input::InputEvent::Terminal(Event::Key(ev))) => match ev.code {
                KeyCode::Char(c) => {
                    ret.push(c);
                    if echo {
                        execute!(terminal_io.writer, Print(c))?;
                    }
                }
                KeyCode::Backspace => {
                    if !ret.is_empty() {
                        if echo {
                            execute!(
                                terminal_io.writer,
                                cursor::MoveLeft(1),
                                Print(" "),
                                cursor::MoveLeft(1)
                            )?;
                        }
                        ret.pop();
                    }
                }
                KeyCode::Enter => {
                    break;
                }
                _ => (),
            },
            _ => (),
        }
    }
    printline(terminal_io, "");
    Ok(TimeoutResult::RESULT(ret.trim().to_string()))
}

pub fn is_barcode(input: &str) -> bool {
    if !input.chars().all(|chr| chr.is_ascii_digit()) {
        return false;
    }

    let len = input.len();
    let size_even = len % 2 == 0;

    if len != 8 && len != 12 && len != 13 && len != 14 {
        return false;
    }

    let code: Vec<u32> = input
        .chars()
        .map(|char| char.to_digit(10).unwrap())
        .collect();

    let sum: u32 = code[..len - 1]
        .iter()
        .enumerate()
        .map(|(i, &digit)| {
            if i % 2 == 0 {
                if size_even {
                    3 * digit
                } else {
                    digit
                }
            } else {
                if size_even {
                    digit
                } else {
                    3 * digit
                }
            }
        })
        .sum();
    let check_sum = (10 - (sum % 10)) % 10;
    println!("{}", check_sum);
    return check_sum == *code.last().unwrap();
}
