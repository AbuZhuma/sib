use egui::{Grid, Id, RichText, Ui};

use super::sort::{Sort, SortColumn, sort_header};
use crate::components::scroll;
use crate::theme::{Palette, ROW_HEIGHT};

const SORT_KEY: &str = "sort";

pub struct Table<'a> {
    id: &'a str,
    columns: &'a [&'a str],
    sortable: &'a [SortColumn],
    default_sort: Option<Sort>,
}

impl<'a> Table<'a> {
    pub fn new(id: &'a str, columns: &'a [&'a str]) -> Self {
        Self {
            id,
            columns,
            sortable: &[],
            default_sort: None,
        }
    }

    pub fn sortable(mut self, columns: &'a [SortColumn], default: Sort) -> Self {
        self.sortable = columns;
        self.default_sort = Some(default);
        self
    }

    pub fn show(self, ui: &mut Ui, add_rows: impl FnOnce(&mut Ui)) {
        self.show_sorted(ui, |ui, _| add_rows(ui));
    }

    pub fn show_sorted(self, ui: &mut Ui, add_rows: impl FnOnce(&mut Ui, Sort)) {
        let sort_id = Id::new((self.id, SORT_KEY));
        let mut sort: Option<Sort> = ui.ctx().data(|d| d.get_temp(sort_id)).or(self.default_sort);
        scroll::horizontal()
            .id_salt((self.id, "scroll"))
            .show(ui, |ui| {
                Grid::new(self.id)
                    .num_columns(self.columns.len())
                    .striped(true)
                    .min_row_height(ROW_HEIGHT)
                    .spacing([16.0, 0.0])
                    .show(ui, |ui| {
                        self.header(ui, &mut sort);
                        add_rows(ui, sort.unwrap_or(Sort::ascending(0)));
                    });
            });
        if let Some(sort) = sort {
            ui.ctx().data_mut(|d| d.insert_temp(sort_id, sort));
        }
    }

    fn header(&self, ui: &mut Ui, sort: &mut Option<Sort>) {
        let p = Palette::current(ui.ctx());
        for (index, column) in self.columns.iter().enumerate() {
            match self.sortable.iter().find(|c| c.index == index) {
                Some(sortable) => sort_header(ui, column, *sortable, sort),
                None => {
                    ui.label(
                        RichText::new(column.to_uppercase())
                            .small()
                            .color(p.text_secondary),
                    );
                }
            }
        }
        ui.end_row();
    }
}
