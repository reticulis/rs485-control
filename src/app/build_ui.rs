use super::text_formatting::{
    convert_to_hex_format, convert_text_to_hex, time_execute, convert_hex_to_ascii
};
use super::device::{
    rs485_read, rs485_write, read_status_command, checksum, control_command, try_connect_to_device, set_port
};


use glib_macros::clone;
use std::rc::Rc;
use std::cell::RefCell;

use adw::{Application, ApplicationWindow};
use adw::prelude::*;
use gtk::{
    Builder, Button, CheckButton, ComboBoxText, Entry, ScrolledWindow, SpinButton, Switch, TextView,
};
use crate::app::values::{CGOBJECT, CGWINDOW, NFOUND, RELAY_OPEN, RELAY_CLOSE, TypeData};

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

    let ports = RefCell::new(serialport::available_ports().unwrap());

    if ports.borrow().len() != 0 {
        for name in ports.borrow().iter() {
            devices_list.append_text(&*name.port_name);
        }
        match try_connect_to_device(&ports.borrow(), 0) {
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
            match try_connect_to_device(&ports.borrow(), devices_list.active().unwrap() as u8) {
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
            let buf = match switch.is_active() {
                true => {
                    control_command(relays.value() as u8 - 1, 1)
                },
                false => {
                    control_command(relays.value() as u8 - 1, 2)
                }
            };
            rs485_write(&mut port, &buf);
            let data = rs485_read(&mut port);
            match data {
                Ok(d) => {
                    text.insert(
                        &mut text.end_iter(),
                        &*format!("\n{} Sent: {}\n{} Received: {}\n", time_execute(), convert_to_hex_format(&buf), time_execute(), convert_to_hex_format(&d))
                    );
                }
                Err(e) => {
                    text.insert(
                        &mut text.end_iter(),
                        &*format!("{} Error reading data! : {:X?}\n", time_execute(), e.description)
                    )
                }
            }
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

    send_button.connect_clicked(
        clone!(
            @weak entry_command,
            @weak crc_check_button,
            @weak text,
            @strong ports,
            @weak devices_list,
            @strong crc_check_button
            => move |_| {
                let mut port = set_port(
                    &ports.borrow(),
                    devices_list.active().unwrap() as u8
                ).unwrap();

                let entry_text = entry_command.text();

                match convert_text_to_hex(entry_text.to_string()) {
                    Ok((mut v, t)) => {
                        if crc_check_button.is_active() {
                            checksum(&mut v);
                        }
                        rs485_write(&mut port, &v);
                        let data = rs485_read(&mut port);
                        match data {
                            Ok(d) => {
                                let received = match t {
                                    TypeData::ASCII => convert_hex_to_ascii(d),
                                    TypeData::MODBUS => convert_to_hex_format(&d)
                                };
                                text.insert(
                                    &mut text.end_iter(),
                                    &*format!("\n{} Command sent: {}\n{} Received: {:?}\n", time_execute(), entry_text.to_string(), time_execute(), received)
                                );
                            }
                            Err(e) => {
                                text.insert(
                                    &mut text.end_iter(),
                                    &*format!("{} Error reading data! : {:X?}\n", time_execute(), e.description)
                                )
                            }
                        }
                    }
                    Err(e) => {
                        text.insert(
                            &mut text.end_iter(),
                            &*format!("{} {}\n", time_execute(), e)
                        );
                    }
                }
            }
        ));

    window.show();
}