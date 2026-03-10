mod management;
mod setting;
mod user;

use crate::input;
use crate::rv_api;
use crate::utils;
use crate::TerminalIO;
use crate::DEVELOPMENT_MODE;
use crate::INPUT_TIMEOUT_LONG;
use crate::INPUT_TIMEOUT_SHORT;

use crossterm::{
    cursor::{self, RestorePosition, SavePosition},
    event::{Event, KeyCode},
    execute, queue,
    style::Print,
    terminal,
};

use rv_api::{login_rfid, ApiResult, ApiResultValue};
use std::{
    io,
    sync::mpsc::RecvTimeoutError,
    time::{Duration, Instant},
};
use utils::{ConfirmResult, TimeoutResult};

fn register(username: &str, terminal_io: &mut TerminalIO) -> TimeoutResult<()> {
    utils::printline(
        terminal_io,
        &format!("\r\nuser {username} does not exist, create a new user? [yN]"),
    );
    match utils::confirm_with_default(terminal_io, ConfirmResult::NO).unwrap() {
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
                        ONLY for the use of the members of TKO-äly ry. [yn]\r\n\
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
        utils::printline(terminal_io, "Given passwords do not match, aborting.");
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

    let email = match input_email(terminal_io, INPUT_TIMEOUT_LONG) {
        TimeoutResult::TIMEOUT => {
            utils::printline(terminal_io, "Timed out!");
            std::thread::sleep(std::time::Duration::from_millis(2000));
            return TimeoutResult::TIMEOUT;
        }
        TimeoutResult::RESULT(email) => email,
    };

    match rv_api::register(&username, &password1, &full_name, &email).unwrap() {
        rv_api::ApiResult::Success => {
            utils::printline(terminal_io, &format!("{username} registered successfully"));
            utils::confirm_enter_to_continue(terminal_io);
        }
        rv_api::ApiResult::Fail(msg) => {
            utils::printline(terminal_io, &format!("registration failed: {msg}"));
            utils::confirm_enter_to_continue(terminal_io);
        }
    }
    TimeoutResult::RESULT(())
}

fn input_email(terminal_io: &mut TerminalIO, timeout: Duration) -> TimeoutResult<String> {
    let time = Instant::now();
    loop {
        if Instant::now() - time >= timeout {
            return TimeoutResult::TIMEOUT;
        }

        execute!(terminal_io.writer, Print("\r\nEnter your email address:")).unwrap();

        let email1 = match utils::readline(terminal_io, timeout) {
            TimeoutResult::TIMEOUT => {
                return TimeoutResult::TIMEOUT;
            }
            TimeoutResult::RESULT(s) => s,
        };

        execute!(
            terminal_io.writer,
            Print("\r\nEnter your email address again:")
        )
        .unwrap();

        let email2 = match utils::readline(terminal_io, timeout) {
            TimeoutResult::TIMEOUT => {
                return TimeoutResult::TIMEOUT;
            }
            TimeoutResult::RESULT(s) => s,
        };
        utils::printline(terminal_io, "");

        if email1.is_empty() {
            return TimeoutResult::TIMEOUT;
        }

        if email1.split("@").count() != 2 {
            utils::printline(terminal_io, "Given emails are not valid, try again.");
            std::thread::sleep(std::time::Duration::from_millis(3000));
            for _ in 0..6 {
                utils::clear_line(terminal_io);
            }
            continue;
        }

        if email1 != email2 {
            utils::printline(terminal_io, "Given emails do not match, try again.");
            std::thread::sleep(std::time::Duration::from_millis(3000));
            for _ in 0..6 {
                utils::clear_line(terminal_io);
            }
            continue;
        }

        return TimeoutResult::RESULT(email1);
    }
}

fn set_valid_email(
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
) -> Option<()> {
    utils::printline(
        terminal_io,
        "To continue using RV you need to provide valid email",
    );
    let email = match input_email(terminal_io, INPUT_TIMEOUT_LONG) {
        TimeoutResult::TIMEOUT => {
            utils::printline(terminal_io, "Timed out!");
            std::thread::sleep(std::time::Duration::from_millis(2000));
            return None;
        }
        TimeoutResult::RESULT(email) => email,
    };

    match rv_api::change_email(credentials, &email) {
        Ok(apiresult) => {
            if let ApiResult::Fail(e) = apiresult {
                if e == "Email taken" {
                    utils::printline(
                        terminal_io,
                        "Email is allredy in system. Use another email adres",
                    );
                } else {
                    utils::printline(
                        terminal_io,
                        "Error encountered when connecting to backend, try again",
                    );
                }
                std::thread::sleep(std::time::Duration::from_millis(2000));
                return None;
            }
        }
        Err(_) => {
            utils::printline(
                terminal_io,
                "Error encountered when connecting to backend, try again",
            );
            std::thread::sleep(std::time::Duration::from_millis(2000));
            return None;
        }
    }
    Some(())
}

fn set_valid_full_name(
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
) -> Option<()> {
    utils::printline(
        terminal_io,
        "To continue using RV you need to provide your FULL name",
    );
    utils::printline(terminal_io, "");

    execute!(terminal_io.writer, Print("Enter your FULL name: ")).unwrap();

    let full_name = match utils::readline(terminal_io, INPUT_TIMEOUT_LONG) {
        TimeoutResult::TIMEOUT => {
            utils::printline(terminal_io, "Timed out!");
            std::thread::sleep(std::time::Duration::from_millis(2000));
            return None;
        }
        TimeoutResult::RESULT(s) => s,
    };

    if full_name.is_empty() {
        return None;
    }

    match rv_api::change_username(credentials, &full_name) {
        Ok(apiresult) => {
            if let ApiResult::Fail(_) = apiresult {
                utils::printline(
                    terminal_io,
                    "Error encountered when connecting to backend, try again",
                );
                std::thread::sleep(std::time::Duration::from_millis(2000));
                return None;
            }
        }
        Err(_) => {
            utils::printline(
                terminal_io,
                "Error encountered when connecting to backend, try again",
            );
            std::thread::sleep(std::time::Duration::from_millis(2000));
            return None;
        }
    }
    Some(())
}

pub fn main_loop(terminal_io: &mut TerminalIO) -> io::Result<()> {
    'main: loop {
        execute!(
            terminal_io.writer,
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, terminal::size()?.1)
        )?;
        let leaderboard = match rv_api::get_leaderboard().unwrap() {
            ApiResultValue::Success(v) => v,
            ApiResultValue::Fail(err) => {
                utils::print_error_line(terminal_io, &err);
                continue;
            }
        };
        execute!(terminal_io.writer, SavePosition).unwrap();
        leaderboard
            .iter()
            .take(20)
            .enumerate()
            .for_each(|(idx, val)| {
                queue!(
                    terminal_io.writer,
                    cursor::MoveTo(50, idx as u16 + 5),
                    Print(format!(
                        "{:<20} | {:>6}",
                        val.name.chars().take(20).collect::<String>(),
                        utils::format_money(&val.saldo)
                    ))
                )
                .unwrap();
            });
        execute!(terminal_io.writer, RestorePosition).unwrap();
        execute!(
            terminal_io.writer,
            Print("to log in or register\r\n"),
            Print("enter username: "),
        )?;
        utils::print_rv_logo(terminal_io);
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
                        user::user_loop(terminal_io, &credentials);
                        (&terminal_io, &credentials);
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
                    KeyCode::F(5) => {
                        break;
                    }
                    _ => (),
                },
                Ok(input::InputEvent::Rfid(rfid)) => match login_rfid(&rfid) {
                    Some(credentials) => {
                        let user = match rv_api::get_user_info(&credentials) {
                            Err(_) => {
                                utils::printline(
                                    terminal_io,
                                    "error encountered when connecting to backend, try again",
                                );
                                std::thread::sleep(std::time::Duration::from_millis(2000));
                                continue 'main;
                            }
                            Ok(u) => u,
                        };

                        if user.email.split("@").count() != 2 {
                            if let None = set_valid_email(terminal_io, &credentials) {
                                continue 'main;
                            }
                        }

                        if user.full_name == "no name" {
                            if let None = set_valid_full_name(terminal_io, &credentials) {
                                continue 'main;
                            }
                        }

                        user::user_loop(terminal_io, &credentials);
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

        let user = match rv_api::get_user_info(&credentials) {
            Err(_) => {
                utils::printline(
                    terminal_io,
                    "error encountered when connecting to backend, try again",
                );
                std::thread::sleep(std::time::Duration::from_millis(2000));
                continue 'main;
            }
            Ok(u) => u,
        };

        if user.email.split("@").count() != 2 {
            if let None = set_valid_email(terminal_io, &credentials) {
                continue 'main;
            }
        }

        if user.full_name == "no name" {
            if let None = set_valid_full_name(terminal_io, &credentials) {
                continue 'main;
            }
        }

        if credentials.password_reset {
            execute!(terminal_io.writer, Print("Enter new password: ")).unwrap();

            let password1 = match utils::readpasswd(terminal_io, INPUT_TIMEOUT_LONG) {
                TimeoutResult::TIMEOUT => continue 'main,
                TimeoutResult::RESULT(s) => s,
            };

            utils::printline(terminal_io, "");
            execute!(terminal_io.writer, Print("Enter new password again: ")).unwrap();

            let password2 = match utils::readpasswd(terminal_io, INPUT_TIMEOUT_LONG) {
                TimeoutResult::TIMEOUT => continue 'main,
                TimeoutResult::RESULT(s) => s,
            };

            utils::printline(terminal_io, "");

            if password1.is_empty() {
                utils::printline(
                    terminal_io,
                    "Empty password is not allowed! Password not changed.",
                );
                continue 'main;
            } else if password1 == password2 {
                match rv_api::change_password(&credentials, &password1).unwrap() {
                    rv_api::ApiResult::Success => {
                        utils::printline(terminal_io, "New password successfully changed.");
                    }
                    rv_api::ApiResult::Fail(msg) => {
                        utils::print_error_line(
                            terminal_io,
                            &format!("Password change failed: {msg}"),
                        );
                        continue 'main;
                    }
                }
            } else {
                utils::printline(terminal_io, "Passwords do not match! Password not changed.");
                continue 'main;
            }
        }

        user::user_loop(terminal_io, &credentials);
    }
}
