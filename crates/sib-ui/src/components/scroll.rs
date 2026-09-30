use egui::ScrollArea;
use egui::scroll_area::ScrollBarVisibility;

pub fn vertical() -> ScrollArea {
    ScrollArea::vertical().scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
}

pub fn horizontal() -> ScrollArea {
    ScrollArea::horizontal().scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
}
