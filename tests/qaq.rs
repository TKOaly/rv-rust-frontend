use crossterm::{self, event};
use regex::Regex;
use rvterminal::{self, main_loop, TerminalIO, TerminalWriter};
use std::sync::mpsc::Sender;

fn send_string_to_channel(str: &str, sender: &Sender<rvterminal::input::InputEvent>) {
    for c in str.chars() {
        sender
            .send(rvterminal::input::InputEvent::Terminal(event::Event::Key(
                event::KeyEvent::new(event::KeyCode::Char(c), event::KeyModifiers::NONE),
            )))
            .unwrap();
    }
}

fn send_enter_to_channel(sender: &Sender<rvterminal::input::InputEvent>) {
    sender
        .send(rvterminal::input::InputEvent::Terminal(event::Event::Key(
            event::KeyEvent::new(event::KeyCode::Enter, event::KeyModifiers::NONE),
        )))
        .unwrap();
}

#[test]
fn can_quit_main_loop() {
    let (sender, receiver) = std::sync::mpsc::channel::<rvterminal::input::InputEvent>();
    let mut terminal_io = TerminalIO {
        recv: receiver,
        writer: TerminalWriter::new(true),
    };
    send_string_to_channel("quit", &sender);
    send_enter_to_channel(&sender);
    drop(sender);
    main_loop(&mut terminal_io).unwrap();
}

#[test]
fn login_test() {
    let (sender, receiver) = std::sync::mpsc::channel::<rvterminal::input::InputEvent>();
    let mut terminal_io = TerminalIO {
        recv: receiver,
        writer: TerminalWriter::new(true),
    };
    send_string_to_channel("test", &sender);
    send_enter_to_channel(&sender);
    send_string_to_channel("test", &sender);
    send_enter_to_channel(&sender);
    send_enter_to_channel(&sender);
    send_string_to_channel("quit", &sender);
    send_enter_to_channel(&sender);
    drop(sender);
    main_loop(&mut terminal_io).unwrap();
    //terminal_io.writer.flush().unwrap();
    let out_str = String::from_utf8(terminal_io.writer.test_output).unwrap();
    let re = Regex::new(r"Dear test").unwrap();
    assert!(re.is_match(&out_str));
}
