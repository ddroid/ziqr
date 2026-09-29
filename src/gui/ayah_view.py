import gi

gi.require_version("Gtk", "4.0")
from gi.repository import GLib, Gtk

# Rows kept built just outside the viewport, so a small scroll does not
# have to construct widgets on the frame they appear.
ROW_BUFFER = 4
ROW_GAP = 10
DEFAULT_ROW_HEIGHT = 120


def visible_range(heights, scroll_y, page_size, buffer=ROW_BUFFER):
    """Indexes [start, end) of rows that intersect the viewport, plus buffer."""
    count = len(heights)
    if count == 0:
        return 0, 0
    if page_size <= 1:
        page_size = 600
    scroll_y = max(0.0, scroll_y)

    start = count - 1
    covered = 0
    for index, height in enumerate(heights):
        if covered + height > scroll_y:
            start = index
            break
        covered += height

    end = count
    covered = 0
    limit = scroll_y + page_size
    for index, height in enumerate(heights):
        covered += height
        if covered >= limit:
            end = index + 1
            break

    start = max(0, start - buffer)
    end = min(count, end + buffer)
    if end < start:
        end = start
    return start, end


class AyahViewport(Gtk.ScrolledWindow):
    """Scrollable ayah list that only builds the rows on screen.

    A surah such as Al-Baqarah has 286 ayahs. Building a widget for each
    one, on the main thread, is what made the Quran page stall. Spacers
    stand in for the rows above and below the window.
    """

    def __init__(self):
        super().__init__()
        self.set_vexpand(True)
        self.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        self.set_propagate_natural_height(False)

        self._ayahs = []
        self._heights = []
        self._range = (-1, -1)
        self._pool = []
        self._spacer_top = None
        self._spacer_bottom = None
        self._lock_scroll = False
        self._in_refresh = False
        self._measure_queued = False
        self._measure_passes = 0
        self._measured_width = -1

        self._content = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        self._content.set_hexpand(True)
        self._top = Gtk.Box()
        self._rows = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        self._bottom = Gtk.Box()
        self._content.append(self._top)
        self._content.append(self._rows)
        self._content.append(self._bottom)
        self.set_child(self._content)

        adjustment = self.get_vadjustment()
        adjustment.connect("value-changed", self._on_adjustment)
        adjustment.connect("notify::page-size", self._on_adjustment)
        self.connect("notify::width", self._on_width)
        self.connect("map", self._on_map)

    def set_ayahs(self, ayahs):
        """Replace the list. `ayahs` is (number, english, arabic)."""
        self._ayahs = list(ayahs)
        self._heights = [DEFAULT_ROW_HEIGHT] * len(self._ayahs)
        self._range = (-1, -1)
        self._spacer_top = None
        self._spacer_bottom = None
        self._measure_passes = 0
        self._lock_scroll = True
        self.get_vadjustment().set_value(0)
        self._lock_scroll = False
        self._refresh()

    def visible_row_count(self):
        count = 0
        child = self._rows.get_first_child()
        while child is not None:
            count += 1
            child = child.get_next_sibling()
        return count

    def _on_adjustment(self, *_args):
        if self._lock_scroll or self._in_refresh:
            return
        self._refresh()

    def _on_width(self, *_args):
        width = self.get_width()
        if width <= 1 or abs(width - self._measured_width) < 16:
            return
        self._measured_width = width
        self._measure_passes = 0
        self._queue_measure()

    def _on_map(self, *_args):
        self._measure_passes = 0
        self._refresh()
        self._queue_measure()

    def _refresh(self):
        if self._in_refresh:
            return
        self._in_refresh = True
        needs_measure = False
        try:
            for _pass in range(3):
                adjustment = self.get_vadjustment()
                start, end = visible_range(
                    self._heights,
                    adjustment.get_value(),
                    adjustment.get_page_size(),
                )
                rebound = (start, end) != self._range
                if rebound:
                    self._range = (start, end)
                    self._rebind(start, end)
                    needs_measure = True
                spacers_changed = self._apply_spacers(start, end)
                if not rebound and not spacers_changed:
                    break
        finally:
            self._in_refresh = False
        if needs_measure:
            self._queue_measure()

    def _apply_spacers(self, start, end):
        top = sum(self._heights[:start])
        bottom = sum(self._heights[end:])
        if top == self._spacer_top and bottom == self._spacer_bottom:
            return False
        self._spacer_top = top
        self._spacer_bottom = bottom
        self._top.set_size_request(-1, int(top))
        self._bottom.set_size_request(-1, int(bottom))
        return True

    def _make_row(self):
        row = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=50)
        row.add_css_class("card")
        row.set_margin_bottom(ROW_GAP)
        english = Gtk.Label(
            wrap=True,
            xalign=0,
            hexpand=True,
            halign=Gtk.Align.START,
            margin_start=10,
            margin_end=10,
            margin_top=10,
            margin_bottom=10,
        )
        english.set_wrap_mode(Gtk.WrapMode.WORD)
        arabic = Gtk.Label(
            wrap=True,
            xalign=1,
            hexpand=True,
            halign=Gtk.Align.END,
            margin_start=10,
            margin_end=10,
            margin_top=10,
            margin_bottom=10,
        )
        arabic.set_wrap_mode(Gtk.WrapMode.WORD)
        row.append(english)
        row.append(arabic)
        row.english_label = english
        row.arabic_label = arabic
        return row

    def _rebind(self, start, end):
        needed = end - start
        while len(self._pool) < needed:
            self._pool.append(self._make_row())

        for row in self._pool:
            parent = row.get_parent()
            if parent is not None:
                parent.remove(row)

        for offset, index in enumerate(range(start, end)):
            number, english, arabic = self._ayahs[index]
            row = self._pool[offset]
            row.ayah_index = index
            row.english_label.set_label("%s. %s" % (number, english))
            row.arabic_label.set_label(arabic)
            self._rows.append(row)

    def _queue_measure(self):
        if self._measure_queued:
            return
        self._measure_queued = True
        GLib.idle_add(self._measure_rows)

    def _measure_rows(self):
        self._measure_queued = False
        if self._measure_passes >= 8:
            return GLib.SOURCE_REMOVE
        width = self.get_width()
        if width <= 1:
            return GLib.SOURCE_REMOVE

        start, end = self._range
        if end <= start:
            return GLib.SOURCE_REMOVE

        adjustment = self.get_vadjustment()
        scroll_y = adjustment.get_value()
        changed = False
        delta_above = 0
        child = self._rows.get_first_child()
        while child is not None:
            index = child.ayah_index
            _minimum, natural, _baseline_min, _baseline_nat = child.measure(
                Gtk.Orientation.VERTICAL, width
            )
            # A label that has not been given a width yet reports a tiny
            # height. Trusting that would mark every row as a few pixels
            # and the window would build them all.
            if natural < 24:
                continue
            height = natural + ROW_GAP
            if abs(height - self._heights[index]) > 2:
                row_top = sum(self._heights[:index])
                if row_top < scroll_y:
                    delta_above += height - self._heights[index]
                self._heights[index] = height
                changed = True
            child = child.get_next_sibling()

        if not changed:
            self._measured_width = width
            return GLib.SOURCE_REMOVE

        self._measure_passes += 1
        self._measured_width = width
        self._lock_scroll = True
        if delta_above:
            adjustment.set_value(max(0, scroll_y + delta_above))
        self._spacer_top = None
        self._lock_scroll = False
        self._refresh()
        return GLib.SOURCE_REMOVE
