use egui::{Grid, RichText, Ui};

use crate::theme::{Palette, ROW_HEIGHT};

pub struct Table<'a> {
    id: &'a str,
    columns: &'a [&'a str],
}

impl<'a> Table<'a> {
    pub fn new(id: &'a str, columns: &'a [&'a str]) -> Self {
        Self { id, columns }
    }

    pub fn show(self, ui: &mut Ui, add_rows: impl FnOnce(&mut Ui)) {
        Grid::new(self.id)
            .num_columns(self.columns.len())
            .striped(true)
            .min_row_height(ROW_HEIGHT)
            .spacing([16.0, 0.0])
            .show(ui, |ui| {
                header_row(ui, self.columns);
                add_rows(ui);
            });
    }
}

pub fn header_row(ui: &mut Ui, columns: &[&str]) {
    let p = Palette::current(ui.ctx());
    for column in columns {
        ui.label(
            RichText::new(column.to_uppercase())
                .small()
                .color(p.text_secondary),
        );
    }
    ui.end_row();
}
