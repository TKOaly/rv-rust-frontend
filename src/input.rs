use crossterm;
use evdev::{self, InputEventKind};
use rusb::{Context, UsbContext};
use std::{
    sync::{
        self,
        mpsc::{Receiver, Sender},
    },
    thread,
};

const RFID_VENDOR: u16 = 0x413d;
const RFID_PRODUCT: u16 = 0x2107;

pub enum InputEvent {
    Rfid(String),
    Terminal(crossterm::event::Event),
}

struct HotPlugHandler {
    chan: Sender<InputEvent>,
}

fn get_rfid_device() -> Option<evdev::Device> {
    for device in evdev::enumerate().map(|v| v.1) {
        let input_id = device.input_id();
        if input_id.vendor() == RFID_VENDOR && input_id.product() == RFID_PRODUCT {
            return Some(device);
        }
    }
    None
}

impl<T: rusb::UsbContext> rusb::Hotplug<T> for HotPlugHandler {
    fn device_arrived(&mut self, _device: rusb::Device<T>) {
        let sender = self.chan.clone();
        thread::spawn(move || {
            let Some(mut device) = get_rfid_device() else {
                // some other device
                return;
            };

            device.grab().unwrap();
            let mut rfid = String::new();
            loop {
                let Ok(ev) = device.fetch_events() else {
                    // device probably disconnected
                    return;
                };
                for e in ev.filter(|e| e.value() == 0) {
                    if let InputEventKind::Key(k) = e.kind() {
                        match k {
                            evdev::Key::KEY_0 => rfid.push('0'),
                            evdev::Key::KEY_1 => rfid.push('1'),
                            evdev::Key::KEY_2 => rfid.push('2'),
                            evdev::Key::KEY_3 => rfid.push('3'),
                            evdev::Key::KEY_4 => rfid.push('4'),
                            evdev::Key::KEY_5 => rfid.push('5'),
                            evdev::Key::KEY_6 => rfid.push('6'),
                            evdev::Key::KEY_7 => rfid.push('7'),
                            evdev::Key::KEY_8 => rfid.push('8'),
                            evdev::Key::KEY_9 => rfid.push('9'),
                            evdev::Key::KEY_A => rfid.push('a'),
                            evdev::Key::KEY_B => rfid.push('b'),
                            evdev::Key::KEY_C => rfid.push('c'),
                            evdev::Key::KEY_D => rfid.push('d'),
                            evdev::Key::KEY_E => rfid.push('e'),
                            evdev::Key::KEY_F => rfid.push('f'),
                            evdev::Key::KEY_ENTER => {
                                sender.send(InputEvent::Rfid(rfid.to_string())).unwrap();
                                rfid.clear();
                            }
                            _ => (),
                        }
                    }
                }
            }
        });
    }
    fn device_left(&mut self, _device: rusb::Device<T>) {}
}

// Call only once
pub fn init() -> Receiver<InputEvent> {
    let (sender, receiver) = sync::mpsc::channel::<InputEvent>();

    let sender2 = sender.clone(); // Terminal input
    thread::spawn(move || loop {
        let ev = crossterm::event::read().unwrap();
        sender2.send(InputEvent::Terminal(ev)).unwrap();
    });
    let sender2 = sender.clone(); // RFID input
    thread::spawn(move || {
        let ctx = rusb::Context::new().unwrap();
        let _reg: rusb::Registration<Context> = Some(
            rusb::HotplugBuilder::new()
                .enumerate(true)
                .vendor_id(RFID_VENDOR)
                .product_id(RFID_PRODUCT)
                .register(&ctx, Box::new(HotPlugHandler { chan: sender2 }))
                .unwrap(),
        )
        .unwrap();
        loop {
            ctx.handle_events(None).unwrap()
        }
    });
    receiver
}
