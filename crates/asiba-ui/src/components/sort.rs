use std::cmp::Ordering;

use egui::{CursorIcon, Label, RichText, Sense, Ui};

use crate::theme::{GAP_SMALL, Palette};

const ARROW_ASCENDING: &str = "▲";
const ARROW_DESCENDING: &str = "▼";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sort {
    pub column: usize,
    pub is_descending: bool,
}

impl Sort {
    pub const fn descending(column: usize) -> Self {
        Self {
            column,
            is_descending: true,
        }
    }

    pub const fn ascending(column: usize) -> Self {
        Self {
            column,
            is_descending: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SortColumn {
    pub index: usize,
    pub descending_first: bool,
}

impl SortColumn {
    pub const fn number(index: usize) -> Self {
        Self {
            index,
            descending_first: true,
        }
    }

    pub const fn text(index: usize) -> Self {
        Self {
            index,
            descending_first: false,
        }
    }

    fn toggled(self, current: Option<Sort>) -> Sort {
        match current {
            Some(sort) if sort.column == self.index => Sort {
                column: self.index,
                is_descending: !sort.is_descending,
            },
            _ => Sort {
                column: self.index,
                is_descending: self.descending_first,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SortKey {
    Missing,
    Number(f64),
    Text(String),
}

impl SortKey {
    pub fn text(value: &str) -> Self {
        Self::Text(value.to_lowercase())
    }

    pub fn number(value: impl Into<f64>) -> Self {
        Self::Number(value.into())
    }

    pub fn optional(value: Option<f64>) -> Self {
        value.map(Self::Number).unwrap_or(Self::Missing)
    }

    fn rank(&self) -> u8 {
        match self {
            Self::Missing => 0,
            Self::Number(_) => 1,
            Self::Text(_) => 2,
        }
    }
}

impl Eq for SortKey {}

impl PartialOrd for SortKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SortKey {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Number(a), Self::Number(b)) => a.total_cmp(b),
            (Self::Text(a), Self::Text(b)) => a.cmp(b),
            _ => self.rank().cmp(&other.rank()),
        }
    }
}

pub fn sort_rows<T>(rows: &mut [T], sort: Sort, key: impl Fn(&T, usize) -> SortKey) {
    rows.sort_by_cached_key(|row| key(row, sort.column));
    if sort.is_descending {
        rows.reverse();
    }
}

pub fn sort_header(ui: &mut Ui, label: &str, column: SortColumn, sort: &mut Option<Sort>) {
    let p = Palette::current(ui.ctx());
    let active = sort.filter(|s| s.column == column.index);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = GAP_SMALL;
        let title = RichText::new(label.to_uppercase())
            .small()
            .color(if active.is_some() {
                p.text
            } else {
                p.text_secondary
            });
        if clickable(ui, title) {
            *sort = Some(column.toggled(*sort));
        }
        ui.spacing_mut().item_spacing.x = 0.0;
        let is_ascending = active.is_some_and(|s| !s.is_descending);
        let is_descending = active.is_some_and(|s| s.is_descending);
        if clickable(ui, arrow(&p, ARROW_ASCENDING, is_ascending)) {
            *sort = Some(Sort::ascending(column.index));
        }
        if clickable(ui, arrow(&p, ARROW_DESCENDING, is_descending)) {
            *sort = Some(Sort::descending(column.index));
        }
    });
}

fn arrow(p: &Palette, glyph: &str, is_active: bool) -> RichText {
    let color = if is_active { p.accent } else { p.text_muted };
    RichText::new(glyph).small().color(color)
}

fn clickable(ui: &mut Ui, text: RichText) -> bool {
    ui.add(Label::new(text).sense(Sense::click()))
        .on_hover_cursor(CursorIcon::PointingHand)
        .clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(row: &(&str, Option<f64>), column: usize) -> SortKey {
        match column {
            0 => SortKey::text(row.0),
            _ => SortKey::optional(row.1),
        }
    }

    #[test]
    fn sort_rows_descending_puts_missing_values_last() {
        let mut rows = vec![("a", Some(1.0)), ("b", None), ("c", Some(5.0))];
        sort_rows(&mut rows, Sort::descending(1), key);
        assert_eq!(
            rows.iter().map(|r| r.0).collect::<Vec<_>>(),
            ["c", "a", "b"]
        );
    }

    #[test]
    fn sort_rows_ascending_by_text_ignores_case() {
        let mut rows = vec![("b", None), ("A", None), ("c", None)];
        sort_rows(&mut rows, Sort::ascending(0), key);
        assert_eq!(
            rows.iter().map(|r| r.0).collect::<Vec<_>>(),
            ["A", "b", "c"]
        );
    }

    #[test]
    fn toggled_same_column_flips_direction() {
        let column = SortColumn::number(2);
        let first = column.toggled(None);
        assert!(first.is_descending);
        let second = column.toggled(Some(first));
        assert!(!second.is_descending);
        let other = SortColumn::text(1).toggled(Some(second));
        assert_eq!(other, Sort::ascending(1));
    }
}
