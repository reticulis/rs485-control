use crate::app::values::APPLICATION_ID;
use crate::app::window::App;
use adw::prelude::{ApplicationExt, ApplicationExtManual};

mod app;

fn main() {
    let application = adw::Application::new(Some(APPLICATION_ID), Default::default());

    application.connect_startup(|_| {
        adw::init();
    });

    application.connect_activate(App::run);
    application.run();
}