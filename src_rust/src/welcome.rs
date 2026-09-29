use adw::prelude::*;
use adw::{Application, ApplicationWindow, Carousel, CarouselIndicatorDots, StatusPage, Clamp};
use gtk::{Box as GtkBox, Button, Orientation};
use crate::config::AppConfig;
use crate::main_window::MainWindow;
use crate::preferences::preferences_page;

pub struct WelcomeWindow {
    window: ApplicationWindow,
}

impl WelcomeWindow {
    pub fn new(app: &Application, config: AppConfig) -> Self {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("As-salamu alaykum - Ziqr")
            .default_width(400)
            .default_height(600)
            .build();

        let main_box = GtkBox::new(Orientation::Vertical, 0);
        main_box.set_vexpand(true);
        window.set_content(Some(&main_box));

        // Carousel for welcome pages
        let carousel = Carousel::new();
        carousel.set_hexpand(true);
        carousel.set_vexpand(true);
        carousel.set_allow_scroll_wheel(true);
        carousel.set_allow_long_swipes(false);
        main_box.append(&carousel);

        // Indicator dots
        let dots = CarouselIndicatorDots::new();
        dots.set_carousel(Some(&carousel));
        main_box.append(&dots);

        // Page 1 - Welcome
        let page1 = StatusPage::builder()
            .title("As-salamu alaykum !")
            .description("we will run through the setup process now...")
            .icon_name("pro.ddroid.Ziqr")
            .hexpand(true)
            .vexpand(true)
            .build();
        carousel.append(&page1);

        // Page 2 - Setup/Preferences
        let page2_box = GtkBox::new(Orientation::Vertical, 0);
        page2_box.set_hexpand(true);
        page2_box.set_vexpand(true);
        page2_box.set_halign(gtk::Align::Center);
        page2_box.set_valign(gtk::Align::Center);

        let clamp = Clamp::new();
        let prefs = preferences_page(&config);
        clamp.set_child(Some(&prefs));
        page2_box.append(&clamp);
        carousel.append(&page2_box);

        // Page 3 - Done
        let page3 = StatusPage::builder()
            .title("All set")
            .description("Ziqr app is now all setup, you can change settings later at any time.")
            .icon_name("pro.ddroid.Ziqr")
            .hexpand(true)
            .vexpand(true)
            .build();

        let done_box = GtkBox::new(Orientation::Vertical, 10);
        done_box.set_halign(gtk::Align::Center);

        let done_btn = Button::with_label("Next");
        done_btn.add_css_class("suggested-action");
        done_box.append(&done_btn);

        let back_btn = Button::with_label("Go back");
        done_box.append(&back_btn);

        page3.set_child(Some(&done_box));
        carousel.append(&page3);

        // Button handlers
        let carousel_clone = carousel.clone();
        back_btn.connect_clicked(move |_| {
            carousel_clone.scroll_to(&page2_box, true);
        });

        let config_clone = config.clone();
        let window_clone = window.clone();
        done_btn.connect_clicked(move |_| {
            config_clone.set("App", "first_run", "No");
            
            if let Some(app) = window_clone.application() {
                let adw_app = app.downcast::<adw::Application>().ok();
                if let Some(adw_app) = adw_app {
                    let main_win = MainWindow::new(&adw_app, config_clone.clone());
                    main_win.present();
                }
            }
            window_clone.close();
        });

        Self { window }
    }

    pub fn present(&self) {
        self.window.present();
    }
}
