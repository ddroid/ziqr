use adw::prelude::*;
use adw::{Application, StyleManager};

use crate::config::AppConfig;
use crate::main_window::MainWindow;
use crate::welcome::WelcomeWindow;

pub struct EemanApp {
    app: Application,
    config: AppConfig,
}

impl EemanApp {
    pub fn new() -> Self {
        let app = Application::builder()
            .application_id("pro.ddroid.Ziqr")
            .build();

        let config = AppConfig::new();

        Self { app, config }
    }

    pub fn run(&self) {
        let config = self.config.clone();
        let app = self.app.clone();

        app.connect_activate(move |app| {
            if let Some(window) = app.windows().into_iter().next() {
                window.present();
                return;
            }

            // Register fonts AFTER GTK init but BEFORE widgets are created
            crate::fonts::register_fonts();

            // Apply theme
            let style_manager = StyleManager::default();
            if let Some(theme) = config.get("Appearance", "theme") {
                match theme.as_str() {
                    "Dark" => style_manager.set_color_scheme(adw::ColorScheme::ForceDark),
                    "Light" => style_manager.set_color_scheme(adw::ColorScheme::ForceLight),
                    _ => style_manager.set_color_scheme(adw::ColorScheme::Default),
                }
            }

            // Load CSS for Arabic font styling (embedded at compile time)
            let css_provider = gtk::CssProvider::new();
            css_provider.load_from_data(include_str!("../resources/style.css"));
            gtk::style_context_add_provider_for_display(
                &gtk::gdk::Display::default().expect("Could not get display"),
                &css_provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );

            // Check first run
            let first_run = config
                .get("App", "first_run")
                .unwrap_or_else(|| "Yes".to_string());

            if first_run == "Yes" {
                let win = WelcomeWindow::new(app, config.clone());
                win.present();
            } else {
                let win = MainWindow::new(app, config.clone());
                win.present();
            }
        });

        // Actions
        let quit_action = gtk::gio::SimpleAction::new("quit", None);
        quit_action.connect_activate(|_, _| {
            Application::default().quit();
        });
        self.app.add_action(&quit_action);
        self.app
            .set_accels_for_action("app.quit", &["<primary>q"]);

        self.app.run_with_args::<String>(&[]);
    }
}
