use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use gtk::prelude::*;

/// Install Arabic fonts to user's local font directory.
/// Fonts in ~/.local/share/fonts/ are auto-detected by fontconfig/GTK4.
///
/// The file is rewritten whenever the embedded copy differs, so a repaired
/// font replaces an older install instead of sitting behind it.
pub fn register_fonts() {
    let font_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("fonts");

    fs::create_dir_all(&font_dir).ok();

    let fonts: [(&str, &[u8]); 1] = [
        ("Hafs.ttf", include_bytes!("../fonts/Hafs.ttf")),
    ];

    let mut changed = false;
    for (filename, data) in fonts {
        let font_path = font_dir.join(filename);
        let current = fs::read(&font_path).ok();
        if current.as_deref() != Some(data) {
            if fs::write(&font_path, data).is_ok() {
                changed = true;
            }
        }
    }
    if changed {
        // Rebuild fontconfig cache so GTK4 picks up the new font
        rebuild_font_cache();
    }
}

/// Apply the Hafs family to `label` once the font is installed.
/// The family lookup runs once, not once per ayah row.
pub fn apply_arabic_font(label: &gtk::Label, points: i32) {
    let Some(family) = arabic_family(label) else {
        return;
    };
    let desc = gtk::pango::FontDescription::from_string(&format!("{family} {points}"));
    let attr = gtk::pango::AttrFontDesc::new(&desc);
    let attrs = gtk::pango::AttrList::new();
    attrs.insert(attr);
    label.set_attributes(Some(&attrs));
}

fn arabic_family(label: &gtk::Label) -> Option<String> {
    static CACHE: OnceLock<Option<String>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            let ctx = label.pango_context();
            let map = ctx.font_map()?;
            map.list_families().into_iter().find_map(|family| {
                let name = family.name().to_string();
                if name.contains("KFGQPC") || name.contains("Hafs") {
                    Some(name)
                } else {
                    None
                }
            })
        })
        .clone()
}

/// Rebuild fontconfig cache after installing new fonts
fn rebuild_font_cache() {
    std::process::Command::new("fc-cache")
        .arg("-f")
        .arg(dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("fonts"))
        .output()
        .ok();
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_hafs_font_exists() {
        let font_data = include_bytes!("../fonts/Hafs.ttf");
        assert!(font_data.len() > 0, "Hafs.ttf should not be empty");
        assert!(font_data.len() > 4, "Font file should have valid header");
    }
}
