use gtk::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::surah_data::Ayah;
use crate::fonts::apply_arabic_font;

const ROW_BUFFER: i32 = 4;
const ROW_GAP: i32 = 10;
const DEFAULT_ROW_HEIGHT: i32 = 120;

pub fn visible_range(heights: &[i32], scroll_y: f64, page_size: f64, buffer: i32) -> (usize, usize) {
    let count = heights.len();
    if count == 0 {
        return (0, 0);
    }
    let page_size = if page_size <= 1.0 { 600.0 } else { page_size };
    let scroll_y = scroll_y.max(0.0);

    let mut start = count - 1;
    let mut covered = 0.0;
    for (index, height) in heights.iter().enumerate() {
        if covered + *height as f64 > scroll_y {
            start = index;
            break;
        }
        covered += *height as f64;
    }

    let mut end = count;
    let mut covered = 0.0;
    let limit = scroll_y + page_size;
    for (index, height) in heights.iter().enumerate() {
        covered += *height as f64;
        if covered >= limit {
            end = index + 1;
            break;
        }
    }

    let start = start.saturating_sub(buffer.max(0) as usize);
    let end = (end + buffer.max(0) as usize).min(count);
    if end < start {
        (start, start)
    } else {
        (start, end)
    }
}

pub fn estimate_ayah_height(ayah: &Ayah) -> i32 {
    let ar_len = ayah.text_arabic.chars().count();
    let en_len = ayah.text_english.chars().count();
    let ar_lines = (ar_len / 30).max(1);
    let en_lines = (en_len / 45).max(1);
    let lines = ar_lines.max(en_lines);
    (lines as i32 * 46 + 48).max(DEFAULT_ROW_HEIGHT)
}

struct Flags {
    lock_scroll: Cell<bool>,
    in_refresh: Cell<bool>,
    measure_queued: Cell<bool>,
    measured_width: Cell<i32>,
}

struct RowWidgets {
    root: gtk::Box,
    english: gtk::Label,
    arabic: gtk::Label,
    bound_ayah: Cell<Option<u32>>,
}

struct ViewState {
    scrolled: gtk::ScrolledWindow,
    top: gtk::Box,
    rows: gtk::Box,
    bottom: gtk::Box,
    ayahs: Vec<Rc<Ayah>>,
    heights: Vec<i32>,
    measured: Vec<bool>,
    range: (usize, usize),
    pool: Vec<RowWidgets>,
    spacer_top: Option<i32>,
    spacer_bottom: Option<i32>,
}

impl ViewState {
    fn refresh(&mut self, flags: &Flags) -> bool {
        if flags.in_refresh.get() {
            return false;
        }
        flags.in_refresh.set(true);

        let adjustment = self.scrolled.vadjustment();
        let (start, end) = visible_range(
            &self.heights,
            adjustment.value(),
            adjustment.page_size(),
            ROW_BUFFER,
        );

        let mut needs_measure = false;
        if (start, end) != self.range {
            self.range = (start, end);
            self.rebind(start, end);
            self.apply_spacers(start, end);
            needs_measure = true;
        }

        flags.in_refresh.set(false);
        needs_measure
    }

    fn apply_spacers(&mut self, start: usize, end: usize) -> bool {
        let top: i32 = self.heights[..start].iter().sum();
        let bottom: i32 = self.heights[end..].iter().sum();
        if Some(top) == self.spacer_top && Some(bottom) == self.spacer_bottom {
            return false;
        }
        self.spacer_top = Some(top);
        self.spacer_bottom = Some(bottom);
        self.top.set_size_request(-1, top);
        self.bottom.set_size_request(-1, bottom);
        true
    }

    fn rebind(&mut self, start: usize, end: usize) {
        let needed = end.saturating_sub(start);
        while self.pool.len() < needed {
            let row = make_row();
            self.rows.append(&row.root);
            self.pool.push(row);
        }

        for i in 0..self.pool.len() {
            let row = &self.pool[i];
            if i < needed {
                let ayah = &self.ayahs[start + i];
                if row.bound_ayah.get() != Some(ayah.number) {
                    row.english
                        .set_label(&format!("{}. {}", ayah.number, ayah.text_english));
                    row.arabic.set_label(&ayah.text_arabic);
                    row.bound_ayah.set(Some(ayah.number));
                }
                row.root.set_visible(true);
            } else {
                row.root.set_visible(false);
                row.bound_ayah.set(None);
            }
        }
    }

    fn measure(&mut self, flags: &Flags, queue: &mut bool) {
        let width = self.scrolled.width();
        if width <= 1 {
            return;
        }
        let (start, end) = self.range;
        if end <= start {
            return;
        }

        let adjustment = self.scrolled.vadjustment();
        let scroll_y = adjustment.value();
        let mut changed = false;
        let mut delta_above = 0.0;
        let visible = end - start;

        for offset in 0..visible {
            let index = start + offset;
            if self.measured.get(index).copied().unwrap_or(false) {
                continue;
            }
            let (_min, natural, _, _) = self.pool[offset].root.measure(gtk::Orientation::Vertical, width);
            if natural < 24 {
                continue;
            }
            let height = natural + ROW_GAP;
            if (height - self.heights[index]).abs() > 2 {
                let row_top: i32 = self.heights[..index].iter().sum();
                if (row_top as f64) < scroll_y {
                    delta_above += (height - self.heights[index]) as f64;
                }
                self.heights[index] = height;
                changed = true;
            }
            if index < self.measured.len() {
                self.measured[index] = true;
            }
        }

        flags.measured_width.set(width);
        if !changed {
            return;
        }

        flags.lock_scroll.set(true);
        if delta_above != 0.0 {
            adjustment.set_value((scroll_y + delta_above).max(0.0));
        }
        self.apply_spacers(start, end);
        flags.lock_scroll.set(false);

        if self.refresh(flags) {
            *queue = true;
        }
    }
}

fn make_row() -> RowWidgets {
    let root = gtk::Box::new(gtk::Orientation::Horizontal, 50);
    root.add_css_class("card");
    root.set_margin_bottom(ROW_GAP);

    let english = gtk::Label::new(None);
    english.set_wrap(true);
    english.set_wrap_mode(gtk::pango::WrapMode::Word);
    english.set_xalign(0.0);
    english.set_hexpand(true);
    english.set_halign(gtk::Align::Start);
    english.set_margin_start(10);
    english.set_margin_end(10);
    english.set_margin_top(10);
    english.set_margin_bottom(10);

    let arabic = gtk::Label::new(None);
    arabic.set_wrap(true);
    arabic.set_wrap_mode(gtk::pango::WrapMode::Word);
    arabic.set_xalign(1.0);
    arabic.set_hexpand(true);
    arabic.set_halign(gtk::Align::End);
    arabic.set_margin_start(10);
    arabic.set_margin_end(10);
    arabic.set_margin_top(10);
    arabic.set_margin_bottom(10);
    arabic.add_css_class("arabic-text");
    apply_arabic_font(&arabic, 28);

    root.append(&english);
    root.append(&arabic);
    RowWidgets {
        root,
        english,
        arabic,
        bound_ayah: Cell::new(None),
    }
}

pub struct AyahViewport {
    scrolled: gtk::ScrolledWindow,
    state: Rc<RefCell<ViewState>>,
    flags: Rc<Flags>,
}

impl AyahViewport {
    pub fn new() -> Self {
        let scrolled = gtk::ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scrolled.set_propagate_natural_height(false);

        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.set_hexpand(true);
        let top = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let rows = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let bottom = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.append(&top);
        content.append(&rows);
        content.append(&bottom);
        scrolled.set_child(Some(&content));

        let state = Rc::new(RefCell::new(ViewState {
            scrolled: scrolled.clone(),
            top,
            rows,
            bottom,
            ayahs: Vec::new(),
            heights: Vec::new(),
            measured: Vec::new(),
            range: (usize::MAX, usize::MAX),
            pool: Vec::new(),
            spacer_top: None,
            spacer_bottom: None,
        }));
        let flags = Rc::new(Flags {
            lock_scroll: Cell::new(false),
            in_refresh: Cell::new(false),
            measure_queued: Cell::new(false),
            measured_width: Cell::new(-1),
        });

        let state_value = state.clone();
        let flags_value = flags.clone();
        scrolled.vadjustment().connect_value_changed(move |_| {
            if flags_value.lock_scroll.get() || flags_value.in_refresh.get() {
                return;
            }
            let needs = state_value.borrow_mut().refresh(&flags_value);
            if needs {
                queue_measure(&state_value, &flags_value);
            }
        });
        let state_page = state.clone();
        let flags_page = flags.clone();
        scrolled
            .vadjustment()
            .connect_page_size_notify(move |_| {
                if flags_page.lock_scroll.get() || flags_page.in_refresh.get() {
                    return;
                }
                let needs = state_page.borrow_mut().refresh(&flags_page);
                if needs {
                    queue_measure(&state_page, &flags_page);
                }
            });

        let state_width = state.clone();
        let flags_width = flags.clone();
        scrolled.connect_notify_local(Some("width"), move |widget, _| {
            let width = widget.width();
            if width <= 1 || (width - flags_width.measured_width.get()).abs() < 16 {
                return;
            }
            flags_width.measured_width.set(width);
            let mut st = state_width.borrow_mut();
            st.measured.fill(false);
            queue_measure(&state_width, &flags_width);
        });

        let state_map = state.clone();
        let flags_map = flags.clone();
        scrolled.connect_map(move |_| {
            let _ = state_map.borrow_mut().refresh(&flags_map);
            queue_measure(&state_map, &flags_map);
        });

        Self {
            scrolled,
            state,
            flags,
        }
    }

    pub fn widget(&self) -> &gtk::ScrolledWindow {
        &self.scrolled
    }

    pub fn set_ayahs(&self, ayahs: Vec<Rc<Ayah>>) {
        let mut state = self.state.borrow_mut();
        state.heights = ayahs.iter().map(|a| estimate_ayah_height(a)).collect();
        state.measured = vec![false; ayahs.len()];
        state.ayahs = ayahs;
        state.range = (usize::MAX, usize::MAX);
        state.spacer_top = None;
        state.spacer_bottom = None;
        self.flags.lock_scroll.set(true);
        state.scrolled.vadjustment().set_value(0.0);
        self.flags.lock_scroll.set(false);
        let needs = state.refresh(&self.flags);
        drop(state);
        if needs {
            queue_measure(&self.state, &self.flags);
        }
    }
}

fn queue_measure(state: &Rc<RefCell<ViewState>>, flags: &Rc<Flags>) {
    if flags.measure_queued.get() {
        return;
    }
    flags.measure_queued.set(true);
    let state = state.clone();
    let flags = flags.clone();
    glib::idle_add_local(move || {
        flags.measure_queued.set(false);
        let mut again = false;
        state.borrow_mut().measure(&flags, &mut again);
        if again {
            queue_measure(&state, &flags);
        }
        glib::ControlFlow::Break
    });
}

#[cfg(test)]
mod tests {
    use super::visible_range;

    #[test]
    fn empty_range() {
        assert_eq!(visible_range(&[], 0.0, 600.0, 4), (0, 0));
    }

    #[test]
    fn top_of_long_surah_stays_small() {
        let heights = vec![100; 286];
        let (start, end) = visible_range(&heights, 0.0, 600.0, 4);
        assert!(end - start < 20, "{start} {end}");
        assert_eq!(start, 0);
    }

    #[test]
    fn middle_of_long_surah_stays_small() {
        let heights = vec![100; 286];
        let (start, end) = visible_range(&heights, 8000.0, 600.0, 4);
        assert!(end - start < 20, "{start} {end}");
        assert!(start > 70, "{start}");
    }
}
