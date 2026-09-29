pub mod surah_data;
pub mod ayah_view;

use adw::prelude::*;
use gtk::{Box as GtkBox, DropDown, Label, Orientation, Spinner};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use crate::config::AppConfig;
use crate::fonts::apply_arabic_font;
use self::ayah_view::AyahViewport;
use self::surah_data::{fetch_all_surahs, fetch_surah_ayahs, Ayah, create_shared_client};

struct CachedSurah {
    ayahs: Vec<Rc<Ayah>>,
}

pub struct QuranPage {
    widget: GtkBox,
}

impl QuranPage {
    pub fn new(_config: AppConfig) -> Self {
        let widget = GtkBox::new(Orientation::Vertical, 0);
        widget.set_vexpand(true);

        let dropdown_box = GtkBox::new(Orientation::Horizontal, 8);
        dropdown_box.set_margin_top(12);
        dropdown_box.set_margin_bottom(12);
        dropdown_box.set_margin_start(12);
        dropdown_box.set_margin_end(12);

        let dropdown_label = Label::new(Some("Surah:"));
        dropdown_box.append(&dropdown_label);

        let dropdown = DropDown::from_strings(&["Loading surahs..."]);
        dropdown.set_hexpand(true);
        dropdown.set_sensitive(false);
        dropdown_box.append(&dropdown);
        widget.append(&dropdown_box);

        let header_box = GtkBox::new(Orientation::Vertical, 4);
        header_box.set_margin_top(12);
        header_box.set_margin_bottom(12);
        header_box.set_margin_start(12);
        header_box.set_margin_end(12);

        let arabic_heading = Label::new(None);
        arabic_heading.add_css_class("surah-heading");
        arabic_heading.set_justify(gtk::Justification::Center);
        arabic_heading.set_halign(gtk::Align::Center);
        apply_arabic_font(&arabic_heading, 36);
        header_box.append(&arabic_heading);

        let english_heading = Label::new(None);
        english_heading.add_css_class("title-3");
        english_heading.add_css_class("dim-label");
        english_heading.set_justify(gtk::Justification::Center);
        english_heading.set_halign(gtk::Align::Center);
        header_box.append(&english_heading);
        widget.append(&header_box);

        let stack = gtk::Stack::new();
        stack.set_vexpand(true);

        let status = GtkBox::new(Orientation::Vertical, 12);
        status.set_valign(gtk::Align::Center);
        status.set_halign(gtk::Align::Center);
        status.set_vexpand(true);
        let spinner = Spinner::new();
        spinner.start();
        let status_label = Label::new(Some("Loading surahs..."));
        status_label.add_css_class("dim-label");
        status_label.set_wrap(true);
        status_label.set_margin_start(24);
        status_label.set_margin_end(24);
        status.append(&spinner);
        status.append(&status_label);

        let viewport = AyahViewport::new();
        stack.add_named(&status, Some("status"));
        stack.add_named(viewport.widget(), Some("ayahs"));
        stack.set_visible_child_name("status");
        widget.append(&stack);

        let client = match create_shared_client() {
            Ok(client) => client,
            Err(error) => {
                spinner.stop();
                status_label.set_label(&format!("Error creating HTTP client: {error}"));
                return Self { widget };
            }
        };

        let surah_cache: Rc<RefCell<HashMap<u32, CachedSurah>>> = Rc::new(RefCell::new(HashMap::new()));
        let request_id = Rc::new(Cell::new(0u64));
        let inflight = Rc::new(Cell::new(None::<u32>));
        let names: Rc<RefCell<Vec<surah_data::SurahMeta>>> = Rc::new(RefCell::new(Vec::new()));

        let dropdown_w = dropdown.downgrade();
        let spinner_w = spinner.downgrade();
        let status_w = status_label.downgrade();
        let stack_w = stack.downgrade();
        let arabic_w = arabic_heading.downgrade();
        let english_w = english_heading.downgrade();
        let viewport_ayahs = Rc::new(viewport);

        let show_status = {
            let spinner_w = spinner_w.clone();
            let status_w = status_w.clone();
            let stack_w = stack_w.clone();
            move |text: &str, spinning: bool| {
                if let Some(label) = status_w.upgrade() {
                    label.set_label(text);
                }
                if let Some(spinner) = spinner_w.upgrade() {
                    if spinning {
                        spinner.set_visible(true);
                        spinner.start();
                    } else {
                        spinner.stop();
                        spinner.set_visible(false);
                    }
                }
                if let Some(stack) = stack_w.upgrade() {
                    stack.set_visible_child_name("status");
                }
            }
        };

        let client_for_init = client.clone();
        let dropdown_init = dropdown_w.clone();
        let show_status_init = show_status.clone();
        let names_init = names.clone();
        let cache_init = surah_cache.clone();
        let inflight_init = inflight.clone();
        let request_init = request_id.clone();
        let arabic_init = arabic_w.clone();
        let english_init = english_w.clone();
        let viewport_init = viewport_ayahs.clone();
        let client_init = client.clone();
        let stack_init = stack_w.clone();
        let spinner_init = spinner_w.clone();

        crate::finish_on_main(
            async move { fetch_all_surahs(&client_for_init).await },
            move |result| match result {
                Ok(surahs) => {
                    if let Some(dropdown) = dropdown_init.upgrade() {
                        let items: Vec<String> = surahs
                            .iter()
                            .map(|surah| {
                                format!("{}. {} ({})", surah.number, surah.english_name, surah.name)
                            })
                            .collect();
                        let refs: Vec<&str> = items.iter().map(String::as_str).collect();
                        dropdown.set_model(Some(&gtk::StringList::new(&refs)));
                        dropdown.set_sensitive(true);
                        *names_init.borrow_mut() = surahs;
                        begin_surah_load(
                            1,
                            &names_init,
                            &cache_init,
                            &inflight_init,
                            &request_init,
                            &arabic_init,
                            &english_init,
                            &viewport_init,
                            &stack_init,
                            &spinner_init,
                            &show_status_init,
                            &client_init,
                        );
                    }
                }
                Err(error) => show_status_init(&format!("Could not load the surah list. {error}"), false),
            },
        );

        let names_h = names.clone();
        let cache_h = surah_cache.clone();
        let inflight_h = inflight.clone();
        let request_h = request_id.clone();
        let arabic_h = arabic_w.clone();
        let english_h = english_w.clone();
        let viewport_h = viewport_ayahs.clone();
        let stack_h = stack_w.clone();
        let spinner_h = spinner_w.clone();
        let client_h = client;
        let show_status_h = show_status;

        dropdown.connect_selected_notify(move |dropdown| {
            if !dropdown.is_sensitive() {
                return;
            }
            let idx = dropdown.selected();
            if idx == gtk::INVALID_LIST_POSITION {
                return;
            }
            begin_surah_load(
                idx + 1,
                &names_h,
                &cache_h,
                &inflight_h,
                &request_h,
                &arabic_h,
                &english_h,
                &viewport_h,
                &stack_h,
                &spinner_h,
                &show_status_h,
                &client_h,
            );
        });

        Self { widget }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.widget
    }
}

fn begin_surah_load(
    number: u32,
    names: &Rc<RefCell<Vec<surah_data::SurahMeta>>>,
    cache: &Rc<RefCell<HashMap<u32, CachedSurah>>>,
    inflight: &Rc<Cell<Option<u32>>>,
    request_id: &Rc<Cell<u64>>,
    arabic: &glib::WeakRef<Label>,
    english: &glib::WeakRef<Label>,
    viewport: &Rc<AyahViewport>,
    stack: &glib::WeakRef<gtk::Stack>,
    spinner: &glib::WeakRef<Spinner>,
    show_status: &impl Fn(&str, bool),
    client: &reqwest::Client,
) {
    if let Some(meta) = names.borrow().iter().find(|meta| meta.number == number) {
        if let Some(label) = arabic.upgrade() {
            label.set_label(&meta.name);
        }
        if let Some(label) = english.upgrade() {
            label.set_label(&format!(
                "{}, {}",
                meta.english_name, meta.english_name_translation
            ));
        }
    }

    if let Some(cached) = cache.borrow().get(&number) {
        inflight.set(None);
        viewport.set_ayahs(cached.ayahs.clone());
        if let Some(stack) = stack.upgrade() {
            stack.set_visible_child_name("ayahs");
        }
        if let Some(spinner) = spinner.upgrade() {
            spinner.stop();
        }
        return;
    }

    if inflight.get() == Some(number) {
        return;
    }
    inflight.set(Some(number));
    let current_request = request_id.get() + 1;
    request_id.set(current_request);

    let title = names
        .borrow()
        .iter()
        .find(|meta| meta.number == number)
        .map(|meta| meta.english_name.clone())
        .unwrap_or_else(|| "surah".to_string());
    show_status(&format!("Loading {title}..."), true);
    viewport.set_ayahs(Vec::new());

    let cache = cache.clone();
    let inflight = inflight.clone();
    let request_id = request_id.clone();
    let arabic = arabic.clone();
    let english = english.clone();
    let viewport = viewport.clone();
    let stack = stack.clone();
    let spinner = spinner.clone();
    let client = client.clone();

    crate::finish_on_main(
        async move { fetch_surah_ayahs(&client, number).await },
        move |result| {
            if request_id.get() != current_request {
                return;
            }
            inflight.set(None);
            if let Some(spinner) = spinner.upgrade() {
                spinner.stop();
            }
            match result {
                Ok((meta, ayahs)) => {
                    if let Some(label) = arabic.upgrade() {
                        label.set_label(&meta.name);
                    }
                    if let Some(label) = english.upgrade() {
                        label.set_label(&format!(
                            "{}, {}",
                            meta.english_name, meta.english_name_translation
                        ));
                    }
                    let ayahs: Vec<Rc<Ayah>> = ayahs.into_iter().map(Rc::new).collect();
                    cache.borrow_mut().insert(
                        number,
                        CachedSurah {
                            ayahs: ayahs.clone(),
                        },
                    );
                    viewport.set_ayahs(ayahs);
                    if let Some(stack) = stack.upgrade() {
                        stack.set_visible_child_name("ayahs");
                    }
                }
                Err(_) => {
                    if let Some(stack) = stack.upgrade() {
                        stack.set_visible_child_name("status");
                    }
                    if let Some(label) = arabic.upgrade() {
                        label.set_label("Could not load this surah.");
                    }
                }
            }
        },
    );
}
