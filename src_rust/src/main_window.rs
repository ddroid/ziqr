use adw::prelude::*;
use adw::{Application, ApplicationWindow};
use crate::config::AppConfig;
use crate::preferences::PrefsWindow;
use crate::prayer::PrayerPage;
use crate::quran::QuranPage;

pub struct MainWindow {
    window: ApplicationWindow,
}

impl MainWindow {
    pub fn new(app: &Application, config: AppConfig) -> Self {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Ziqr")
            .default_width(500)
            .default_height(600)
            .hide_on_close(true)
            .build();

        // Main layout
        let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        window.set_content(Some(&main_box));

        // Header bar
        let header = adw::HeaderBar::new();
        main_box.append(&header);

        // Menu button
        let menu_btn = gtk::MenuButton::new();
        menu_btn.set_icon_name("open-menu-symbolic");

        let menu = gio::Menu::new();
        menu.append(Some("Preferences"), Some("win.preferences"));
        menu.append(Some("About"), Some("win.about"));
        menu.append(Some("Quit"), Some("app.quit"));

        let popover = gtk::PopoverMenu::from_model(Some(&menu));
        menu_btn.set_popover(Some(&popover));
        header.pack_end(&menu_btn);

        // Actions
        let about_action = gtk::gio::SimpleAction::new("about", None);
        about_action.connect_activate(|_, _| {
            let about = adw::AboutWindow::builder()
                .application_name("Ziqr")
                .application_icon("pro.ddroid.Ziqr")
                .developer_name("ddroid")
                .version("0.1.2")
                .license_type(gtk::License::Gpl30)
                .build();
            about.present();
        });
        window.add_action(&about_action);

        let prefs_action = gtk::gio::SimpleAction::new("preferences", None);
        let config_clone = config.clone();
        let window_clone = window.clone();
        prefs_action.connect_activate(move |_, _| {
            let prefs = PrefsWindow::new(&window_clone, config_clone.clone());
            prefs.present();
        });
        window.add_action(&prefs_action);

        let stack = adw::ViewStack::new();
        stack.set_vexpand(true);

        let prayer_page = PrayerPage::new(config.clone(), app.clone());
        stack.add_titled_with_icon(prayer_page.widget(), Some("prayer"), "Prayer", "alarm-symbolic");

        let quran_page = QuranPage::new(config.clone());
        stack.add_titled_with_icon(
            quran_page.widget(),
            Some("quran"),
            "Quran",
            "accessories-dictionary-symbolic",
        );

        let switcher = adw::ViewSwitcher::builder()
            .policy(adw::ViewSwitcherPolicy::Wide)
            .stack(&stack)
            .build();
        header.set_title_widget(Some(&switcher));

        main_box.append(&stack);

        let switcher_bar = adw::ViewSwitcherBar::builder()
            .stack(&stack)
            .reveal(false)
            .build();
        main_box.append(&switcher_bar);

        let narrow = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            550.0,
            adw::LengthUnit::Sp,
        ));
        narrow.add_setter(&switcher, "visible", Some(&false.to_value()));
        narrow.add_setter(&switcher_bar, "reveal", Some(&true.to_value()));
        window.add_breakpoint(narrow);

        Self { window }
    }

    pub fn present(&self) {
        self.window.present();
    }
}
