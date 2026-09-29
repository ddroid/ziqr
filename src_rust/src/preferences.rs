use adw::prelude::*;
use crate::config::AppConfig;

const METHODS: &[(&str, &str)] = &[
    ("University of Islamic Sciences, Karachi", "1"),
    ("Islamic Society of North America", "2"),
    ("Muslim World League", "3"),
    ("Umm Al-Qura University, Makkah", "4"),
    ("Egyptian General Authority of Survey", "5"),
    ("Institute of Geophysics, University of Tehran", "7"),
    ("Gulf Region", "8"),
    ("Kuwait", "9"),
    ("Qatar", "10"),
    ("Majlis Ugama Islam Singapura, Singapore", "11"),
    ("Union Organization islamic de France", "12"),
    ("Diyanet İşleri Başkanlığı, Turkey", "13"),
    ("Spiritual Administration of Muslims of Russia", "14"),
    ("Moonsighting Committee Worldwide", "15"),
    ("Dubai (unofficial)", "16"),
];

pub struct PrefsWindow {
    window: adw::PreferencesWindow,
}

impl PrefsWindow {
    pub fn new(parent: &adw::ApplicationWindow, config: AppConfig) -> Self {
        let window = adw::PreferencesWindow::builder()
            .transient_for(parent)
            .title("Preferences")
            .build();
        window.add(&preferences_page(&config));
        Self { window }
    }

    pub fn present(&self) {
        self.window.present();
    }
}

/// Setup and appearance page shared by the welcome carousel and the menu.
pub fn preferences_page(config: &AppConfig) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::new();

    let setup_group = adw::PreferencesGroup::builder().title("Setup").build();

    let location_row = adw::ComboRow::builder().title("Location Mode").build();
    location_row.set_model(Some(&gtk::StringList::new(&[
        "Automatic using IP",
        "Manual (City, Country)",
    ])));

    let school_row = adw::ComboRow::builder()
        .title("School of thought")
        .build();
    school_row.set_model(Some(&gtk::StringList::new(&[
        "Standard (Shafi'i, Maliki and Hanbali)",
        "Hanafi",
    ])));

    let method_row = adw::ComboRow::builder()
        .title("Calculation Method")
        .build();
    method_row.set_model(Some(&gtk::StringList::new(&[
        "Automatic (Nearest)",
        "Manual (Choose Method)",
    ])));

    let country_row = adw::EntryRow::builder().title("Country").build();
    country_row.set_show_apply_button(true);
    let city_row = adw::EntryRow::builder().title("City").build();
    city_row.set_show_apply_button(true);

    let calc_row = adw::ComboRow::builder()
        .title("Calculation Method")
        .build();
    let method_names: Vec<&str> = METHODS.iter().map(|(name, _)| *name).collect();
    calc_row.set_model(Some(&gtk::StringList::new(&method_names)));

    let loc_mode = config.get("Prayer", "location_mode").unwrap_or_default();
    location_row.set_selected(if loc_mode == "Manual" { 1 } else { 0 });
    let manual_location = location_row.selected() == 1;
    city_row.set_sensitive(manual_location);
    country_row.set_sensitive(manual_location);

    let school = config.get("Prayer", "hanafi_school").unwrap_or_default();
    school_row.set_selected(if school == "1" { 1 } else { 0 });

    let meth_mode = config.get("Prayer", "method_mode").unwrap_or_default();
    method_row.set_selected(if meth_mode == "Manual" { 1 } else { 0 });
    calc_row.set_sensitive(method_row.selected() == 1);

    let method = config.get("Prayer", "method").unwrap_or_else(|| "2".into());
    let method_index = METHODS
        .iter()
        .position(|(_, id)| *id == method)
        .unwrap_or(1) as u32;
    calc_row.set_selected(method_index);

    let city = config.get("Prayer", "city").unwrap_or_default();
    if !city.is_empty() && city != "None" {
        city_row.set_text(&city);
    }
    let country = config.get("Prayer", "country").unwrap_or_default();
    if !country.is_empty() && country != "None" {
        country_row.set_text(&country);
    }

    let config_loc = config.clone();
    let city_for_loc = city_row.clone();
    let country_for_loc = country_row.clone();
    location_row.connect_selected_notify(move |row| {
        let manual = row.selected() == 1;
        city_for_loc.set_sensitive(manual);
        country_for_loc.set_sensitive(manual);
        config_loc.set(
            "Prayer",
            "location_mode",
            if manual { "Manual" } else { "Automatic" },
        );
    });

    let config_school = config.clone();
    school_row.connect_selected_notify(move |row| {
        config_school.set("Prayer", "hanafi_school", &row.selected().to_string());
    });

    let config_method = config.clone();
    let calc_for_method = calc_row.clone();
    method_row.connect_selected_notify(move |row| {
        let manual = row.selected() == 1;
        calc_for_method.set_sensitive(manual);
        config_method.set(
            "Prayer",
            "method_mode",
            if manual { "Manual" } else { "Automatic" },
        );
    });

    let config_city = config.clone();
    city_row.connect_apply(move |entry| {
        config_city.set("Prayer", "city", entry.text().as_str());
    });
    let config_country = config.clone();
    country_row.connect_apply(move |entry| {
        config_country.set("Prayer", "country", entry.text().as_str());
    });

    let config_calc = config.clone();
    calc_row.connect_selected_notify(move |row| {
        let selected = row.selected() as usize;
        if let Some((_, id)) = METHODS.get(selected) {
            config_calc.set("Prayer", "method", id);
        }
    });

    setup_group.add(&location_row);
    setup_group.add(&school_row);
    setup_group.add(&method_row);
    setup_group.add(&country_row);
    setup_group.add(&city_row);
    setup_group.add(&calc_row);
    page.add(&setup_group);

    let appearance = adw::PreferencesGroup::builder()
        .title("Appearance")
        .build();
    let theme_row = adw::ActionRow::builder().title("Dark theme").build();
    let theme_switch = gtk::Switch::builder()
        .valign(gtk::Align::Center)
        .build();
    let dark = config.get("Appearance", "theme").as_deref() == Some("Dark");
    theme_switch.set_active(dark);
    theme_row.add_suffix(&theme_switch);
    theme_row.set_activatable_widget(Some(&theme_switch));

    let config_theme = config.clone();
    theme_switch.connect_state_set(move |_, state| {
        config_theme.set(
            "Appearance",
            "theme",
            if state { "Dark" } else { "Light" },
        );
        let style = adw::StyleManager::default();
        style.set_color_scheme(if state {
            adw::ColorScheme::ForceDark
        } else {
            adw::ColorScheme::ForceLight
        });
        glib::Propagation::Proceed
    });
    appearance.add(&theme_row);
    page.add(&appearance);

    page
}
