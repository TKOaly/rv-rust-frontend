use crossterm::{
    self,
    event::{self, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers},
};
use evdev::{self, InputEventKind};
use rusb::{Context, UsbContext};
use std::{
    fs,
    io::{BufRead, BufReader},
    os::unix::net::UnixListener,
    path::Path,
    sync::{
        self,
        mpsc::{Receiver, Sender},
    },
    thread,
};

use crate::DEVELOPMENT_MODE;

const RFID_VENDOR: u16 = 0x413d;
const RFID_PRODUCT: u16 = 0x2107;

const BARCODE1_VENDOR: u16 = 0x24ea;
const BARCODE1_PRODUCT: u16 = 0x0197;

const BARCODE2_VENDOR: u16 = 0x04d9;
const BARCODE2_PRODUCT: u16 = 0x1400;

#[derive(Debug)]
pub enum InputEvent {
    Rfid(String),
    Barcode(String),
    Terminal(crossterm::event::Event),
}

struct HotPlugHandler {
    chan: Sender<InputEvent>,
}

fn rfid_key_to_char(key: evdev::Key) -> Option<char> {
    match key {
        evdev::Key::KEY_0 => Some('0'),
        evdev::Key::KEY_1 => Some('1'),
        evdev::Key::KEY_2 => Some('2'),
        evdev::Key::KEY_3 => Some('3'),
        evdev::Key::KEY_4 => Some('4'),
        evdev::Key::KEY_5 => Some('5'),
        evdev::Key::KEY_6 => Some('6'),
        evdev::Key::KEY_7 => Some('7'),
        evdev::Key::KEY_8 => Some('8'),
        evdev::Key::KEY_9 => Some('9'),
        evdev::Key::KEY_A => Some('a'),
        evdev::Key::KEY_B => Some('b'),
        evdev::Key::KEY_C => Some('c'),
        evdev::Key::KEY_D => Some('d'),
        evdev::Key::KEY_E => Some('e'),
        evdev::Key::KEY_F => Some('f'),
        _ => None,
    }
}

fn barcode_key_to_char(key: evdev::Key) -> Option<char> {
    match key {
        evdev::Key::KEY_0 => Some('0'),
        evdev::Key::KEY_1 => Some('1'),
        evdev::Key::KEY_2 => Some('2'),
        evdev::Key::KEY_3 => Some('3'),
        evdev::Key::KEY_4 => Some('4'),
        evdev::Key::KEY_5 => Some('5'),
        evdev::Key::KEY_6 => Some('6'),
        evdev::Key::KEY_7 => Some('7'),
        evdev::Key::KEY_8 => Some('8'),
        evdev::Key::KEY_9 => Some('9'),
        evdev::Key::KEY_A => Some('a'),
        evdev::Key::KEY_B => Some('b'),
        evdev::Key::KEY_C => Some('c'),
        evdev::Key::KEY_D => Some('d'),
        evdev::Key::KEY_E => Some('e'),
        evdev::Key::KEY_F => Some('f'),
        evdev::Key::KEY_G => Some('g'),
        evdev::Key::KEY_H => Some('h'),
        evdev::Key::KEY_I => Some('i'),
        evdev::Key::KEY_J => Some('j'),
        evdev::Key::KEY_K => Some('k'),
        evdev::Key::KEY_L => Some('l'),
        evdev::Key::KEY_M => Some('m'),
        evdev::Key::KEY_N => Some('n'),
        evdev::Key::KEY_O => Some('o'),
        evdev::Key::KEY_P => Some('p'),
        evdev::Key::KEY_Q => Some('q'),
        evdev::Key::KEY_R => Some('r'),
        evdev::Key::KEY_S => Some('s'),
        evdev::Key::KEY_T => Some('t'),
        evdev::Key::KEY_U => Some('u'),
        evdev::Key::KEY_V => Some('v'),
        evdev::Key::KEY_W => Some('w'),
        evdev::Key::KEY_X => Some('x'),
        evdev::Key::KEY_Y => Some('y'),
        evdev::Key::KEY_Z => Some('z'),
        evdev::Key::KEY_SLASH => Some('/'),
        evdev::Key::KEY_SEMICOLON => Some(':'),
        evdev::Key::KEY_EQUAL => Some('='),
        evdev::Key::KEY_MINUS => Some('-'),
        evdev::Key::KEY_DOT => Some('.'),
        _ => None,
    }
}

fn get_device(vendor_id: u16, product_id: u16) -> Option<evdev::Device> {
    for device in evdev::enumerate().map(|v| v.1) {
        let input_id = device.input_id();
        if input_id.vendor() == vendor_id && input_id.product() == product_id {
            return Some(device);
        }
    }
    None
}

fn capture_device_input<K>(
    vendor: u16,
    product: u16,
    key_to_char: K,
    input_event_variant: fn(String) -> InputEvent,
    sender: Sender<InputEvent>,
) where
    K: Fn(evdev::Key) -> Option<char>,
{
    if let Some(mut device) = get_device(vendor, product) {
        device.grab().unwrap();
        let mut input = String::new();
        loop {
            let Ok(ev) = device.fetch_events() else {
                return;
            };
            for e in ev.filter(|e| e.value() == 1) {
                if let InputEventKind::Key(k) = e.kind() {
                    if k == evdev::Key::KEY_ENTER {
                        sender.send(input_event_variant(input.clone())).unwrap();
                        input.clear();
                    } else if let Some(ch) = key_to_char(k) {
                        input.push(ch);
                    }
                }
            }
        }
    }
}

fn register_device_input(vendor: u16, product: u16, sender: Sender<InputEvent>) {
    let ctx = rusb::Context::new().unwrap();
    let _reg: rusb::Registration<Context> = Some(
        rusb::HotplugBuilder::new()
            .enumerate(true)
            .vendor_id(vendor)
            .product_id(product)
            .register(&ctx, Box::new(HotPlugHandler { chan: sender }))
            .unwrap(),
    )
    .unwrap();
    loop {
        ctx.handle_events(None).unwrap()
    }
}

impl<T: rusb::UsbContext> rusb::Hotplug<T> for HotPlugHandler {
    fn device_arrived(&mut self, _device: rusb::Device<T>) {
        // RFID reader
        let sender = self.chan.clone();
        thread::spawn(move || {
            capture_device_input(
                RFID_VENDOR,
                RFID_PRODUCT,
                rfid_key_to_char,
                InputEvent::Rfid,
                sender,
            );
        });

        // Barcode scanner 1
        let sender = self.chan.clone();
        thread::spawn(move || {
            capture_device_input(
                BARCODE1_VENDOR,
                BARCODE1_PRODUCT,
                barcode_key_to_char,
                InputEvent::Barcode,
                sender,
            );
        });

        // Barcode scanner 2
        let sender = self.chan.clone();
        thread::spawn(move || {
            capture_device_input(
                BARCODE2_VENDOR,
                BARCODE2_PRODUCT,
                barcode_key_to_char,
                InputEvent::Barcode,
                sender,
            );
        });
    }
    fn device_left(&mut self, _device: rusb::Device<T>) {}
}

fn deserialize_software_keyboard_input_event(key: &str) -> Option<InputEvent> {
    match key {
        "Enter" => Some(InputEvent::Terminal(crossterm::event::Event::Key(
            KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            },
        ))),
        "Backspace" => Some(InputEvent::Terminal(crossterm::event::Event::Key(
            KeyEvent {
                code: KeyCode::Backspace,
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            },
        ))),
        key => {
            if key.starts_with("Fn") {
                let num = key.replace("Fn", "").parse::<u8>().ok()?;
                return Some(InputEvent::Terminal(crossterm::event::Event::Key(
                    KeyEvent {
                        code: KeyCode::F(num),
                        modifiers: KeyModifiers::NONE,
                        kind: KeyEventKind::Press,
                        state: KeyEventState::NONE,
                    },
                )));
            }
            if let Some(last) = key.chars().last() {
                return Some(InputEvent::Terminal(crossterm::event::Event::Key(
                    KeyEvent {
                        code: KeyCode::Char(last),
                        modifiers: KeyModifiers::NONE,
                        kind: KeyEventKind::Press,
                        state: KeyEventState::NONE,
                    },
                )));
            }
            None
        }
    }
}

fn deserialize_software_input_event(line: &str) -> Result<InputEvent, String> {
    let line = line.trim();
    let pos = line
        .find('|')
        .ok_or_else(|| format!("missing '|' in: {}", line))?;
    let event = &line[..pos];
    let string = &line[pos + 1..];

    match event {
        "Barcode" => Ok(InputEvent::Barcode(string.to_string())),
        "RFID" => Ok(InputEvent::Rfid(string.to_string())),
        "Keyboard" => match deserialize_software_keyboard_input_event(string) {
            Some(event) => Ok(event),
            None => Err(format!("unknown keyboard event: {}", string)),
        },
        other => Err(format!("unknown event: {}", other)),
    }
}

fn software_input(sender: Sender<InputEvent>) {
    thread::spawn(move || {
        let socket_path = "/tmp/rvterminal.sock";
        if Path::new(socket_path).exists() {
            fs::remove_file(socket_path).expect("Failed to remove existing socket");
        }
        let listener = UnixListener::bind(socket_path).expect("Failed to bind");

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let sender = sender.clone();
                    thread::spawn(move || {
                        let reader = BufReader::new(stream);
                        for line in reader.lines() {
                            if let Ok(line) = line {
                                if let Ok(event) = deserialize_software_input_event(&line) {
                                    sender.send(event).unwrap();
                                }
                            }
                        }
                    });
                }
                Err(_) => {
                    break;
                }
            }
        }
    });
}

// Call only once
pub fn init() -> Receiver<InputEvent> {
    let (sender, receiver) = sync::mpsc::channel::<InputEvent>();

    // Terminal input
    let sender2 = sender.clone();
    thread::spawn(move || loop {
        let ev = crossterm::event::read().unwrap();
        sender2.send(InputEvent::Terminal(ev)).unwrap();
    });

    // RFID input
    let sender2 = sender.clone();
    thread::spawn(move || {
        register_device_input(RFID_VENDOR, RFID_PRODUCT, sender2);
    });

    // BARCODE scanner input 1
    let sender2 = sender.clone();
    thread::spawn(move || {
        register_device_input(BARCODE1_VENDOR, BARCODE1_PRODUCT, sender2);
    });

    // BARCODE scanner input 2
    let sender2 = sender.clone();
    thread::spawn(move || {
        register_device_input(BARCODE2_VENDOR, BARCODE2_PRODUCT, sender2);
    });

    if *DEVELOPMENT_MODE {
        software_input(sender);
    }

    receiver
}
