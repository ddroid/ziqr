use dirs::data_dir;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

extern crate ini;
use ini::Ini;

#[derive(Clone)]
pub struct AppConfig {
    inner: Arc<RwLock<Ini>>,
    path: PathBuf,
}

impl AppConfig {
    pub fn new() -> Self {
        let data_path = data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("eeman");

        fs::create_dir_all(&data_path).ok();

        let path = data_path.join("config.ini");

        // Copy default config if not exists
        if !path.exists() {
            let default = include_str!("../config.ini");
            fs::write(&path, default).ok();
        }

        let conf = Ini::load_from_file(&path).unwrap_or_else(|_| {
            let default = include_str!("../config.ini");
            Ini::load_from_str(default).unwrap()
        });

        Self {
            inner: Arc::new(RwLock::new(conf)),
            path,
        }
    }

    pub fn get(&self, section: &str, key: &str) -> Option<String> {
        self.inner
            .read()
            .ok()
            .and_then(|conf| conf.general_section().get(key).map(|s| s.to_string()))
            .or_else(|| {
                self.inner
                    .read()
                    .ok()
                    .and_then(|conf| conf.section(Some(section))?.get(key).map(|s| s.to_string()))
            })
    }

    pub fn set(&self, section: &str, key: &str, value: &str) {
        if let Ok(mut conf) = self.inner.write() {
            conf.with_section(Some(section)).set(key, value);
            conf.write_to_file(&self.path).ok();
        }
    }

}
