use std::io::{Read, Write};
use std::time::Duration;
use crc::{Crc, CRC_16_MODBUS};
use glib_macros::clone;

use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Builder, Button, ComboBoxText, Switch};

fn main() {
    let application = Application::new(
        Some("com.github.reticulis.rs485-control"),
        Default::default(),
    );
    application.connect_activate(build_ui);
    application.run();
}

fn build_ui(application: &Application) {

    let ui_src = include_str!("rs485a.ui");
    let builder = Builder::from_string(ui_src);

    let window: ApplicationWindow = builder.object("window").expect("Couldn't get window");
    window.set_application(Some(application));

    let relays: ComboBoxText = builder.object("relays").expect("Couldn't get window");
    let devices: ComboBoxText = builder.object("devices").expect("Couldn't get window");
    let on_off_switch: Switch = builder.object("on_off_switch").expect("Couldn't get window");

    for i in 1..17 {
        relays.append_text(&*format!("Relay: {}", i))
    }

    for device in serialport::available_ports().expect("No ports found!") {
        devices.append_text(&*device.port_name)
    }

    devices.set_active(Some(0));
    relays.set_active(Some(0));

    on_off_switch.connect_active_notify(clone!(@weak relays => move |s| {
        match s.is_active() {
            true => rs485_write(&hex_relays(relays.active().unwrap() as u8, 1)),
            false => rs485_write(&hex_relays(relays.active().unwrap() as u8, 2))
        }
    }));

    relays.connect_active_notify(move |_| {
        rs485_write(&check_relay(0));
        rs485_read(&[0])
    });

    window.show();
}

fn rs485_write(buf: &[u8]) {
    let mut port = serialport::new("/dev/ttyUSB1", 9600)
        .timeout(Duration::from_millis(10))
        .open().expect("Failed to open port");
    port.write(buf).unwrap();
}

fn rs485_read(buf: &[u8]) {
    let mut port = serialport::new("/dev/ttyUSB1", 9600)
        .timeout(Duration::from_millis(0))
        .open_native().expect("Failed to open port");
    let mut serial_buf: Vec<u8> = vec![0; 32];
    port.read_to_end(&mut serial_buf).expect("Found no data!");
    println!("{:?}", serial_buf)
}

fn hex_relays(c: u8, oc: u8) -> Vec<u8> {
    let mut relay = vec![0x01, 0x06, 0x00, c+1_u8, oc, 0x00];
    let checksum = Crc::<u16>::new(&CRC_16_MODBUS)
        .checksum(&relay);
    relay.push(((checksum << 8) >> 8) as u8);
    relay.push((checksum >> 8) as u8);

    relay
}

fn check_relay(id: u8) -> Vec<u8> {
    // let mut relay = vec![0x01, 0x03, 0x00, 0x01, 0x00, 0x01];
    // relay
    unimplemented!()
}
