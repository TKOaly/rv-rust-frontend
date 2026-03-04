use crate::input;
use crate::utils;
use crate::rv_api;
use crate::utils::print_title;
use crate::utils::printline;
use crate::utils::clear_terminal;
use crate::utils::TimeoutResult;
use crate::TerminalIO;
use crate::INPUT_TIMEOUT_LONG;
use crate::INPUT_TIMEOUT_SHORT;

use crossterm::{
    cursor,
    event::{Event, KeyCode},
    execute, queue,
    style::{Print, PrintStyledContent, Stylize},
    terminal
};
use input::InputEvent;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

fn change_username(
    timeout: Duration,
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
) -> TimeoutResult<()> {
    print_title(terminal_io, "Change your username:");
    execute!(terminal_io.writer, Print("New username: ")).unwrap();
    let username = match utils::readline(terminal_io, timeout) {
        TimeoutResult::TIMEOUT => {
            return TimeoutResult::TIMEOUT;
        }
        TimeoutResult::RESULT(s) => s,
    };

    match rv_api::change_username(credentials, &username).unwrap() {
        rv_api::ApiResult::Success => {
            utils::printline(terminal_io, "Username successfully changed.");
        }
        rv_api::ApiResult::Fail(msg) => {
            utils::print_error_line(terminal_io, &format!("Username change failed: {msg}"));
        }
    }

    return TimeoutResult::RESULT(());
}

fn change_real_name(
    timeout: Duration,
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
) -> TimeoutResult<()> {
    print_title(terminal_io, "Change your FULL name:");
    execute!(terminal_io.writer, Print("Your FULL name: ")).unwrap();
    let full_name = match utils::readline(terminal_io, timeout) {
        TimeoutResult::TIMEOUT => {
            return TimeoutResult::TIMEOUT;
        }
        TimeoutResult::RESULT(s) => s,
    };

    match rv_api::change_full_name(credentials, &full_name).unwrap() {
        rv_api::ApiResult::Success => {
            utils::printline(terminal_io, "Name successfully changed.");
        }
        rv_api::ApiResult::Fail(msg) => {
            utils::print_error_line(terminal_io, &format!("Name change failed: {msg}"));
        }
    }
    return TimeoutResult::RESULT(());
}

fn change_user_email(
    timeout: Duration,
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
) -> TimeoutResult<()> {
    print_title(terminal_io, "Change Email");

    execute!(terminal_io.writer, Print("Enter new email: ")).unwrap();
    let email1;
    match utils::readline(terminal_io, timeout) {
        TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
        TimeoutResult::RESULT(s) => email1 = s,
    }

    utils::printline(terminal_io, "");
    execute!(terminal_io.writer, Print("Enter new email again: ")).unwrap();
    let email2;
    match utils::readline(terminal_io, timeout) {
        TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
        TimeoutResult::RESULT(s) => email2 = s,
    }
    utils::printline(terminal_io, "");

    if email1.len() == 0 {
        utils::printline(
            terminal_io,
            "Empty email is not allowed! Email not changed.",
        );
    } else if email1.split("@").count() != 2 {
        utils::printline(terminal_io, "You did not provide valid email address");
    } else if email1 == email2 {
        match rv_api::change_email(credentials, &email1).unwrap() {
            rv_api::ApiResult::Success => {
                utils::printline(terminal_io, "Email successfully changed.");
            }
            rv_api::ApiResult::Fail(msg) => {
                utils::print_error_line(terminal_io, &format!("Email change failed: {msg}"));
            }
        }
    } else {
        utils::printline(terminal_io, "Emails do not match! email not changed.");
    }
    utils::printline(terminal_io, "");
    utils::confirm_enter_to_continue(terminal_io);
    utils::printline(terminal_io, "");
    TimeoutResult::RESULT(())
}

fn change_privacy(
    timeout: Duration,
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
) -> TimeoutResult<()> {
    utils::print_title(terminal_io, "Privacy Settings");
    printline(terminal_io, "Change the account's privacy level");
    printline(terminal_io, "0 = No restrictions");
    printline(
        terminal_io,
        "1 = Hide username from public (for example, leaderboards)",
    );
    printline(
        terminal_io,
        "2 = Hide all data from public (for example, list of recent purchases)",
    );
    printline(terminal_io, "<Enter> do not change");
    loop {
        match terminal_io.recv.recv_timeout(timeout) {
            Ok(input::InputEvent::Terminal(Event::Key(ev))) => match ev.code {
                KeyCode::Char(c) => match c {
                    '0' => {
                        rv_api::change_privacy_level(credentials, 0).unwrap();
                        printline(terminal_io, "Changed privacy level to 0");
                        return TimeoutResult::RESULT(());
                    }
                    '1' => {
                        rv_api::change_privacy_level(credentials, 1).unwrap();
                        printline(terminal_io, "Changed privacy level to 1");
                        return TimeoutResult::RESULT(());
                    }
                    '2' => {
                        rv_api::change_privacy_level(credentials, 2).unwrap();
                        printline(terminal_io, "Changed privacy level to 2");
                        return TimeoutResult::RESULT(());
                    }
                    _ => (),
                },
                KeyCode::Enter => return TimeoutResult::RESULT(()),
                _ => (),
            },
            Err(RecvTimeoutError::Timeout) => return TimeoutResult::TIMEOUT,
            _ => return TimeoutResult::RESULT(()),
        }
    }
}

fn change_user_password_user(
    timeout: Duration,
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
) -> TimeoutResult<()> {
    print_title(terminal_io, "Change password");

    execute!(terminal_io.writer, Print("Enter new password: ")).unwrap();
    let password1;
    match utils::readpasswd(terminal_io, timeout) {
        TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
        TimeoutResult::RESULT(s) => password1 = s,
    }

    utils::printline(terminal_io, "");
    execute!(terminal_io.writer, Print("Enter new password again: ")).unwrap();
    let password2;
    match utils::readpasswd(terminal_io, timeout) {
        TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
        TimeoutResult::RESULT(s) => password2 = s,
    }
    utils::printline(terminal_io, "");

    if password1.len() == 0 {
        utils::printline(
            terminal_io,
            "Empty password is not allowed! Password not changed.",
        );
    } else if password1 == password2 {
        match rv_api::change_password(credentials, &password1).unwrap() {
            rv_api::ApiResult::Success => {
                utils::printline(terminal_io, "Password successfully changed.");
            }
            rv_api::ApiResult::Fail(msg) => {
                utils::print_error_line(terminal_io, &format!("Password change failed: {msg}"));
            }
        }
    } else {
        utils::printline(terminal_io, "Passwords do not match! Password not changed.");
    }
    utils::printline(terminal_io, "");
    utils::confirm_enter_to_continue(terminal_io);
    utils::printline(terminal_io, "");
    TimeoutResult::RESULT(())
}

fn change_user_rfid(
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
) -> TimeoutResult<()> {
    print_title(terminal_io, "Set login RFID");
    utils::printline(
        terminal_io,
        "Scan RFID to use for logging in. ENTER to cancel.",
    );
    loop {
        match terminal_io.recv.recv_timeout(INPUT_TIMEOUT_SHORT) {
            Err(RecvTimeoutError::Timeout) => return TimeoutResult::TIMEOUT,
            Ok(input::InputEvent::Terminal(Event::Key(ev))) => match ev.code {
                KeyCode::Enter => {
                    utils::printline(terminal_io, "RFID change cancelled");
                    return TimeoutResult::RESULT(());
                }
                _ => (),
            },
            Ok(input::InputEvent::Rfid(rfid)) => {
                rv_api::change_rfid(credentials, &rfid).unwrap();
                utils::printline(terminal_io, "RFID changed successfully");
                return TimeoutResult::RESULT(());
            }
            _ => return TimeoutResult::RESULT(()),
        }
    }
}

pub fn settings_loop(
    terminal_io: &mut TerminalIO,
    credentials: &rv_api::AuthenticationResponse,
) -> TimeoutResult<()> {
    clear_terminal(terminal_io);
    'main: loop {
        let user_info = rv_api::get_user_info(&credentials).unwrap();

        clear_terminal(terminal_io);

        queue!(
            terminal_io.writer,
            cursor::MoveTo(0, terminal::size()?.1),
            Print("Current values\r\n"),
            PrintStyledContent("Name: ".dark_green().bold()),
            Print(format!("{}\r\n", user_info.full_name)),
            PrintStyledContent("Email: ".dark_green().bold()),
            Print(format!("{}\r\n", user_info.email)),
            PrintStyledContent("Privacy level: ".dark_green().bold()),
            Print(format!("{}\r\n", user_info.privacy_level)),
            Print("\r\n"),
            Print("Available commands (press key to select):\r\n"),
            PrintStyledContent("R".dark_green().bold()),
            Print(" - manage your rfid\r\n"),
            PrintStyledContent("P".dark_green().bold()),
            Print(" - change your password\r\n"),
            PrintStyledContent("E".dark_green().bold()),
            Print(" - change your email\r\n"),
            PrintStyledContent("N".dark_green().bold()),
            Print(" - change your FULL name\r\n"),
            PrintStyledContent("V".dark_green().bold()),
            Print(" - change your privacy level\r\n"),
        )
        .unwrap();

        if utils::is_barcode(&user_info.username) {
            queue!(
                terminal_io.writer,
                PrintStyledContent("U".dark_green().bold()),
                Print(" - change your username\r\n"),
            )
            .unwrap();
        }

        queue!(
            terminal_io.writer,
            PrintStyledContent("<enter>".dark_green().bold()),
            Print(" - exit settings\r\n"),
        )
        .unwrap();

        execute!(
            terminal_io.writer,
            Print(&format!(
                "\nDear {}, your saldo is {} > ",
                user_info.username,
                utils::format_money(&user_info.money_balance)
            ))
        )
        .unwrap();

        let mut command = String::new();
        loop {
            match terminal_io.recv.recv_timeout(INPUT_TIMEOUT_LONG) {
                Err(RecvTimeoutError::Timeout) => return TimeoutResult::TIMEOUT,
                Ok(InputEvent::Terminal(Event::Key(ev))) => match ev.code {
                    KeyCode::Char(c) => match c.to_ascii_lowercase() {
                        'r' => {
                            printline(terminal_io, "");
                            match change_user_rfid(terminal_io, &credentials) {
                                TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
                                TimeoutResult::RESULT(_) => (),
                            }
                            printline(terminal_io, "");
                            break;
                        }
                        'v' => {
                            printline(terminal_io, "");
                            match change_privacy(INPUT_TIMEOUT_SHORT, terminal_io, &credentials) {
                                TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
                                TimeoutResult::RESULT(_) => (),
                            }
                            printline(terminal_io, "");
                            break;
                        }
                        'p' => {
                            printline(terminal_io, "\n");
                            match change_user_password_user(
                                INPUT_TIMEOUT_LONG,
                                terminal_io,
                                &credentials,
                            ) {
                                TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
                                TimeoutResult::RESULT(_) => (),
                            }
                            printline(terminal_io, "");
                            break;
                        }
                        'n' => {
                            printline(terminal_io, "");
                            match change_real_name(INPUT_TIMEOUT_LONG, terminal_io, &credentials) {
                                TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
                                TimeoutResult::RESULT(_) => (),
                            }
                            printline(terminal_io, "");
                            break;
                        }
                        'e' => {
                            printline(terminal_io, "\n");
                            match change_user_email(INPUT_TIMEOUT_LONG, terminal_io, &credentials) {
                                TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
                                TimeoutResult::RESULT(_) => (),
                            }
                            printline(terminal_io, "");
                            break;
                        }
                        'u' => {
                            if utils::is_barcode(&user_info.username) {
                                printline(terminal_io, "");
                                match change_username(INPUT_TIMEOUT_LONG, terminal_io, &credentials)
                                {
                                    TimeoutResult::TIMEOUT => return TimeoutResult::TIMEOUT,
                                    TimeoutResult::RESULT(_) => (),
                                }
                                printline(terminal_io, "");
                            }
                            break;
                        }
                        'q' => {
                            // for jääräs
                            clear_terminal(terminal_io);
                            break 'main;
                        }
                        _ => {}
                    },
                    KeyCode::Backspace => {
                        if !command.is_empty() {
                            execute!(
                                terminal_io.writer,
                                cursor::MoveLeft(1),
                                Print(" "),
                                cursor::MoveLeft(1)
                            )
                            .unwrap();
                            command.pop();
                        }
                    }
                    KeyCode::Enter => {
                        command = command.trim().to_string();
                        utils::printline(terminal_io, "\r\n");
                        if command.is_empty() {
                            clear_terminal(terminal_io);
                            break 'main;
                        } else {
                            utils::print_error_line(
                                terminal_io,
                                &format!("unknown command: {}\r\n", &command),
                            );
                            break;
                        }
                    }
                    KeyCode::F(5) => {
                        break;
                    }
                    _ => (),
                },
                _ => (),
            }
        }
    }
    TimeoutResult::RESULT(())
}