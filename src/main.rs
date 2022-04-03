use chrono::Timelike;
use crc::{Crc, CRC_16_MODBUS};
use glib_macros::clone;
use std::io::{Read, Write};
use std::rc::Rc;
use std::time::Duration;

use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Builder, Button, ComboBoxText, ScrolledWindow, Switch,
    TextBuffer, TextView,
};
use serialport::{Error, SerialPort, SerialPortInfo};

struct App {
    // TODO, REQUIRE IMPROVE
    ports: Result<Vec<SerialPortInfo>, String>,
}

impl App {
    fn new() -> App {
        App {
            ports: refresh_devices(),
        }
    }
    fn run(self) {
        let application = Application::new(
            Some("com.github.reticulis.rs485-control"),
            Default::default(),
        );
        application.connect_activate(move |f| self.build_ui(f));
        application.run();
    }

    fn build_ui(&self, application: &Application) {
        let ui_src = include_str!("rs485a.ui");
        let builder = Builder::from_string(ui_src);

        let window: ApplicationWindow = builder
            .object("window")
            .expect("Couldn't get window");
        window.set_application(Some(application));

        let relays: ComboBoxText = builder
            .object("relays")
            .expect("Couldn't get window");
        let devices_list: ComboBoxText = builder
            .object("devices")
            .expect("Couldn't get window");
        let on_off_switch: Switch = builder
            .object("on_off_switch")
            .expect("Couldn't get window");
        let refresh_button: Button = builder
            .object("refresh_button")
            .expect("Couldn't get window");
        let scrolled_window: ScrolledWindow = builder
            .object("scrolled_window")
            .expect("Couldn't get window");
        let text_view: TextView = builder
            .object("text_view")
            .expect("Couldn't get window");
        let text = text_view.buffer();

        // TODO
        for i in 1..17 {
            relays.append_text(&*format!("Relay: {}", i))
        }

        match refresh_devices() {
            Ok(d) => {
                for name in d {
                    devices_list.append_text(&*name.port_name);
                }
            }
            Err(e) => devices_list.append_text(&*e),
        }

        devices_list.set_active(Some(0));
        relays.set_active(Some(0));

        relays.set_sensitive(false);
        on_off_switch.set_sensitive(false);

        let ports = &self.ports.clone().unwrap();

        try_connect_to_device(ports, 0, &text, &relays, &on_off_switch);

        devices_list.connect_active_notify(clone!(@weak on_off_switch, @weak relays, @strong ports, @weak text => move |devices_list| {
            try_connect_to_device(&ports, devices_list.active().unwrap() as usize, &text, &relays, &on_off_switch);
        }));

        refresh_button.connect_clicked(clone!(@weak devices_list, @strong ports => move |_| {
            devices_list.remove_all();
            match refresh_devices() {
                Ok(d) => {
                    for name in d {
                        devices_list.append_text(&*name.port_name);
                    }
                }
                Err(e) => devices_list.append_text(&*e)
            };
            devices_list.set_active(Some(0));
        }));

        let on_off_switch_handler_id = Rc::new(on_off_switch.connect_state_notify(clone!(@weak relays, @weak text, @weak text_view, @strong ports, @weak devices_list => move |switch| {
            let mut port = set_port(&ports, devices_list.active().unwrap() as usize).unwrap();
            match switch.is_active() {
                true => {
                    let buf = control_command(relays.active().unwrap() as u8, 1);
                    rs485_write(&mut port, &buf);
                    let time = chrono::Local::now();
                    text.insert(&mut text.end_iter(), &*format!("{:>02}:{:>02}:{:>02} Sent: {:X?}\n", time.hour(), time.minute(), time.second(),&buf));
                },
                false => {
                    let buf = control_command(relays.active().unwrap() as u8, 2);
                    rs485_write(&mut port, &buf);
                    let time = chrono::Local::now();
                    text.insert(&mut text.end_iter(), &*format!("{:>02}:{:>02}:{:>02} Sent: {:X?}\n", time.hour(), time.minute(), time.second(),&buf));
                }
            }
        })));

        relays.connect_active_notify(clone!(@weak on_off_switch, @strong ports, @weak text, @weak text_view, @strong on_off_switch_handler_id => move |relay| {
            let mut port = set_port(&ports, devices_list.active().unwrap() as usize).unwrap();
            let buf = read_status_command(relay.active().unwrap() as u8);
            rs485_write(&mut port, &buf);
            let data = rs485_read(&mut port);
            dbg!(&buf);
            dbg!(&data);
            dbg!(relay.active().unwrap());
            let time = chrono::Local::now();
            text.insert(&mut text.end_iter(), &*format!("{:>02}:{:>02}:{:>02} Sent: {:X?}\n{:>02}:{:>02}:{:>02} Received: {:X?}\n", time.hour(), time.minute(), time.second(), &buf, time.hour(), time.minute(), time.second(), &data));
            on_off_switch.block_signal(&on_off_switch_handler_id);
            if data == [0x01, 0x03, 0x02, 0x00, 0x01, 0x79, 0x84] {
                on_off_switch.set_state(true);
            } else if data == [0x01, 0x03, 0x02, 0x00, 0x00, 0xB8, 0x44] {
                on_off_switch.set_state(false);
            } else {
                text.insert(&mut text.end_iter(), &*format!("{:>02}:{:>02}:{:>02} Valid data! : {:X?}\n", time.hour(), time.minute(), time.second(), &data))
            };
            on_off_switch.unblock_signal(&on_off_switch_handler_id);
        }));

        text.connect_changed(clone!(@weak scrolled_window, @weak text_view => move |text| {
            if scrolled_window.vadjustment().upper() != scrolled_window.vadjustment().page_size() {
                let t = text.create_mark(Some("screen"), &text.end_iter(), false);
                text_view.scroll_to_mark(&t, 0., true, 0., 0.);
            }
        }));

        // TEST ASCII
        // let mut port = set_port(&ports);
        // rs485_write(&mut port, &[0x41, 0x54, 0x2B, 0x4F, 0x31]); // AT+O1
        // println!("{:X?}", rs485_read(&mut port).iter().map(|&x| x as char).collect::<Vec<char>>());

        window.show();
    }
}

fn refresh_devices() -> Result<Vec<SerialPortInfo>, String> {
    match serialport::available_ports() {
        Ok(s) => Ok(s),
        Err(e) => Err(format!("Not found ports! {:?}", e)),
    }
}

fn rs485_write(port: &mut Box<dyn SerialPort>, buf: &[u8]) {
    port.write(&buf).unwrap();
}

fn rs485_write_ascii() {
    unimplemented!()
}

fn rs485_read(port: &mut Box<dyn SerialPort>) -> [u8; 7] {
    let mut read_buf: [u8; 7] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    port.read(&mut read_buf).expect("Found no data!");
    read_buf
}

fn rs485_read_ascii() {
    unimplemented!()
}

fn control_command(id: u8, command: u8) -> Vec<u8> {
    let mut control = vec![0x01, 0x06, 0x00, id + 1, command, 0x00];
    let checksum = Crc::<u16>::new(&CRC_16_MODBUS).checksum(&control);
    control.push(((checksum << 8) >> 8) as u8);
    control.push((checksum >> 8) as u8);

    control
}

fn read_status_command(id: u8) -> Vec<u8> {
    let mut read_status = vec![0x01, 0x03, 0x00, id + 1, 0x00, 0x01];
    let checksum = Crc::<u16>::new(&CRC_16_MODBUS).checksum(&read_status);
    read_status.push(((checksum << 8) >> 8) as u8);
    read_status.push((checksum >> 8) as u8);

    read_status
}

// TODO
fn set_port(ports: &Vec<SerialPortInfo>, id: usize) -> Result<Box<dyn SerialPort>, Error> {
    match serialport::new(&*ports[id].port_name, 9600)
        .timeout(Duration::from_millis(100))
        .open()
    {
        Ok(e) => Ok(e),
        Err(e) => Err(e),
    }
}

fn try_connect_to_device(
    ports: &Vec<SerialPortInfo>,
    id: usize,
    text: &TextBuffer,
    relays: &ComboBoxText,
    on_off_switch: &Switch,
) {
    let time = chrono::Local::now();
    match set_port(&ports, id) {
        Ok(_) => {
            text.insert(
                &mut text.end_iter(),
                &*format!(
                    "{:>02}:{:>02}:{:>02} Connected!\n",
                    time.hour(),
                    time.minute(),
                    time.second()
                ),
            );
            relays.set_sensitive(true);
            on_off_switch.set_sensitive(true);
        }
        Err(e) => {
            text.insert(
                &mut text.end_iter(),
                &*format!(
                    "{:>02}:{:>02}:{:>02} Failed connecting! {}\n",
                    time.hour(),
                    time.minute(),
                    time.second(),
                    e
                ),
            );
            relays.set_sensitive(false);
            on_off_switch.set_sensitive(false);
        }
    };
}

fn main() {
    let app = App::new();
    app.run()
}
