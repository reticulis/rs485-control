use std::cell::RefCell;
use super::text_formatting::{convert_to_hex_format, convert_text_to_hex, time_execute};
use super::device::{rs485_read, rs485_write, read_status_command, control_command, try_connect_to_device, set_port};

use glib_macros::clone;
use std::rc::Rc;

use adw::{Application, ApplicationWindow};
use gtk::prelude::*;
use gtk::{
    Builder, Button, CheckButton, ComboBoxText, Entry, ScrolledWindow, SpinButton, Switch, TextView,
};
use crate::app::values::{CGOBJECT, CGWINDOW, NFOUND, RELAY_OPEN, RELAY_CLOSE};

pub fn build_ui(application: &Application) {
    let ui_src = include_str!("../rs485.ui");
    let builder = Builder::from_string(ui_src);

    let window: ApplicationWindow = builder
        .object("window")
        .expect(CGWINDOW);
    window.set_application(Some(application));

    let relays: SpinButton = builder
        .object("relays")
        .expect(CGOBJECT);
    let devices_list: ComboBoxText = builder
        .object("devices")
        .expect(CGOBJECT);
    let on_off_switch: Switch = builder
        .object("on_off_switch")
        .expect(CGOBJECT);
    let refresh_button: Button = builder
        .object("refresh_button")
        .expect(CGOBJECT);
    let scrolled_window: ScrolledWindow = builder
        .object("scrolled_window")
        .expect(CGOBJECT);
    let text_view: TextView = builder
        .object("text_view")
        .expect(CGOBJECT);
    let entry_command: Entry = builder
        .object("entry_command")
        .expect(CGOBJECT);
    let crc_check_button: CheckButton = builder
        .object("crc_check_button")
        .expect(CGOBJECT);
    let send_button: Button = builder
        .object("send_button")
        .expect(CGOBJECT);
    let text = text_view.buffer();

    let ports= serialport::available_ports().unwrap();

    if ports.len() != 0 {
        for name in ports {
            devices_list.append_text(&*name.port_name);
        }
        match try_connect_to_device(&ports, 0) {
            (b, s) => {
                relays.set_sensitive(b);
                on_off_switch.set_sensitive(b);
                entry_command.set_sensitive(b);
                crc_check_button.set_sensitive(b);
                send_button.set_sensitive(b);
                text.insert(
                    &mut text.end_iter(),
                    &*format!("{} {}", time_execute(), s)
                );
                devices_list.set_active(Some(0));
            }
        }
    } else {
        text.insert(
            &mut text.end_iter(),
            NFOUND
        );
    }

    devices_list.connect_active_notify(clone!(
        @weak on_off_switch,
        @weak relays,
        @strong ports,
        @weak text,
        @weak send_button,
        @weak entry_command,
        @weak crc_check_button
        => move |devices_list| {
            match try_connect_to_device(&ports, devices_list.active().unwrap() as u8) {
                (b, s) => {
                    relays.set_sensitive(b);
                    on_off_switch.set_sensitive(b);
                    entry_command.set_sensitive(b);
                    crc_check_button.set_sensitive(b);
                    send_button.set_sensitive(b);
                    text.insert(
                        &mut text.end_iter(),
                        &*format!("{} {}", time_execute(), s)
                    );
                }
            }
        }
    ));

    let ports = RefCell::new(ports);

    refresh_button.connect_clicked(clone!(@weak devices_list, @strong ports => move |_| {
        devices_list.remove_all();
        for name in serialport::available_ports().unwrap() {
            devices_list.append_text(&*name.port_name);
        }

        let mut  ports = ports.borrow_mut();

        *ports = serialport::available_ports().unwrap();

        devices_list.set_active(Some(0));
    }));

    let on_off_switch_handler_id = Rc::new(on_off_switch.connect_active_notify(clone!(
        @weak relays,
        @weak text,
        @weak text_view,
        @strong ports,
        @weak devices_list
        => move |switch| {
            let mut port = set_port(
                &ports.borrow(),
                devices_list.active().unwrap() as u8
            ).unwrap();
            let buf: Vec<u8>;
            match switch.is_active() {
                true => {
                    buf = control_command(relays.value() as u8 - 1, 1);

                },
                false => {
                    buf = control_command(relays.value() as u8 - 1, 2);
                }
            }
            rs485_write(&mut port, &buf);
            text.insert(
                &mut text.end_iter(),
                &*format!("\n{} Sent: {}\n", time_execute(), convert_to_hex_format(&buf))
            );
        }
    )));

    relays.connect_value_changed(clone!(
        @weak on_off_switch,
        @strong ports,
        @weak text,
        @weak text_view,
        @strong on_off_switch_handler_id,
        @weak devices_list
        => move |relay| {
            let mut port = set_port(
                &ports.borrow(), devices_list.active().unwrap() as u8
            ).unwrap();
            let buf = read_status_command(relay.value() as u8 - 1);
            rs485_write(&mut port, &buf);
            match rs485_read(&mut port) {
                Ok(data) => {
                    text.insert(
                        &mut text.end_iter(),
                        &*format!("\n{} Sent: {}\n{} Received: {}\n",
                            time_execute(),
                            convert_to_hex_format(&buf),
                            time_execute(),
                            convert_to_hex_format(&data)
                        )
                     );
                    on_off_switch.block_signal(&on_off_switch_handler_id);
                    match data {
                         o if o == RELAY_OPEN => on_off_switch.set_state(true),
                         c if c == RELAY_CLOSE=> on_off_switch.set_state(false),
                         _ => {
                            text.insert(
                                &mut text.end_iter(),
                                &*format!("{} Valid data! : {:X?}\n", time_execute(), &data)
                            )
                        }
                    }
                    on_off_switch.unblock_signal(&on_off_switch_handler_id);
                }
                Err(e) => {
                    text.insert(
                        &mut text.end_iter(),
                        &*format!("{} Error reading data! : {:X?}\n", time_execute(), e.description)
                    )
                }
            };

        }
    ));

    text.connect_changed(
        clone!(@weak scrolled_window, @weak text_view => move |text| {
            if scrolled_window.vadjustment().upper() != scrolled_window.vadjustment().page_size() {
                let t = text.create_mark(Some("screen"), &text.end_iter(), false);
                text_view.scroll_to_mark(&t, 0., true, 0., 0.);
            }
        }),
    );

    send_button.connect_clicked(clone!(@weak entry_command, @weak crc_check_button, @weak text, @strong ports, @weak devices_list => move |_| {
        let mut port = set_port(
                    &ports.borrow(),
                    devices_list.active().unwrap() as u8
                ).unwrap();
        match convert_text_to_hex(entry_command.text().to_string()) {
            Ok(v) => {
                text.insert(
                    &mut text.end_iter(),
                    &*format!("{} Command sent: {}\n", time_execute(), entry_command.text().to_string())
                );
                rs485_write(&mut port, &v);
            }
            Err(e) => {
                text.insert(
                    &mut text.end_iter(),
                    &*format!("{} {}\n", time_execute(), e)
                );
            }
        }
    }));

    // TEST ASCII
    // let mut port = set_port(&ports);
    // rs485_write(&mut port, &[0x41, 0x54, 0x2B, 0x4F, 0x31]); // AT+O1
    // println!("{:X?}", rs485_read(&mut port).iter().map(|&x| x as char).collect::<Vec<char>>());

    window.show();
}