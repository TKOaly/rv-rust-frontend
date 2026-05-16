use crossterm::{
    self,
    event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers},
};
use evdev::{self, EventSummary};
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
    time::{Duration, Instant},
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

fn rfid_key_to_char(key: evdev::KeyCode) -> Option<char> {
    match key {
        evdev::KeyCode::KEY_0 => Some('0'),
        evdev::KeyCode::KEY_1 => Some('1'),
        evdev::KeyCode::KEY_2 => Some('2'),
        evdev::KeyCode::KEY_3 => Some('3'),
        evdev::KeyCode::KEY_4 => Some('4'),
        evdev::KeyCode::KEY_5 => Some('5'),
        evdev::KeyCode::KEY_6 => Some('6'),
        evdev::KeyCode::KEY_7 => Some('7'),
        evdev::KeyCode::KEY_8 => Some('8'),
        evdev::KeyCode::KEY_9 => Some('9'),
        evdev::KeyCode::KEY_A => Some('a'),
        evdev::KeyCode::KEY_B => Some('b'),
        evdev::KeyCode::KEY_C => Some('c'),
        evdev::KeyCode::KEY_D => Some('d'),
        evdev::KeyCode::KEY_E => Some('e'),
        evdev::KeyCode::KEY_F => Some('f'),
        _ => None,
    }
}

fn barcode_key_to_char(key: evdev::KeyCode) -> Option<char> {
    match key {
        evdev::KeyCode::KEY_0 => Some('0'),
        evdev::KeyCode::KEY_1 => Some('1'),
        evdev::KeyCode::KEY_2 => Some('2'),
        evdev::KeyCode::KEY_3 => Some('3'),
        evdev::KeyCode::KEY_4 => Some('4'),
        evdev::KeyCode::KEY_5 => Some('5'),
        evdev::KeyCode::KEY_6 => Some('6'),
        evdev::KeyCode::KEY_7 => Some('7'),
        evdev::KeyCode::KEY_8 => Some('8'),
        evdev::KeyCode::KEY_9 => Some('9'),
        evdev::KeyCode::KEY_A => Some('a'),
        evdev::KeyCode::KEY_B => Some('b'),
        evdev::KeyCode::KEY_C => Some('c'),
        evdev::KeyCode::KEY_D => Some('d'),
        evdev::KeyCode::KEY_E => Some('e'),
        evdev::KeyCode::KEY_F => Some('f'),
        evdev::KeyCode::KEY_G => Some('g'),
        evdev::KeyCode::KEY_H => Some('h'),
        evdev::KeyCode::KEY_I => Some('i'),
        evdev::KeyCode::KEY_J => Some('j'),
        evdev::KeyCode::KEY_K => Some('k'),
        evdev::KeyCode::KEY_L => Some('l'),
        evdev::KeyCode::KEY_M => Some('m'),
        evdev::KeyCode::KEY_N => Some('n'),
        evdev::KeyCode::KEY_O => Some('o'),
        evdev::KeyCode::KEY_P => Some('p'),
        evdev::KeyCode::KEY_Q => Some('q'),
        evdev::KeyCode::KEY_R => Some('r'),
        evdev::KeyCode::KEY_S => Some('s'),
        evdev::KeyCode::KEY_T => Some('t'),
        evdev::KeyCode::KEY_U => Some('u'),
        evdev::KeyCode::KEY_V => Some('v'),
        evdev::KeyCode::KEY_W => Some('w'),
        evdev::KeyCode::KEY_X => Some('x'),
        evdev::KeyCode::KEY_Y => Some('y'),
        evdev::KeyCode::KEY_Z => Some('z'),
        evdev::KeyCode::KEY_SLASH => Some('/'),
        evdev::KeyCode::KEY_SEMICOLON => Some(':'),
        evdev::KeyCode::KEY_EQUAL => Some('='),
        evdev::KeyCode::KEY_MINUS => Some('-'),
        evdev::KeyCode::KEY_DOT => Some('.'),
        _ => None,
    }
}

fn get_device(vendor_id: u16, product_id: u16) -> Option<evdev::Device> {
    for device in evdev::enumerate() {
        let input_id = device.1.input_id();
        if input_id.vendor() == vendor_id && input_id.product() == product_id {
            return Some(device.1);
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
    K: Fn(evdev::KeyCode) -> Option<char>,
{
    if let Some(mut device) = get_device(vendor, product) {
        if device.is_grabbed() {
            return;
        }

        let start = Instant::now();
        loop {
            match device.grab() {
                Ok(_) => break,
                Err(_) => {
                    if start.elapsed() > Duration::from_secs(1) {
                        return;
                    }
                    thread::sleep(Duration::from_millis(50));
                }
            }
        }

        let mut input = String::new();
        loop {
            let Ok(ev) = device.fetch_events() else {
                return;
            };
            for e in ev.filter(|e| e.value() == 1) {
                if let EventSummary::Key(_, k, _) = e.destructure() {
                    if k == evdev::KeyCode::KEY_ENTER {
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
        "Space" => Some(InputEvent::Terminal(crossterm::event::Event::Key(
            KeyEvent {
                code: KeyCode::Char(' '),
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
