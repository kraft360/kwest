//! Layouts: work out where each child goes from the children's sizes.
//!
//! A layout only does arithmetic and draws nothing. It returns a [`Geometry`]: the total size, plus
//! one location per child, relative to the container's origin and in the order the sizes were
//! given. A container matches each child to its location by that order, so widgets need no IDs. All
//! arithmetic is checked: a layout too big for `u16` returns [`WidgetError::LayoutOverflow`]
//! instead of wrapping around.
//!
//! - [`FlowLayout`] places children in a single row or column.
//! - [`ColumnLayout`] stacks children vertically and centers them, optionally within a given size.
//!   Grids and the whole screen use it.

use super::model::{Geometry, Point, Size};
use crate::errors::WidgetError;

/// Direction in which a flow layout places its widgets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Axis {
    /// Left to right.
    Horizontal,
    /// Top to bottom. Nothing lays out a vertical flow yet, but it is kept, tested and ready.
    /// `expect` (rather than `allow`) warns once it is used, so this attribute can't go stale.
    #[cfg_attr(not(test), expect(dead_code, reason = "kept for future layouts"))]
    Vertical,
}

/// Add two coordinates, returning `WidgetError::LayoutOverflow` instead of overflowing.
fn add(a: u16, b: u16) -> Result<u16, WidgetError> {
    a.checked_add(b).ok_or(WidgetError::LayoutOverflow {
        maximum_size: u16::MAX,
    })
}

/// Places widgets one after another along an axis, aligned to the start of the other axis.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FlowLayout {
    /// Direction the widgets are placed in.
    axis: Axis,
    /// Cells of space between neighboring widgets.
    spacing: u16,
}

impl FlowLayout {
    /// Create a flow layout along the axis, with the given spacing between widgets.
    #[must_use]
    pub const fn new(axis: Axis, spacing: u16) -> Self {
        Self { axis, spacing }
    }

    /// Compute the geometry for widgets of the given sizes.
    /// Spacing goes only between widgets, not before the first or after the last. No widgets give
    /// a size of 0 × 0.
    ///
    /// # Errors
    ///
    /// Will return `WidgetError::LayoutOverflow` if the total along the layout axis
    /// exceeds `u16::MAX`.
    pub fn compute(self, widget_sizes: &[Size]) -> Result<Geometry, WidgetError> {
        let mut locations = Vec::with_capacity(widget_sizes.len());
        let mut width: u16 = 0;
        let mut height: u16 = 0;
        match self.axis {
            Axis::Horizontal => {
                let mut gap = 0;
                for size in widget_sizes {
                    // Add 0 before the first widget, spacing before the rest.
                    width = add(width, gap)?;
                    locations.push(Point::new(width, 0));
                    width = add(width, size.width)?;
                    height = height.max(size.height);
                    gap = self.spacing;
                }
            }
            Axis::Vertical => {
                let mut gap = 0;
                for size in widget_sizes {
                    // Add 0 before the first widget, spacing before the rest.
                    height = add(height, gap)?;
                    locations.push(Point::new(0, height));
                    width = width.max(size.width);
                    height = add(height, size.height)?;
                    gap = self.spacing;
                }
            }
        }
        Ok(Geometry {
            size: Size::new(width, height),
            locations,
        })
    }
}

/// Stacks widgets vertically, each centered horizontally.
/// Given a size (see `with_size`), it also centers the whole stack within that size.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ColumnLayout {
    /// Cells of space between neighboring widgets.
    spacing: u16,
    /// Size to center the stack in. Without one, the layout is exactly as big as the widgets
    /// need.
    size: Option<Size>,
}

impl ColumnLayout {
    /// Create a column layout with the given spacing, sized to fit its widgets exactly.
    #[must_use]
    pub const fn new(spacing: u16) -> Self {
        Self {
            spacing,
            size: None,
        }
    }

    /// Center the stack within the given size. If the widgets need more room in either
    /// direction, the layout grows to fit them instead.
    #[must_use]
    pub const fn with_size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }

    /// Compute the geometry for widgets of the given sizes.
    ///
    /// # Errors
    ///
    /// Will return `WidgetError::LayoutOverflow` if the minimum height required by the widgets
    /// exceeds `u16::MAX`.
    pub fn compute(self, widget_sizes: &[Size]) -> Result<Geometry, WidgetError> {
        // First pass: the smallest size that holds every widget.
        let mut min_width: u16 = 0;
        let mut min_height: u16 = 0;
        let mut gap = 0;
        for size in widget_sizes {
            min_width = min_width.max(size.width);
            min_height = add(add(min_height, size.height)?, gap)?;
            // The next widget (if any) is preceded by `spacing`.
            gap = self.spacing;
        }
        // The layout takes the requested size, grown where the widgets need more.
        let requested_size = self.size.unwrap_or_default();
        let width = requested_size.width.max(min_width);
        let height = requested_size.height.max(min_height);
        let mut locations = Vec::with_capacity(widget_sizes.len());
        // Second pass: place the widgets, starting low enough to center the stack.
        //
        // Cannot overflow: the loop below adds exactly `min_height` in total, starting from
        // `(height - min_height) / 2`, so `y` ends at most at `height`. `add` is kept so that
        // the arithmetic stays checked (it satisfies overflow lints, and a future mismatch
        // with the first pass would return an error instead of wrapping).
        //
        // Cannot underflow: `height` is `max(requested, min_height)`.
        let mut y = (height - min_height) / 2;
        let mut gap = 0;
        for size in widget_sizes {
            // Add 0 before the first widget, spacing before the rest.
            y = add(y, gap)?;
            // Cannot underflow: `width >= min_width >= size.width`.
            locations.push(Point::new((width - size.width) / 2, y));
            y = add(y, size.height)?;
            gap = self.spacing;
        }
        Ok(Geometry {
            size: Size::new(width, height),
            locations,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Error every overflowing layout returns.
    const OVERFLOW: WidgetError = WidgetError::LayoutOverflow {
        maximum_size: u16::MAX,
    };

    /// Shorthand for building sizes.
    fn sizes(dimensions: &[(u16, u16)]) -> Vec<Size> {
        dimensions
            .iter()
            .map(|&(width, height)| Size::new(width, height))
            .collect()
    }

    #[test]
    fn horizontal_flow_places_widgets_left_to_right() {
        let geometry = FlowLayout::new(Axis::Horizontal, 2)
            .compute(&sizes(&[(3, 1), (5, 2)]))
            .unwrap();
        // 3 + 2 spacing + 5 wide; as tall as the tallest widget, which are aligned to the top.
        assert_eq!(geometry.size, Size::new(10, 2));
        assert_eq!(geometry.locations, [Point::new(0, 0), Point::new(5, 0)]);
    }

    #[test]
    fn vertical_flow_places_widgets_top_to_bottom() {
        let geometry = FlowLayout::new(Axis::Vertical, 1)
            .compute(&sizes(&[(3, 1), (5, 2)]))
            .unwrap();
        // As wide as the widest widget, which are aligned to the left; 1 + 1 spacing + 2 tall.
        assert_eq!(geometry.size, Size::new(5, 4));
        assert_eq!(geometry.locations, [Point::new(0, 0), Point::new(0, 2)]);
    }

    #[test]
    fn flow_adds_no_spacing_around_a_single_widget() {
        let geometry = FlowLayout::new(Axis::Horizontal, 4)
            .compute(&sizes(&[(3, 1)]))
            .unwrap();
        assert_eq!(geometry.size, Size::new(3, 1));
        assert_eq!(geometry.locations, [Point::ZERO]);
    }

    #[test]
    fn flow_without_widgets_is_empty() {
        for axis in [Axis::Horizontal, Axis::Vertical] {
            let geometry = FlowLayout::new(axis, 1).compute(&[]).unwrap();
            assert_eq!(geometry.size, Size::new(0, 0));
            assert_eq!(geometry.locations, Vec::<Point>::new());
        }
    }

    #[test]
    fn flow_overflow_is_an_error() {
        let layout = FlowLayout::new(Axis::Horizontal, 0);
        assert_eq!(
            layout.compute(&sizes(&[(u16::MAX, 1), (1, 1)])),
            Err(OVERFLOW)
        );
    }

    #[test]
    fn flow_spacing_counts_toward_overflow() {
        // The widgets alone fit exactly; the spacing between them does not.
        let layout = FlowLayout::new(Axis::Vertical, 1);
        assert_eq!(
            layout.compute(&sizes(&[(1, u16::MAX - 1), (1, 1)])),
            Err(OVERFLOW)
        );
    }

    #[test]
    fn column_without_size_fits_its_widgets() {
        let geometry = ColumnLayout::new(1)
            .compute(&sizes(&[(4, 1), (2, 1)]))
            .unwrap();
        assert_eq!(geometry.size, Size::new(4, 3));
        // The narrower widget is centered under the wider one.
        assert_eq!(geometry.locations, [Point::new(0, 0), Point::new(1, 2)]);
    }

    #[test]
    fn column_centering_rounds_toward_the_top_left() {
        // 3 columns and 3 rows to spare, split 1 before and 2 after.
        let geometry = ColumnLayout::new(0)
            .with_size(Size::new(4, 4))
            .compute(&sizes(&[(1, 1)]))
            .unwrap();
        assert_eq!(geometry.locations, [Point::new(1, 1)]);
    }

    #[test]
    fn column_with_size_centers_the_stack() {
        let geometry = ColumnLayout::new(1)
            .with_size(Size::new(10, 9))
            .compute(&sizes(&[(4, 1), (2, 1)]))
            .unwrap();
        assert_eq!(geometry.size, Size::new(10, 9));
        // The stack is 3 tall, so it starts (9 - 3) / 2 = 3 rows down.
        assert_eq!(geometry.locations, [Point::new(3, 3), Point::new(4, 5)]);
    }

    #[test]
    fn column_grows_where_the_size_is_too_small() {
        // Wide enough, but too short: only the height grows.
        let geometry = ColumnLayout::new(1)
            .with_size(Size::new(10, 1))
            .compute(&sizes(&[(4, 1), (2, 1)]))
            .unwrap();
        assert_eq!(geometry.size, Size::new(10, 3));
        assert_eq!(geometry.locations, [Point::new(3, 0), Point::new(4, 2)]);
    }

    #[test]
    fn column_without_widgets_takes_the_requested_size() {
        let empty = ColumnLayout::new(1).compute(&[]).unwrap();
        assert_eq!(empty.size, Size::new(0, 0));
        let sized = ColumnLayout::new(1)
            .with_size(Size::new(5, 3))
            .compute(&[])
            .unwrap();
        assert_eq!(sized.size, Size::new(5, 3));
        assert_eq!(sized.locations, Vec::<Point>::new());
    }

    #[test]
    fn column_overflow_is_an_error() {
        let layout = ColumnLayout::new(0);
        assert_eq!(
            layout.compute(&sizes(&[(1, u16::MAX), (1, 1)])),
            Err(OVERFLOW)
        );
    }

    #[test]
    fn column_spacing_counts_toward_overflow() {
        let layout = ColumnLayout::new(1);
        assert_eq!(
            layout.compute(&sizes(&[(1, u16::MAX - 1), (1, 1)])),
            Err(OVERFLOW)
        );
    }
}
