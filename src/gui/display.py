from datetime import datetime
import threading

import gi
from eeman.configuration import config, get_conf
from eeman.libs import setup

from . import preferences as pref
from . import welcome
from .ayah_view import AyahViewport

gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
gi.require_version("Notify", "0.7")
from gi.repository import Adw, Gio, Gtk, Notify, GLib  # noqa E:402

Notify.init("Ziqr")


def due_prayer_notifications(now, rows, notified):
    """Prayer names that should notify at `now` (HH:MM).

    `rows` is (name, time, bell_on). `notified` maps a name to the minute
    it already fired, so the same minute cannot notify twice.
    """
    due = []
    for name, hhmm, bell_on in rows:
        if bell_on and hhmm == now and notified.get(name) != now:
            notified[name] = now
            due.append(name)
    return due


class DisplayWindow(Adw.ApplicationWindow):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        get_conf()

        self.set_name(welcome.app_name)
        self.set_default_size(500, 600)
        self.set_hide_on_close(True)

        self._notified_minute = {}
        self._prayer_rows = []
        self._surahs_ready = False
        self._surah_data = None
        self._ayah_cache = {}
        self._surah_generation = 0
        self._inflight_number = None

        self.box_main = Gtk.Box(
            orientation=Gtk.Orientation.VERTICAL,
            halign=Gtk.Align.FILL,
            valign=Gtk.Align.FILL,
            hexpand=True,
            vexpand=True,
        )
        self.set_content(self.box_main)

        self.hb = Adw.HeaderBar(centering_policy=Adw.CenteringPolicy.STRICT)
        self.box_main.append(self.hb)

        # Create a menu
        preferences_action = Gio.SimpleAction.new("pref", None)
        about_action = Gio.SimpleAction.new("about", None)
        preferences_action.connect("activate", self.show_preferences)
        about_action.connect("activate", self.show_about)
        self.add_action(preferences_action)
        self.add_action(about_action)
        menu = Gio.Menu.new()
        menu.append("Preferences", "win.pref")
        menu.append("About", "win.about")
        self.popover = Gtk.PopoverMenu()
        self.popover.set_menu_model(menu)
        self.hamburger = Gtk.MenuButton()
        self.hamburger.set_popover(self.popover)
        self.hamburger.set_icon_name("open-menu-symbolic")  # Give it a nice icon

        # Add menu button to the header bar
        self.hb.pack_end(self.hamburger)
        self.stack = Adw.ViewStack()
        self.box_main.append(self.stack)

        # Squeezer
        self.sq_viewswitcher = Adw.Squeezer(
            halign=Gtk.Align.FILL,
        )
        self.sq_viewswitcher.set_switch_threshold_policy(
            Adw.FoldThresholdPolicy.NATURAL
        )
        self.sq_viewswitcher.set_transition_type(Adw.SqueezerTransitionType.CROSSFADE)
        self.sq_viewswitcher.set_xalign(1)
        self.sq_viewswitcher.set_homogeneous(True)
        self.hb.set_title_widget(self.sq_viewswitcher)

        # ViewSwitcher (wide)
        self.viewswitcher_wide = Adw.ViewSwitcher(
            halign=Gtk.Align.CENTER, margin_start=50, margin_end=50
        )
        self.viewswitcher_wide.set_policy(Adw.ViewSwitcherPolicy.WIDE)
        self.viewswitcher_wide.set_stack(self.stack)
        self.sq_viewswitcher.add(self.viewswitcher_wide)

        # ViewSwitcher (narrow)
        self.viewswitcher_narrow = Adw.ViewSwitcher(
            halign=Gtk.Align.CENTER,
        )
        self.viewswitcher_narrow.set_policy(Adw.ViewSwitcherPolicy.NARROW)
        self.viewswitcher_narrow.set_stack(self.stack)
        self.sq_viewswitcher.add(self.viewswitcher_narrow)

        # ViewSwitcherBar (bottom viewswitcher)
        self.viewswitcherbar = Adw.ViewSwitcherBar(vexpand=True, valign=Gtk.Align.END)
        self.viewswitcherbar.set_stack(self.stack)
        self.viewswitcherbar.set_reveal(False)
        self.box_main.append(self.viewswitcherbar)

        # Window Title
        self.wintitle = Adw.WindowTitle(title=welcome.app_name)
        self.sq_viewswitcher.add(self.wintitle)

        # Connect signals
        self.sq_viewswitcher.connect(
            "notify::visible-child", self.on_sq_get_visible_child
        )

        self._build_prayer_page()
        self._build_quran_page()

        # One clock for every prayer. Returning SOURCE_CONTINUE makes GLib
        # repeat this same timeout. Scheduling another timeout from here
        # used to double the callbacks every second until menu clicks stalled.
        GLib.timeout_add_seconds(1, self._on_prayer_clock)

        threading.Thread(target=self._fetch_prayer_times, daemon=True).start()
        threading.Thread(target=self._fetch_surah_index, daemon=True).start()

    def _build_prayer_page(self):
        self.page1 = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        self.stack.add_titled(self.page1, "page0", "Prayer")
        self.stack.get_page(self.page1).set_icon_name("alarm-symbolic")
        self.clamp = Adw.Clamp()
        self.page1.append(self.clamp)
        self.box_wrapper = Gtk.Box(
            spacing=10,
            margin_start=20,
            margin_end=20,
            margin_top=20,
            margin_bottom=20,
            orientation=Gtk.Orientation.VERTICAL,
        )
        self.clamp.set_child(self.box_wrapper)

        self.prayer_status = Gtk.Box(
            orientation=Gtk.Orientation.HORIZONTAL,
            spacing=8,
            halign=Gtk.Align.CENTER,
        )
        self.prayer_spinner = Gtk.Spinner(spinning=True)
        self.date_label = Gtk.Label(label="Loading prayer times...")
        self.prayer_status.append(self.prayer_spinner)
        self.prayer_status.append(self.date_label)
        self.timezone_label = Gtk.Label(label="", margin_top=50)
        self.hijri_date_label = Gtk.Label(label="", margin_bottom=50)
        self.box_wrapper.append(self.prayer_status)
        self.box_wrapper.append(self.hijri_date_label)

        self.prayers = ["Fajr", "Sunrise", "Dhuhr", "Asr", "Maghrib", "Isha"]
        for prayer in self.prayers:
            prayer_box = Gtk.Box()
            prayer_box.get_style_context().add_class("card")
            self.box_wrapper.append(prayer_box)
            prayer_label = Gtk.Label(
                label=prayer,
                margin_start=10,
                margin_end=10,
                margin_top=10,
                margin_bottom=10,
            )
            prayer_time_label = Gtk.Label(
                label="...",
                margin_start=10,
                margin_end=10,
                margin_top=10,
                margin_bottom=10,
                halign=Gtk.Align.END,
            )
            prayer_notify_button = Gtk.Button(
                halign=Gtk.Align.END,
                hexpand=True,
                has_frame=False,
            )
            if config["Prayer"]["%s_notify" % prayer] == "Yes":
                prayer_notify_button.set_icon_name("bell-outline-symbolic")
            else:
                prayer_notify_button.set_icon_name("bell-outline-none-symbolic")
            prayer_box.append(prayer_label)
            prayer_box.append(prayer_notify_button)
            prayer_box.append(prayer_time_label)
            prayer_notify_button.connect("clicked", self.set_notify, prayer)
            self._prayer_rows.append((prayer, prayer_time_label, prayer_notify_button))
        self.box_wrapper.append(self.timezone_label)

    def _build_quran_page(self):
        self.tb = Adw.ToolbarView()
        self.actionbar = Gtk.ActionBar()
        self.tb.add_top_bar(self.actionbar)
        self.stack.add_titled(self.tb, "page1", "Quran")
        self.stack.get_page(self.tb).set_icon_name("open-book-symbolic")
        self.select_surah = Gtk.DropDown()
        self.select_surah.set_hexpand(True)
        self.actionbar.pack_start(self.select_surah)
        self.surah_list = Gtk.StringList.new(["Loading surahs..."])
        self.select_surah.set_model(self.surah_list)
        self.select_surah.set_sensitive(False)

        self.quran_page_box = Gtk.Box(
            orientation=Gtk.Orientation.VERTICAL, vexpand=True
        )
        self.surah_heading_arabic = Gtk.Label(halign=Gtk.Align.CENTER, margin_top=12)
        self.surah_heading_english = Gtk.Label(
            halign=Gtk.Align.CENTER, margin_bottom=6
        )
        self.surah_heading_english.add_css_class("dim-label")
        self.quran_page_box.append(self.surah_heading_arabic)
        self.quran_page_box.append(self.surah_heading_english)

        self.quran_stack = Gtk.Stack(vexpand=True)
        self.quran_status = Gtk.Box(
            orientation=Gtk.Orientation.VERTICAL,
            spacing=12,
            valign=Gtk.Align.CENTER,
            halign=Gtk.Align.CENTER,
            vexpand=True,
        )
        self.quran_spinner = Gtk.Spinner()
        self.quran_status_label = Gtk.Label(
            label="Loading surahs...", wrap=True, margin_start=24, margin_end=24
        )
        self.quran_status_label.add_css_class("dim-label")
        self.quran_status.append(self.quran_spinner)
        self.quran_status.append(self.quran_status_label)
        self.ayah_view = AyahViewport()
        self.quran_stack.add_named(self.quran_status, "status")
        self.quran_stack.add_named(self.ayah_view, "ayahs")
        self.quran_page_box.append(self.quran_stack)

        self.clamp2 = Adw.Clamp()
        self.clamp2.set_child(self.quran_page_box)
        self.tb.set_content(self.clamp2)
        self._show_status("Loading surahs...", spinning=True)
        self.select_surah.connect("notify::selected-item", self.on_surah_select)

    def _fetch_prayer_times(self):
        try:
            setup.get_response()
            payload = {
                "date": setup.date,
                "hijri": setup.hijri_date,
                "timezone": setup.timezone,
                "prayer": dict(setup.prayer),
            }
        except Exception as exc:
            print("Prayer times: %s" % exc)
            payload = {"error": True}
        GLib.idle_add(self._apply_prayer_times, payload)

    def _apply_prayer_times(self, payload):
        self.prayer_spinner.stop()
        self.prayer_spinner.set_visible(False)
        prayer = payload.get("prayer") or {}
        if payload.get("error") or not prayer.get("Fajr"):
            self.date_label.set_label("Could not load prayer times.")
            return GLib.SOURCE_REMOVE
        self.date_label.set_label(payload.get("date") or "")
        self.hijri_date_label.set_label(payload.get("hijri") or "")
        self.timezone_label.set_label(payload.get("timezone") or "")
        for name, time_label, _button in self._prayer_rows:
            time_label.set_label(prayer.get(name) or "--:--")
        return GLib.SOURCE_REMOVE

    def _fetch_surah_index(self):
        data = setup.get_response_quran_surah_data()
        GLib.idle_add(self._apply_surah_index, data)

    def _apply_surah_index(self, data):
        if not data or not data.get("data"):
            self._show_status("Could not load the surah list.", spinning=False)
            return GLib.SOURCE_REMOVE
        self._surah_data = data
        self.surah_data = data
        names = []
        for surah in range(1, len(data["data"]) + 1):
            names.append(
                "%s. %s"
                % (surah, setup.get_quran_surah_name_english(surah, data))
            )
        self._surahs_ready = True
        self.select_surah.set_model(Gtk.StringList.new(names))
        self.select_surah.set_sensitive(True)
        selected = self.select_surah.get_selected()
        if selected == Gtk.INVALID_LIST_POSITION:
            selected = 0
        self._begin_surah_load(selected + 1)
        return GLib.SOURCE_REMOVE

    def _show_status(self, text, spinning):
        self.quran_status_label.set_label(text)
        if spinning:
            self.quran_spinner.start()
            self.quran_spinner.set_visible(True)
        else:
            self.quran_spinner.stop()
            self.quran_spinner.set_visible(False)
        self.quran_stack.set_visible_child(self.quran_status)

    def _show_ayahs(self, ayahs):
        self.quran_spinner.stop()
        self.ayah_view.set_ayahs(ayahs)
        self.quran_stack.set_visible_child(self.ayah_view)

    def _set_heading(self, number):
        self.surah_heading_arabic.set_label(
            setup.get_quran_surah_name_arabic(number, self._surah_data)
        )
        self.surah_heading_english.set_label(
            setup.get_quran_surah_name_english(number, self._surah_data)
        )

    def _begin_surah_load(self, number):
        if self._surah_data is not None:
            self._set_heading(number)
        cached = self._ayah_cache.get(number)
        if cached is not None:
            self._inflight_number = None
            self._show_ayahs(cached)
            return
        if self._inflight_number == number:
            return
        self._inflight_number = number
        self._surah_generation += 1
        generation = self._surah_generation
        name = "surah"
        if self._surah_data is not None:
            name = setup.get_quran_surah_name_english(number, self._surah_data)
        self._show_status("Loading %s..." % name, spinning=True)
        self.ayah_view.set_ayahs([])

        def work():
            try:
                ayahs = setup.fetch_surah_ayah_texts(number)
                error = None
            except Exception as exc:
                print("Surah %s: %s" % (number, exc))
                ayahs = None
                error = str(exc)
            GLib.idle_add(self._apply_ayahs, generation, number, ayahs, error)

        threading.Thread(target=work, daemon=True).start()

    def _apply_ayahs(self, generation, number, ayahs, error):
        if generation != self._surah_generation:
            return GLib.SOURCE_REMOVE
        self._inflight_number = None
        if error or not ayahs:
            self._show_status("Could not load this surah.", spinning=False)
            return GLib.SOURCE_REMOVE
        self._ayah_cache[number] = ayahs
        if self._surah_data is not None:
            self._set_heading(number)
        self._show_ayahs(ayahs)
        return GLib.SOURCE_REMOVE

    def _on_prayer_clock(self):
        now = datetime.now().strftime("%H:%M")
        rows = []
        for prayer, time_label, notify_btn in self._prayer_rows:
            bell_on = notify_btn.get_icon_name() == "bell-outline-symbolic"
            rows.append((prayer, time_label.get_label(), bell_on))
        for prayer in due_prayer_notifications(now, rows, self._notified_minute):
            Notify.Notification.new("Its time for %s!" % prayer).show()
        return GLib.SOURCE_CONTINUE

    def set_notify(self, notify_btn, prayer):
        if notify_btn.get_icon_name() == "bell-outline-symbolic":
            notify_btn.set_icon_name("bell-outline-none-symbolic")
            setup.set_config("Prayer", "%s_notify" % prayer, "No")
        elif notify_btn.get_icon_name() == "bell-outline-none-symbolic":
            notify_btn.set_icon_name("bell-outline-symbolic")
            setup.set_config("Prayer", "%s_notify" % prayer, "Yes")

    def on_surah_select(self, select_surah, event):
        if not self._surahs_ready:
            return
        selected = select_surah.get_selected()
        if selected == Gtk.INVALID_LIST_POSITION:
            return
        self._begin_surah_load(selected + 1)

    def on_sq_get_visible_child(self, widget, event):
        if self.sq_viewswitcher.get_visible_child() == self.wintitle:
            self.viewswitcherbar.set_reveal(True)
        else:
            self.viewswitcherbar.set_reveal(False)

    def show_preferences(self, action, params):
        self.pref_window = Adw.PreferencesWindow()
        self.pref_window.add(pref.PreferencesPage())
        self.pref_window.present()

    def show_about(self, action, params):
        self.about_window = Adw.AboutWindow()
        self.about_window.present()
        self.about_window.set_application_name("Ziqr")
        self.about_window.set_application_icon("pro.ddroid.Ziqr")
        self.about_window.set_developer_name("ddroid")
        self.about_window.set_version("0.1.2")
        self.about_window.set_license_type(Gtk.License.GPL_3_0)
        self.about_window.set_comments(
            "Ziqr is an app to track prayer times, read the quran etc. "
            "Its open source and written in Python and Gtk 4. "
        )
