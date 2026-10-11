//! Geometry and styling types shared by the widgets and the components: points, sizes, borders
//! and layout results.
//!
//! Coordinates and sizes are `u16`, the type crossterm uses for terminal cells. Adding two
//! [`Point`]s saturates instead of overflowing; locations come from checked layouts, so the limit
//! isn't reached in practice. A [`Border`] has an optional color for each side and a
//! [`LineWeight`], and provides the line and corner characters. A [`Geometry`] is what a layout
//! computes.

use std::ops::Add;

use crossterm::style::{Color, StyledContent, Stylize};

/// Position in terminal cells, on the screen or relative to a container's top-left corner.
/// The default is [`Point::ZERO`]; prefer the constant where the origin is meant.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Point {
    /// X coordinate.
    pub x: u16,
    /// Y coordinate.
    pub y: u16,
}

/// Offset a point by another, typically a container origin by a child's relative location.
/// Coordinates saturate at `u16::MAX` instead of overflowing.
impl Add for Point {
    type Output = Point;

    fn add(self, rhs: Point) -> Point {
        Point::new(self.x.saturating_add(rhs.x), self.y.saturating_add(rhs.y))
    }
}

impl Point {
    /// The top-left corner.
    pub const ZERO: Point = Point::new(0, 0);

    /// Create a point with the specified coordinates.
    #[must_use]
    pub const fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

/// Width and height in terminal cells.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Size {
    /// Width.
    pub width: u16,
    /// Height.
    pub height: u16,
}

impl Size {
    /// Create a size with the specified dimensions.
    #[must_use]
    pub const fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
}

/// Thickness of a border's lines. It is a style, like color: changing it changes the characters
/// drawn, never which sides exist, so a label's size stays the same.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LineWeight {
    /// Thin lines: `┌─┐│└┘`.
    #[default]
    Light,
    /// Thick lines: `┏━┓┃┗┛`.
    Heavy,
}

/// Characters for drawing a border of one weight.
struct Glyphs {
    /// Horizontal line.
    horizontal: &'static str,
    /// Vertical line.
    vertical: &'static str,
    /// Top-left corner.
    top_left: &'static str,
    /// Top-right corner.
    top_right: &'static str,
    /// Bottom-left corner.
    bottom_left: &'static str,
    /// Bottom-right corner.
    bottom_right: &'static str,
}

impl LineWeight {
    /// Get the characters for this weight.
    const fn glyphs(self) -> Glyphs {
        match self {
            Self::Light => Glyphs {
                horizontal: "─",
                vertical: "│",
                top_left: "┌",
                top_right: "┐",
                bottom_left: "└",
                bottom_right: "┘",
            },
            Self::Heavy => Glyphs {
                horizontal: "━",
                vertical: "┃",
                top_left: "┏",
                top_right: "┓",
                bottom_left: "┗",
                bottom_right: "┛",
            },
        }
    }
}

/// Border around a label: an optional color for each side, and one line weight for all of them.
/// A side that is `None` has no border and takes no space.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Border {
    /// Left border.
    pub left: Option<Color>,
    /// Top border.
    pub top: Option<Color>,
    /// Right border.
    pub right: Option<Color>,
    /// Bottom border.
    pub bottom: Option<Color>,
    /// Thickness of the lines.
    pub weight: LineWeight,
}

impl Border {
    /// Create a border with no sides.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            left: None,
            top: None,
            right: None,
            bottom: None,
            weight: LineWeight::Light,
        }
    }

    /// Create a border with the same color on all sides.
    #[must_use]
    pub const fn all(color: Color) -> Self {
        Self {
            left: Some(color),
            top: Some(color),
            right: Some(color),
            bottom: Some(color),
            weight: LineWeight::Light,
        }
    }

    /// Create a border on the left side only, in the given color. In a row of cells, each cell's
    /// left border separates it from the one before.
    #[must_use]
    pub const fn left(color: Color) -> Self {
        Self {
            left: Some(color),
            ..Self::empty()
        }
    }

    /// Create a border on the bottom side only, in the given color: an underline.
    #[must_use]
    pub const fn bottom(color: Color) -> Self {
        Self {
            bottom: Some(color),
            ..Self::empty()
        }
    }

    /// Get the same sides in another color. Sides without a border stay without one, so the
    /// shape, and the space the border takes, never change. The weight is kept.
    #[must_use]
    pub fn with_color(self, color: Color) -> Self {
        let recolor = |side: Option<Color>| side.map(|_| color);
        Self {
            left: recolor(self.left),
            top: recolor(self.top),
            right: recolor(self.right),
            bottom: recolor(self.bottom),
            weight: self.weight,
        }
    }

    /// Get the same sides and colors drawn with another line weight.
    #[must_use]
    pub fn with_weight(self, weight: LineWeight) -> Self {
        Self { weight, ..self }
    }

    /// Get the width of the left border: 1 if there is one, otherwise 0.
    #[must_use]
    pub fn left_border_width(&self) -> u16 {
        u16::from(self.left.is_some())
    }

    /// Get the height of the top border: 1 if there is one, otherwise 0.
    #[must_use]
    pub fn top_border_height(&self) -> u16 {
        u16::from(self.top.is_some())
    }

    /// Get the width of the right border: 1 if there is one, otherwise 0.
    #[must_use]
    pub fn right_border_width(&self) -> u16 {
        u16::from(self.right.is_some())
    }

    /// Get the height of the bottom border: 1 if there is one, otherwise 0.
    #[must_use]
    pub fn bottom_border_height(&self) -> u16 {
        u16::from(self.bottom.is_some())
    }

    /// Get the top-left corner, in the left border's color.
    /// There is a corner only where two borders meet.
    #[must_use]
    pub fn top_left_corner(&self) -> Option<StyledContent<&str>> {
        if let Some(color) = self.left
            && self.top.is_some()
        {
            Some(self.weight.glyphs().top_left.with(color))
        } else {
            None
        }
    }

    /// Get the top-right corner, in the right border's color.
    /// There is a corner only where two borders meet.
    #[must_use]
    pub fn top_right_corner(&self) -> Option<StyledContent<&str>> {
        if let Some(color) = self.right
            && self.top.is_some()
        {
            Some(self.weight.glyphs().top_right.with(color))
        } else {
            None
        }
    }

    /// Get the bottom-left corner, in the left border's color.
    /// There is a corner only where two borders meet.
    #[must_use]
    pub fn bottom_left_corner(&self) -> Option<StyledContent<&str>> {
        if let Some(color) = self.left
            && self.bottom.is_some()
        {
            Some(self.weight.glyphs().bottom_left.with(color))
        } else {
            None
        }
    }

    /// Get the bottom-right corner, in the right border's color.
    /// There is a corner only where two borders meet.
    #[must_use]
    pub fn bottom_right_corner(&self) -> Option<StyledContent<&str>> {
        if let Some(color) = self.right
            && self.bottom.is_some()
        {
            Some(self.weight.glyphs().bottom_right.with(color))
        } else {
            None
        }
    }

    /// Get a horizontal border line of the given width and color, in this border's weight.
    #[must_use]
    pub fn horizontal_line(&self, width: usize, color: Color) -> StyledContent<String> {
        self.weight.glyphs().horizontal.repeat(width).with(color)
    }

    /// Get a single vertical border character in the given color, in this border's weight.
    #[must_use]
    pub fn vertical_line(&self, color: Color) -> StyledContent<&'static str> {
        self.weight.glyphs().vertical.with(color)
    }
}

/// Result of a layout: the total size, and where each child goes.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Geometry {
    /// Total size, the smallest box that holds every child.
    pub size: Size,
    /// Location of each child relative to the container's top-left corner, in the order the
    /// sizes were given to the layout.
    pub locations: Vec<Point>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::style::Color::{Green, Red};

    #[test]
    fn with_color_keeps_the_weight() {
        let border = Border::all(Green)
            .with_weight(LineWeight::Heavy)
            .with_color(Red);
        assert_eq!(border.weight, LineWeight::Heavy);
        assert_eq!(border.left, Some(Red));
    }

    #[test]
    fn with_weight_keeps_the_sides_and_colors() {
        let border = Border::left(Green).with_weight(LineWeight::Heavy);
        assert_eq!(
            border,
            Border {
                weight: LineWeight::Heavy,
                ..Border::left(Green)
            }
        );
        assert_eq!(border.top, None);
    }

    #[test]
    fn heavy_border_uses_heavy_characters() {
        let border = Border::all(Green).with_weight(LineWeight::Heavy);
        assert_eq!(border.top_left_corner().unwrap().content(), &"┏");
        assert_eq!(border.vertical_line(Green).content(), &"┃");
        assert_eq!(border.horizontal_line(2, Green).content(), "━━");
    }

    #[test]
    fn zero_point_has_zero_coordinates() {
        assert_eq!((Point::ZERO.x, Point::ZERO.y), (0, 0));
    }

    #[test]
    fn default_point_is_zero() {
        assert_eq!(Point::default(), Point::ZERO);
    }

    #[test]
    fn new_point_sets_coordinates() {
        let point = Point::new(10, 15);
        assert_eq!((point.x, point.y), (10, 15));
    }

    #[test]
    fn adding_points_offsets_coordinates() {
        assert_eq!(Point::new(2, 3) + Point::new(10, 20), Point::new(12, 23));
    }

    #[test]
    fn adding_points_saturates() {
        assert_eq!(
            Point::new(u16::MAX, 1) + Point::new(1, 1),
            Point::new(u16::MAX, 2)
        );
    }

    #[test]
    fn new_size_sets_dimensions() {
        let size = Size::new(10, 15);
        assert_eq!((size.width, size.height), (10, 15));
    }

    #[test]
    fn empty_border_has_no_colors_set() {
        let border = Border::empty();
        assert_eq!(
            (border.left, border.top, border.right, border.bottom),
            (None, None, None, None)
        );
    }

    #[test]
    fn single_color_border_has_same_color_set() {
        let border = Border::all(Green);
        assert_eq!(
            (border.left, border.top, border.right, border.bottom),
            (Some(Green), Some(Green), Some(Green), Some(Green))
        );
    }

    #[test]
    fn left_border_has_only_left_side_color_set() {
        let border = Border::left(Green);
        assert_eq!(
            (border.left, border.top, border.right, border.bottom),
            (Some(Green), None, None, None)
        );
    }

    #[test]
    fn bottom_border_has_only_bottom_color_set() {
        let border = Border::bottom(Green);
        assert_eq!(
            (border.left, border.top, border.right, border.bottom),
            (None, None, None, Some(Green))
        );
    }

    #[test]
    fn with_color_keeps_the_side() {
        assert_eq!(Border::left(Green).with_color(Red), Border::left(Red));
        assert_eq!(Border::all(Green).with_color(Red), Border::all(Red));
        assert_eq!(Border::empty().with_color(Red), Border::empty());
    }
}
