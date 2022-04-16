use super::device::{
    checksum, control_command, read_status_command, rs485_read, rs485_write, set_port,
    try_connect_to_device,
};
use super::text_formatting::{
    convert_hex_to_ascii, convert_text_to_hex, convert_to_hex_format, time_execute,
};

use glib_macros::clone;
use std::cell::RefCell;

use crate::app::values::{TypeData, CGOBJECT, CGWINDOW, NFOUND, RELAY_CLOSE, RELAY_OPEN};
use adw::prelude::*;
use adw::{Application, ApplicationWindow};
use glib::SignalHandlerId;
use gtk::{
    Builder, Button, CheckButton, ComboBoxText, Entry, ScrolledWindow, SpinButton, Switch,
    TextBuffer, TextView,
};
use serialport::SerialPortInfo;

pub struct App;

impl App {
    pub fn run(application: &Application) {
        let window = Window::new(application);
        let ui = UI::new(window.builder);
        ui.build();
        window.window.show();
    }
}

pub struct Window {
    pub builder: Builder,
    pub window: ApplicationWindow,
}

impl Window {
    pub fn new(application: &Application) -> Window {
        let ui_src = include_str!("../rs485.ui");
        let builder = Builder::from_string(ui_src);

        let window: ApplicationWindow = builder.object("window").expect(CGWINDOW);
        window.set_application(Some(application));

        Window { builder, window }
    }
}

#[derive(Clone)]
pub struct UI {
    relays: SpinButton,
    devices_list: ComboBoxText,
    on_off_switch: Switch,
    refresh_button: Button,
    scrolled_window: ScrolledWindow,
    text_view: TextView,
    text: TextBuffer,
    entry_command: Entry,
    crc_check_button: CheckButton,
    send_button: Button,
}

impl UI {
    pub fn new(builder: Builder) -> UI {
        let relays: SpinButton = builder.object("relays").expect(CGOBJECT);
        let devices_list: ComboBoxText = builder.object("devices").expect(CGOBJECT);
        let on_off_switch: Switch = builder.object("on_off_switch").expect(CGOBJECT);
        let refresh_button: Button = builder.object("refresh_button").expect(CGOBJECT);
        let scrolled_window: ScrolledWindow = builder.object("scrolled_window").expect(CGOBJECT);
        let text_view: TextView = builder.object("text_view").expect(CGOBJECT);
        let entry_command: Entry = builder.object("entry_command").expect(CGOBJECT);
        let crc_check_button: CheckButton = builder.object("crc_check_button").expect(CGOBJECT);
        let send_button: Button = builder.object("send_button").expect(CGOBJECT);
        let text = text_view.buffer();

        UI {
            relays,
            devices_list,
            on_off_switch,
            refresh_button,
            scrolled_window,
            text_view,
            text,
            entry_command,
            crc_check_button,
            send_button,
        }
    }

    pub fn build(&self) {
        let ports = RefCell::new(serialport::available_ports().unwrap());

        if ports.borrow().len() != 0 {
            for name in ports.borrow().iter() {
                self.devices_list.append_text(&*name.port_name);
            }
            match try_connect_to_device(&ports.borrow(), 0) {
                (b, s) => {
                    self.relays.set_sensitive(b);
                    self.on_off_switch.set_sensitive(b);
                    self.entry_command.set_sensitive(b);
                    self.crc_check_button.set_sensitive(b);
                    self.send_button.set_sensitive(b);
                    self.text.insert(
                        &mut self.text.end_iter(),
                        &*format!("{} {}", time_execute(), s),
                    );
                    self.devices_list.set_active(Some(0));
                }
            }
        } else {
            self.text.insert(&mut self.text.end_iter(), NFOUND);
        }

        self.build_devices_list(&ports);
        self.build_refresh_devices(&ports);
        let on_off_switch_signal_handler_id = self.build_on_off_switch(&ports);
        self.build_relays(&ports, on_off_switch_signal_handler_id);
        self.build_text_changed();
        self.build_send_button(&ports);
    }

    fn build_devices_list(&self, ports: &RefCell<Vec<SerialPortInfo>>) -> SignalHandlerId {
        self.devices_list.connect_active_notify(
            clone!(
                @strong self as ui, @strong ports => move |_| {
                    match try_connect_to_device(&ports.borrow(), ui.devices_list.active().unwrap() as u8) {
                        (b, s) => {
                            ui.relays.set_sensitive(b);
                            ui.on_off_switch.set_sensitive(b);
                            ui.entry_command.set_sensitive(b);
                            ui.crc_check_button.set_sensitive(b);
                            ui.send_button.set_sensitive(b);
                            ui.text.insert(
                                &mut ui.text.end_iter(),
                                &*format!("{} {}", time_execute(), s)
                            );
                        }
                    }
                }
            )
        )
    }

    fn build_refresh_devices<'a>(&self, ports: &RefCell<Vec<SerialPortInfo>>) -> SignalHandlerId {
        self.refresh_button
            .connect_clicked(clone!(@strong ports, @strong self as ui => move |_| {
                ui.devices_list.remove_all();
                for name in serialport::available_ports().unwrap() {
                    ui.devices_list.append_text(&*name.port_name);
                }

                let mut  ports = ports.borrow_mut();

                *ports = serialport::available_ports().unwrap();

                ui.devices_list.set_active(Some(0));
            }))
    }

    fn build_on_off_switch(&self, ports: &RefCell<Vec<SerialPortInfo>>) -> SignalHandlerId {
        self.on_off_switch.connect_active_notify(clone!(
            @strong self as ui, @strong ports => move |switch| {
                let mut port = set_port(
                    &ports.borrow(),
                    ui.devices_list.active().unwrap() as u8
                ).unwrap();
                let buf = match switch.is_active() {
                    true => {
                        control_command(ui.relays.value() as u8 - 1, 1)
                    },
                    false => {
                        control_command(ui.relays.value() as u8 - 1, 2)
                    }
                };
                rs485_write(&mut port, &buf);
                let data = rs485_read(&mut port);
                match data {
                    Ok(d) => {
                        ui.text.insert(
                            &mut ui.text.end_iter(),
                            &*format!(
                                "\n{} Sent: {}\n{} Received: {}\n",
                                time_execute(),
                                convert_to_hex_format(&buf),
                                time_execute(),
                                convert_to_hex_format(&d)
                            )
                        );
                    }
                    Err(e) => {
                        ui.text.insert(
                            &mut ui.text.end_iter(),
                            &*format!("{} Error reading data! : {:X?}\n", time_execute(), e.description)
                        )
                    }
                }
            }
        ))
    }

    fn build_relays(
        &self,
        ports: &RefCell<Vec<SerialPortInfo>>,
        on_off_switch_signal_handler_id: SignalHandlerId,
    ) -> SignalHandlerId {
        self.relays.connect_value_changed(clone!(
            @strong self as ui, @strong ports => move |relay| {
                let mut port = set_port(
                    &ports.borrow(), ui.devices_list.active().unwrap() as u8
                ).unwrap();

                let buf = read_status_command(relay.value() as u8 - 1);

                rs485_write(&mut port, &buf);

                match rs485_read(&mut port) {
                    Ok(data) => {
                        ui.text.insert(
                            &mut ui.text.end_iter(),
                            &*format!("\n{} Sent: {}\n{} Received: {}\n",
                                time_execute(),
                                convert_to_hex_format(&buf),
                                time_execute(),
                                convert_to_hex_format(&data)
                            )
                         );
                        ui.on_off_switch.block_signal(&on_off_switch_signal_handler_id);
                        match data {
                             o if o == RELAY_OPEN => ui.on_off_switch.set_state(true),
                             c if c == RELAY_CLOSE=> ui.on_off_switch.set_state(false),
                             _ => {
                                ui.text.insert(
                                    &mut ui.text.end_iter(),
                                    &*format!(
                                        "{} Valid data! : {:X?}\n",
                                        time_execute(),
                                        &data
                                    )
                                )
                            }
                        }
                        ui.on_off_switch.unblock_signal(&on_off_switch_signal_handler_id);
                    }
                    Err(e) => {
                        ui.text.insert(
                            &mut ui.text.end_iter(),
                            &*format!(
                                "{} Error reading data! : {:X?}\n",
                                time_execute(),
                                e.description
                            )
                        )
                    }
                };

            }
        ))
    }

    fn build_text_changed(&self) -> SignalHandlerId {
        self.text.connect_changed(clone!(@strong self as ui => move |_| {
            if ui.scrolled_window.vadjustment().upper() != ui.scrolled_window.vadjustment().page_size() {
                let t = ui.text.create_mark(Some("screen"), &ui.text.end_iter(), false);
                ui.text_view.scroll_to_mark(&t, 0., true, 0., 0.);
            }
        }))
    }

    fn build_send_button(&self, ports: &RefCell<Vec<SerialPortInfo>>) -> SignalHandlerId {
        self.send_button.connect_clicked(clone!(
            @strong self as ui, @strong ports=> move |_| {
                let mut port = set_port(
                    &ports.borrow(),
                    ui.devices_list.active().unwrap() as u8
                ).unwrap();

                let entry_text = ui.entry_command.text();

                match convert_text_to_hex(entry_text.to_string()) {
                    Ok((mut v, t)) => {
                        if ui.crc_check_button.is_active() {
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
                                ui.text.insert(
                                    &mut ui.text.end_iter(),
                                    &*format!(
                                        "\n{} Command sent: {}\n{} Received: {:?}\n",
                                        time_execute(),
                                        entry_text.to_string(),
                                        time_execute(),
                                        received
                                    )
                                );
                            }
                            Err(e) => {
                                ui.text.insert(
                                    &mut ui.text.end_iter(),
                                    &*format!(
                                        "{} Error reading data! : {:X?}\n",
                                        time_execute(),
                                        e.description
                                    )
                                )
                            }
                        }
                    }
                    Err(e) => {
                        ui.text.insert(
                            &mut ui.text.end_iter(),
                            &*format!("{} {}\n", time_execute(), e)
                        );
                    }
                }
            }
        ))
    }
}
