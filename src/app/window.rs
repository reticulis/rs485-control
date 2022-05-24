use super::device::{
    checksum, control_command, read_status_command, rs485_read, rs485_write, set_port,
};
use super::text_formatting::{
    convert_hex_to_ascii, convert_text_to_hex, convert_to_hex_format, time_execute,
};

use glib_macros::clone;
use std::cell::RefCell;
use std::error::Error;

use crate::app::values::{NotFoundDevices, TypeData, CGOBJECT, CGWINDOW, RELAY_CLOSE, RELAY_OPEN};
use adw::prelude::*;
use adw::{Application, ApplicationWindow};
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
        let text_view: TextView = builder.object("text_view").expect(CGOBJECT);
        let text = text_view.buffer();

        UI {
            relays: builder.object("relays").expect(CGOBJECT),
            devices_list: builder.object("devices").expect(CGOBJECT),
            on_off_switch: builder.object("on_off_switch").expect(CGOBJECT),
            refresh_button: builder.object("refresh_button").expect(CGOBJECT),
            scrolled_window: builder.object("scrolled_window").expect(CGOBJECT),
            text_view,
            text,
            entry_command: builder.object("entry_command").expect(CGOBJECT),
            crc_check_button: builder.object("crc_check_button").expect(CGOBJECT),
            send_button: builder.object("send_button").expect(CGOBJECT),
        }
    }

    pub fn build(&self) {
        let ports = RefCell::new(serialport::available_ports().unwrap());

        if ports.borrow().len() != 0 {
            for name in ports.borrow().iter() {
                self.devices_list.append_text(&*name.port_name);
            }
            self.devices_list.set_active(Some(0));
            let result = match self.check_devices_list(&ports) {
                Ok(_) => "Connected!\n".to_owned(),
                Err(e) => format!("Failed connecting! {}\n", &*e.to_string()),
            };
            self.text.insert(
                &mut self.text.end_iter(),
                &*format!("{} {}", time_execute(), result),
            );
        } else {
            self.text
                .insert(&mut self.text.end_iter(), "Not found devices!\n")
        }

        self.devices_list.connect_active_notify(
            clone!(@strong self as ui, @strong ports => move |_| {
                let result = match ui.check_devices_list(&ports) {
                    Ok(_) => "Connected!\n".to_owned(),
                    Err(e) => format!("Failed connecting! {}\n", &*e.to_string())
                };
                ui.text.insert(
                    &mut ui.text.end_iter(),
                    &*format!("{} {}", time_execute(), result)
                )
            }),
        );

        self.refresh_button
            .connect_clicked(clone!(@strong self as ui, @strong ports => move |_| {
                if let Err(e) = ui.build_refresh_devices(&ports) {
                    ui.text.insert(
                        &mut ui.text.end_iter(),
                        &*format!("{} Error: {}\n", time_execute(), &*e.to_string())
                    )
                };
            }));

        let on_off_switch_signal_handler_id = self.on_off_switch.connect_active_notify(
            clone!(@strong self as ui, @strong ports => move |_| {
                let buf = if ui.on_off_switch.is_active() {
                    control_command(ui.relays.value() as u8 - 1, 1)
                } else {
                    control_command(ui.relays.value() as u8 - 1, 2)
                };

                match ui.build_on_off_switch(&ports, &buf) {
                    Ok(s) => {
                        ui.text.insert(
                            &mut ui.text.end_iter(),
                            &*format!(
                                "{} Sent: {}\n{} Received: {}\n",
                                time_execute(),
                                convert_to_hex_format(&buf),
                                time_execute(),
                                s
                            )
                        )
                    }
                    Err(e) => {
                        ui.text.insert(
                            &mut ui.text.end_iter(),
                            &*format!(
                                "{} Error reading data!: {}\n",
                                time_execute(),
                                &*e.to_string()
                            )
                        )
                    }
                }
            }),
        );

        self.relays
            .connect_value_changed(clone!(@strong self as ui, @strong ports => move |_| {
                let buf = read_status_command(ui.relays.value() as u8 - 1);
                match ui.build_relays(&ports, &buf) {
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

                        if data != RELAY_OPEN || data != RELAY_CLOSE {
                            ui.text.insert(
                                &mut ui.text.end_iter(),
                                &*format!("\n{} Valid data! {:?}\n",
                                    time_execute(), &data
                                )
                            );
                        }

                        ui.on_off_switch.unblock_signal(&on_off_switch_signal_handler_id);
                    }
                    Err(e) => {
                        ui.text.insert(
                            &mut ui.text.end_iter(),
                            &*format!(
                                "{} Error reading data! : {}\n",
                                time_execute(),
                                &*e.to_string()
                            )
                        )
                    }
                }
            }));

        self.text.connect_changed(clone!(@strong self as ui => move |_| {
            if ui.scrolled_window.vadjustment().upper() != ui.scrolled_window.vadjustment().page_size() {
                let t = ui.text.create_mark(Some("screen"), &ui.text.end_iter(), false);
                ui.text_view.scroll_to_mark(&t, 0., true, 0., 0.);
            }
        }));

        self.send_button
            .connect_clicked(clone!(@strong self as ui, @strong ports => move |_| {
                match ui.build_send_button(&ports) {
                    Ok(s) => ui.text.insert(
                        &mut ui.text.end_iter(),
                        &*s
                    ),
                    Err(e) => ui.text.insert(
                        &mut ui.text.end_iter(),
                        &*format!("{} Error sending data: {}\n", time_execute(), &*e.to_string())
                    )
                }
            }));
    }

    fn check_devices_list(
        &self,
        ports: &RefCell<Vec<SerialPortInfo>>,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(device_list_id) = self.devices_list.active() {
            if let Err(e) = set_port(&ports.borrow(), device_list_id as u8) {
                self.set_all_sensitive(false);
                return Err(e);
            }
            self.set_all_sensitive(true);
            return Ok(());
        }
        Err(Box::new(NotFoundDevices))
    }

    fn build_refresh_devices(
        &self,
        ports: &RefCell<Vec<SerialPortInfo>>,
    ) -> Result<(), Box<dyn Error>> {
        self.devices_list.remove_all();
        for name in serialport::available_ports()? {
            self.devices_list.append_text(&*name.port_name);
        }

        let mut ports = ports.borrow_mut();

        *ports = serialport::available_ports()?;

        self.devices_list.set_active(Some(0));
        Ok(())
    }

    fn build_on_off_switch(
        &self,
        ports: &RefCell<Vec<SerialPortInfo>>,
        buf: &[u8],
    ) -> Result<String, Box<dyn Error>> {
        let mut port = set_port(&ports.borrow(), self.devices_list.active().unwrap() as u8)?;

        rs485_write(&mut port, buf)?;
        match rs485_read(&mut port) {
            Ok(d) => Ok(convert_to_hex_format(&d)),
            Err(e) => Err(e),
        }
    }

    fn build_relays(
        &self,
        ports: &RefCell<Vec<SerialPortInfo>>,
        buf: &[u8],
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut port = set_port(&ports.borrow(), self.devices_list.active().unwrap() as u8)?;
        rs485_write(&mut port, buf)?;
        rs485_read(&mut port)
    }

    fn build_send_button(
        &self,
        ports: &RefCell<Vec<SerialPortInfo>>,
    ) -> Result<String, Box<dyn Error>> {
        let mut port =
            set_port(&ports.borrow(), self.devices_list.active().unwrap() as u8).unwrap();

        let entry_text = self.entry_command.text();

        // OMG SORRY
        return match convert_text_to_hex(entry_text.to_string()) {
            Ok(t) => {
                let mut vec = match &t {
                    TypeData::ASCII(v) => v.clone(),
                    TypeData::MODBUS(v) => v.clone(),
                };
                if self.crc_check_button.is_active() {
                    checksum(&mut vec);
                }
                rs485_write(&mut port, &vec)?;
                let data = rs485_read(&mut port);
                match data {
                    Ok(d) => {
                        let received = match &t {
                            TypeData::ASCII(_) => convert_hex_to_ascii(&d),
                            TypeData::MODBUS(_) => convert_to_hex_format(&d),
                        };
                        Ok(format!(
                            "\n{} Command sent: {}\n{} Received: {:?}\n",
                            time_execute(),
                            entry_text,
                            time_execute(),
                            received
                        ))
                    }
                    Err(e) => Err(e),
                }
            }
            Err(e) => Err(e),
        };
    }

    fn set_all_sensitive(&self, b: bool) {
        self.relays.set_sensitive(b);
        self.on_off_switch.set_sensitive(b);
        self.entry_command.set_sensitive(b);
        self.crc_check_button.set_sensitive(b);
        self.send_button.set_sensitive(b);
    }
}
