//! Containers that lay labels out in rows and grids.
//!
//! They know nothing about the game: components such as the board and the keyboard wrap them and
//! decide what each label shows. A container computes its layout once, when it is built. Its labels
//! can be changed afterwards through the `_mut` accessors, but only their content and style
//! (colors, text attributes, border weight): if a label's size changed, the stored layout would be
//! wrong.

use std::io::Write;

use super::label::Label;
use super::layout::{Axis, ColumnLayout, FlowLayout};
use super::model::{Geometry, Point, Size};
use crate::errors::WidgetError;

/// A single row of labels laid out horizontally.
/// Geometry is computed once at construction; label sizes are fixed, so it never goes stale.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GridRow {
    /// Labels in left to right order.
    labels: Vec<Label>,
    /// Geometry of the row; `locations[i]` belongs to `labels[i]`.
    geometry: Geometry,
}

impl GridRow {
    /// Create a row of labels with the given spacing between them.
    ///
    /// # Errors
    ///
    /// Will return `WidgetError::LayoutOverflow` if the row is wider than `u16::MAX`.
    pub fn new(labels: Vec<Label>, spacing: u16) -> Result<Self, WidgetError> {
        let sizes = labels.iter().map(Label::size).collect::<Vec<_>>();
        let geometry = FlowLayout::new(Axis::Horizontal, spacing).compute(&sizes)?;
        Ok(Self { labels, geometry })
    }

    /// Get the size of the row.
    #[must_use]
    pub fn size(&self) -> Size {
        self.geometry.size
    }

    /// Number of labels in the row.
    #[must_use]
    pub fn len(&self) -> usize {
        self.labels.len()
    }

    /// Get the label at the given position. Only tests read labels back.
    #[cfg(test)]
    #[must_use]
    pub fn label(&self, index: usize) -> Option<&Label> {
        self.labels.get(index)
    }

    /// Get the label at the given position for updating.
    /// Updates must not change the label size, as the geometry is not recomputed.
    pub fn label_mut(&mut self, index: usize) -> Option<&mut Label> {
        self.labels.get_mut(index)
    }

    /// Iterate over the labels for updating.
    /// Updates must not change the label size, as the geometry is not recomputed.
    pub fn labels_mut(&mut self) -> impl Iterator<Item = &mut Label> {
        self.labels.iter_mut()
    }

    /// Draw the row with its top-left corner at the given origin.
    ///
    /// # Errors
    ///
    /// Will return `std::io::Error` on rendering error.
    pub fn draw(&self, writer: &mut dyn Write, origin: Point) -> std::io::Result<()> {
        for (label, location) in self.labels.iter().zip(&self.geometry.locations) {
            label.draw(writer, origin + *location)?;
        }
        Ok(())
    }
}

/// Rows of labels stacked vertically, each row centered horizontally.
/// Rows may have different lengths, as on a keyboard.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Grid {
    /// Rows in top to bottom order.
    rows: Vec<GridRow>,
    /// Geometry of the grid; `locations[i]` belongs to `rows[i]`.
    geometry: Geometry,
}

impl Grid {
    /// Create a grid from the rows with the given spacing between them.
    ///
    /// # Errors
    ///
    /// Will return `WidgetError::LayoutOverflow` if the grid is taller than `u16::MAX`.
    pub fn new(rows: Vec<GridRow>, spacing: u16) -> Result<Self, WidgetError> {
        let sizes = rows.iter().map(GridRow::size).collect::<Vec<_>>();
        let geometry = ColumnLayout::new(spacing).compute(&sizes)?;
        Ok(Self { rows, geometry })
    }

    /// Get the size of the grid.
    #[must_use]
    pub fn size(&self) -> Size {
        self.geometry.size
    }

    /// Number of rows in the grid.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Get the row at the given position.
    #[must_use]
    pub fn row(&self, index: usize) -> Option<&GridRow> {
        self.rows.get(index)
    }

    /// Get the row at the given position for updating.
    pub fn row_mut(&mut self, index: usize) -> Option<&mut GridRow> {
        self.rows.get_mut(index)
    }

    /// Iterate over the rows for updating.
    pub fn rows_mut(&mut self) -> impl Iterator<Item = &mut GridRow> {
        self.rows.iter_mut()
    }

    /// Get the label at the given row and column. Only tests read labels back.
    #[cfg(test)]
    #[must_use]
    pub fn label(&self, row: usize, column: usize) -> Option<&Label> {
        self.row(row)?.label(column)
    }

    /// Get the label at the given row and column for updating.
    pub fn label_mut(&mut self, row: usize, column: usize) -> Option<&mut Label> {
        self.row_mut(row)?.label_mut(column)
    }

    /// Draw the grid with its top-left corner at the given origin.
    ///
    /// # Errors
    ///
    /// Will return `std::io::Error` on rendering error.
    pub fn draw(&self, writer: &mut dyn Write, origin: Point) -> std::io::Result<()> {
        for (row, location) in self.rows.iter().zip(&self.geometry.locations) {
            row.draw(writer, origin + *location)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a row of single-letter labels of the given size.
    fn row(letters: &str, size: Size, spacing: u16) -> GridRow {
        let labels = letters
            .chars()
            .map(|ch| Label::new(&ch.to_string(), size))
            .collect();
        GridRow::new(labels, spacing).unwrap()
    }

    #[test]
    fn row_size_includes_spacing() {
        let row = row("ABC", Size::new(3, 2), 1);
        assert_eq!(row.size(), Size::new(11, 2));
        assert_eq!(row.len(), 3);
    }

    #[test]
    fn empty_row_has_zero_size() {
        let row = GridRow::new(Vec::new(), 1).unwrap();
        assert_eq!(row.size(), Size::new(0, 0));
        assert_eq!(row.len(), 0);
    }

    #[test]
    fn row_draws_labels_at_offset_locations() {
        let row = row("AB", Size::new(1, 1), 2);
        let mut writer = Vec::new();
        row.draw(&mut writer, Point::new(10, 5)).unwrap();
        let output = std::str::from_utf8(&writer).unwrap();
        // Labels at x = 10 and x = 13 (1 wide + 2 spacing), y = 5; terminal is 1-based.
        assert!(output.contains("\u{1b}[6;11H"));
        assert!(output.contains("\u{1b}[6;14H"));
    }

    #[test]
    fn grid_with_equal_rows_stacks_them() {
        let size = Size::new(3, 3);
        let grid = Grid::new(vec![row("AB", size, 1), row("CD", size, 1)], 1).unwrap();
        assert_eq!(grid.size(), Size::new(7, 7));
        assert_eq!(grid.len(), 2);
        assert_eq!(grid.label(1, 0).unwrap().content(), "C");
    }

    #[test]
    fn grid_with_ragged_rows_centers_shorter_rows() {
        let size = Size::new(1, 1);
        let grid = Grid::new(vec![row("ABCDE", size, 1), row("FGH", size, 1)], 0).unwrap();
        // Widest row is 9 wide; the 5-wide row is offset by 2.
        assert_eq!(grid.size(), Size::new(9, 2));
        let mut writer = Vec::new();
        grid.draw(&mut writer, Point::ZERO).unwrap();
        let output = std::str::from_utf8(&writer).unwrap();
        // "F" is the first label of the second row at (2, 1).
        assert!(output.contains("\u{1b}[2;3H"));
    }

    #[test]
    fn label_mut_updates_a_cell() {
        let size = Size::new(1, 1);
        let mut grid = Grid::new(vec![row("AB", size, 0)], 0).unwrap();
        grid.label_mut(0, 1).unwrap().update_content("Z");
        assert_eq!(grid.label(0, 1).unwrap().content(), "Z");
        assert!(grid.label_mut(1, 0).is_none());
    }
}
