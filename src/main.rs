use adw::prelude::{ApplicationExt, ApplicationExtManual};
mod app;

fn main() {
    let application = adw::Application::new(
        Some("com.github.reticulis.rs485-control"),
        Default::default(),
    );

    application.connect_startup(|_| {
        adw::init();
    });

    application.connect_activate(app::build_ui::build_ui);
    application.run();
}