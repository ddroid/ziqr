# Ziqr - Rust GTK4 Version

A high-performance Islamic prayer times and Quran reader application built with Rust and GTK4/libadwaita.

## Performance Features

### Lazy Loading for Quran Surahs
- **Virtualized ListView**: Only visible ayahs are rendered (not all 286 ayahs of Al-Baqarah at once)
- **On-demand fetching**: Surahs are fetched only when selected
- **In-memory caching**: Previously viewed surahs load instantly from cache
- **Async API calls**: Non-blocking HTTP requests using `reqwest` + `glib::spawn_future_local`
- **Parallel fetching**: Arabic and English translations fetched concurrently with `futures::try_join!`
- **Efficient data binding**: Uses `glib::BoxedAnyObject` with `SignalListItemFactory` for zero-copy item model

### Key Optimizations
1. **Single API call** for all 114 surah names (loaded once at startup)
2. **Concurrent requests** for surah detail (Arabic + English in parallel)
3. **HashMap cache** with O(1) lookup for previously viewed surahs
4. **Weak references** to prevent memory leaks in async closures
5. **LTO + single codegen unit** in release builds for maximum performance

## Project Structure

```
src_rust/
├── Cargo.toml              # Dependencies and build config
├── build.rs                # Build script
├── config.ini              # Default configuration
└── src/
    ├── main.rs             # Entry point
    ├── app.rs              # Application setup
    ├── config.rs           # INI configuration management
    ├── main_window.rs      # Main window with tabs
    ├── prayer.rs           # Prayer times page (async API)
    ├── preferences.rs      # Preferences window
    ├── welcome.rs          # Welcome/onboarding flow
    └── quran/
        ├── mod.rs          # Quran reader with lazy loading
        └── surah_data.rs   # Data structures and API functions
```

## Dependencies

- `gtk4` 0.9 - GTK4 bindings for Rust
- `libadwaita` 0.7 - GNOME libadwaita bindings
- `reqwest` 0.12 - Async HTTP client (rustls-tls)
- `serde` / `serde_json` - JSON serialization
- `futures` - Async utilities (try_join!, join!)
- `ini` - INI file parsing
- `chrono` - Date/time handling
- `dirs` - Platform-specific directories

## Building

### Prerequisites

Install system dependencies (Ubuntu/Debian):
```bash
sudo apt install \
  libgtk-4-dev \
  libadwaita-1-dev \
  build-essential \
  pkg-config \
  libssl-dev
```

### Compile

```bash
cd src_rust
cargo build --release
```

### Run

```bash
cargo run --release
```

## Comparison with Python Version

| Feature | Python | Rust |
|---------|--------|------|
| UI rendering | All widgets at once | Virtualized (only visible) |
| API calls | Blocking (main thread) | Async (non-blocking) |
| Surah loading | 1-114 only (missing #114) | All 114 surahs |
| Caching | None | In-memory HashMap |
| Concurrent fetch | No | Yes (Arabic + English) |
| Memory usage | High (all widgets) | Low (lazy loading) |
| Startup time | Slower | Faster (LTO optimized) |

## API Endpoints

- **Prayer times**: `http://api.aladhan.com/v1/timingsByCity`
- **Quran surahs**: `https://api.alquran.cloud/v1/surah`
- **Quran ayahs**: `https://api.alquran.cloud/v1/surah/{number}` (Arabic)
- **Quran translation**: `https://api.alquran.cloud/v1/surah/{number}/en.sahih` (English)
- **IP geolocation**: `http://ip-api.com/json/`

## Configuration

Settings are stored in `~/.local/share/eeman/config.ini` (Linux) with sections:
- `[Prayer]` - Location, calculation method, school, notifications
- `[Appearance]` - Theme (Dark/Light)
- `[App]` - First run flag

## License

GPL-3.0-or-later

## Credits

Original Python application by ddroid (SHuRiKeN)
- Islamic Network APIs: https://islamic.network/ and https://alquran.cloud/
- Rust GTK4 conversion with performance optimizations
