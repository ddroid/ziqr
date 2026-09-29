use adw::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::config::AppConfig;

const PRAYERS: [&str; 6] = ["Fajr", "Sunrise", "Dhuhr", "Asr", "Maghrib", "Isha"];
const BELL_ON: &str = "alarm-symbolic";
const BELL_OFF: &str = "notifications-disabled-symbolic";

pub struct PrayerPage {
    widget: gtk::Box,
}

struct Slot {
    name: &'static str,
    hhmm: String,
    button: gtk::Button,
    time_label: gtk::Label,
}

impl PrayerPage {
    pub fn new(config: AppConfig, app: adw::Application) -> Self {
        let widget = gtk::Box::new(gtk::Orientation::Vertical, 0);
        widget.set_vexpand(true);

        let clamp = adw::Clamp::new();
        clamp.set_maximum_size(600);
        widget.append(&clamp);

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 12);
        vbox.set_margin_top(20);
        vbox.set_margin_bottom(20);
        vbox.set_margin_start(20);
        vbox.set_margin_end(20);
        clamp.set_child(Some(&vbox));

        let status = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        status.set_halign(gtk::Align::Center);
        let spinner = gtk::Spinner::new();
        spinner.start();
        let date_label = gtk::Label::new(Some("Loading prayer times..."));
        date_label.add_css_class("title-2");
        status.append(&spinner);
        status.append(&date_label);
        vbox.append(&status);

        let hijri_label = gtk::Label::new(None);
        hijri_label.add_css_class("dim-label");
        vbox.append(&hijri_label);

        let timezone_label = gtk::Label::new(None);
        timezone_label.add_css_class("caption");
        timezone_label.set_margin_top(8);
        vbox.append(&timezone_label);

        let group = gtk::Box::new(gtk::Orientation::Vertical, 4);
        group.set_margin_top(20);
        vbox.append(&group);

        let slots: Rc<RefCell<Vec<Slot>>> = Rc::new(RefCell::new(Vec::new()));
        for name in PRAYERS {
            let on = config
                .get("Prayer", &format!("{name}_notify"))
                .as_deref()
                == Some("Yes");
            let button = gtk::Button::from_icon_name(if on { BELL_ON } else { BELL_OFF });
            button.set_has_frame(false);
            button.set_tooltip_text(Some(if on {
                "Notifications on"
            } else {
                "Notifications off"
            }));

            let time_label = gtk::Label::new(Some("..."));
            time_label.add_css_class("dim-label");
            time_label.set_margin_start(12);

            let row = adw::ActionRow::builder().title(name).build();
            row.add_suffix(&button);
            row.add_suffix(&time_label);
            group.append(&row);

            let config_toggle = config.clone();
            button.connect_clicked(move |button| {
                let enabled = button.icon_name().as_deref() == Some(BELL_ON);
                if enabled {
                    button.set_icon_name(BELL_OFF);
                    button.set_tooltip_text(Some("Notifications off"));
                    config_toggle.set("Prayer", &format!("{name}_notify"), "No");
                } else {
                    button.set_icon_name(BELL_ON);
                    button.set_tooltip_text(Some("Notifications on"));
                    config_toggle.set("Prayer", &format!("{name}_notify"), "Yes");
                }
            });

            slots.borrow_mut().push(Slot {
                name,
                hhmm: String::new(),
                button,
                time_label,
            });
        }

        let slots_clock = slots.clone();
        let notified = Rc::new(RefCell::new(HashMap::<String, String>::new()));
        glib::timeout_add_seconds_local(1, move || {
            let now = chrono::Local::now().format("%H:%M").to_string();
            let snapshot: Vec<(String, String, bool)> = slots_clock
                .borrow()
                .iter()
                .map(|slot| {
                    (
                        slot.name.to_string(),
                        slot.hhmm.clone(),
                        slot.button.icon_name().as_deref() == Some(BELL_ON),
                    )
                })
                .collect();
            let rows: Vec<(&str, &str, bool)> = snapshot
                .iter()
                .map(|(name, time, bell)| (name.as_str(), time.as_str(), *bell))
                .collect();
            for name in due_prayer_names(&now, &rows, &mut notified.borrow_mut()) {
                let note = gio::Notification::new(&format!("Its time for {name}!"));
                app.send_notification(Some(&format!("eeman-{name}")), &note);
            }
            glib::ControlFlow::Continue
        });

        let date_w = date_label.downgrade();
        let hijri_w = hijri_label.downgrade();
        let tz_w = timezone_label.downgrade();
        let spinner_w = spinner.downgrade();
        let slots_fetch = slots.clone();
        let config_fetch = config.clone();

        crate::finish_on_main(
            async move {
                let (city, country) = get_location(&config_fetch).await;
                fetch_prayer_times(&city, &country, &config_fetch).await
            },
            move |result| {
                if let Some(spinner) = spinner_w.upgrade() {
                    spinner.stop();
                    spinner.set_visible(false);
                }
                match result {
                    Ok(data) => {
                        if let Some(date) = date_w.upgrade() {
                            date.set_label(&data.date.readable);
                        }
                        if let Some(hijri) = hijri_w.upgrade() {
                            hijri.set_label(&format!(
                                "{} {}, {} {}",
                                data.date.hijri.month_en,
                                data.date.hijri.day,
                                data.date.hijri.year,
                                data.date.hijri.designation
                            ));
                        }
                        if let Some(tz) = tz_w.upgrade() {
                            tz.set_label(&data.meta.timezone);
                        }
                        let times = [
                            data.timings.fajr,
                            data.timings.sunrise,
                            data.timings.dhuhr,
                            data.timings.asr,
                            data.timings.maghrib,
                            data.timings.isha,
                        ];
                        let mut slots = slots_fetch.borrow_mut();
                        for (slot, time) in slots.iter_mut().zip(times) {
                            slot.hhmm = hhmm(&time);
                            slot.time_label.set_label(&slot.hhmm);
                        }
                    }
                    Err(error) => {
                        if let Some(date) = date_w.upgrade() {
                            date.set_label(&format!("Could not load prayer times. {error}"));
                        }
                    }
                }
            },
        );

        Self { widget }
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.widget
    }
}

pub fn due_prayer_names<'a>(
    now: &str,
    rows: &[(&'a str, &str, bool)],
    notified: &mut HashMap<String, String>,
) -> Vec<&'a str> {
    let mut due = Vec::new();
    for (name, time, bell_on) in rows {
        if *bell_on && *time == now && notified.get(*name).map(String::as_str) != Some(now) {
            notified.insert((*name).to_string(), now.to_string());
            due.push(*name);
        }
    }
    due
}

fn hhmm(raw: &str) -> String {
    raw.split_whitespace().next().unwrap_or("").to_string()
}

fn json_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Number(number) => number.to_string(),
        _ => String::new(),
    }
}

async fn get_location(config: &AppConfig) -> (String, String) {
    let location_mode = config.get("Prayer", "location_mode").unwrap_or_default();

    if location_mode == "Automatic" {
        if let Ok(resp) = reqwest::get("http://ip-api.com/json/").await {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                let city = json["city"].as_str().unwrap_or("Unknown").to_string();
                let country = json["country"].as_str().unwrap_or("Unknown").to_string();
                config.set("Prayer", "city", &city);
                config.set("Prayer", "country", &country);
                return (city, country);
            }
        }
    }

    let city = config
        .get("Prayer", "city")
        .filter(|city| !city.is_empty() && city != "None")
        .unwrap_or_else(|| "Mecca".to_string());
    let country = config
        .get("Prayer", "country")
        .filter(|country| !country.is_empty() && country != "None")
        .unwrap_or_else(|| "Saudi Arabia".to_string());
    (city, country)
}

async fn fetch_prayer_times(
    city: &str,
    country: &str,
    config: &AppConfig,
) -> Result<PrayerData, String> {
    let method = config.get("Prayer", "method").unwrap_or_else(|| "2".to_string());
    let school = config
        .get("Prayer", "hanafi_school")
        .unwrap_or_else(|| "0".to_string());
    let method_mode = config.get("Prayer", "method_mode").unwrap_or_default();

    let client = reqwest::Client::new();
    let mut params = vec![
        ("city".to_string(), city.to_string()),
        ("country".to_string(), country.to_string()),
        ("school".to_string(), school),
    ];

    if method_mode == "Manual" {
        params.push(("method".to_string(), method));
    }

    let resp = client
        .get("http://api.aladhan.com/v1/timingsByCity")
        .query(&params)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let data = &json["data"];
    let timings = &data["timings"];

    Ok(PrayerData {
        timings: PrayerTimings {
            fajr: json_text(&timings["Fajr"]),
            sunrise: json_text(&timings["Sunrise"]),
            dhuhr: json_text(&timings["Dhuhr"]),
            asr: json_text(&timings["Asr"]),
            maghrib: json_text(&timings["Maghrib"]),
            isha: json_text(&timings["Isha"]),
        },
        date: PrayerDate {
            readable: json_text(&data["date"]["readable"]),
            hijri: HijriDate {
                day: json_text(&data["date"]["hijri"]["day"]),
                month_en: json_text(&data["date"]["hijri"]["month"]["en"]),
                year: json_text(&data["date"]["hijri"]["year"]),
                designation: json_text(&data["date"]["hijri"]["designation"]["abbreviated"]),
            },
        },
        meta: PrayerMeta {
            timezone: json_text(&data["meta"]["timezone"]),
        },
    })
}

struct PrayerData {
    timings: PrayerTimings,
    date: PrayerDate,
    meta: PrayerMeta,
}

struct PrayerTimings {
    fajr: String,
    sunrise: String,
    dhuhr: String,
    asr: String,
    maghrib: String,
    isha: String,
}

struct PrayerDate {
    readable: String,
    hijri: HijriDate,
}

struct HijriDate {
    day: String,
    month_en: String,
    year: String,
    designation: String,
}

struct PrayerMeta {
    timezone: String,
}

#[cfg(test)]
mod tests {
    use super::{due_prayer_names, hhmm};
    use std::collections::HashMap;

    #[test]
    fn notifies_once_per_minute() {
        let mut notified = HashMap::new();
        let rows = [("Fajr", "05:11", true), ("Sunrise", "05:11", false)];
        assert_eq!(
            due_prayer_names("05:11", &rows, &mut notified),
            vec!["Fajr"]
        );
        assert!(due_prayer_names("05:11", &rows, &mut notified).is_empty());
        let later = [("Fajr", "05:12", true)];
        assert_eq!(
            due_prayer_names("05:12", &later, &mut notified),
            vec!["Fajr"]
        );
    }

    #[test]
    fn strips_timezone_suffix() {
        assert_eq!(hhmm("05:30 (PKT)"), "05:30");
        assert_eq!(hhmm("05:30"), "05:30");
    }
}
