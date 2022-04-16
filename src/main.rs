use adw::prelude::{ApplicationExt, ApplicationExtManual};
use crate::app::values::APPLICATION_ID;

mod app;

fn main() {
    let application = adw::Application::new(
        Some(APPLICATION_ID),
        Default::default(),
    );

    application.connect_startup(|_| {
        adw::init();
    });

    application.connect_activate(app::build_ui::build_ui);
    application.run();
}