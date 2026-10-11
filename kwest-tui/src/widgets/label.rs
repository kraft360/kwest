//! The label, the single drawing primitive: a fixed-size box with one line of text.
//!
//! A label has an outer size (borders included), a foreground color, text attributes such as bold
//! or underline, and an optional border on each side. Its text is centered in the box and truncated
//! to fit by display width, so a wide character such as `漢` counts as two columns. Drawing writes
//! every cell of the box, padding with spaces, so a label can be drawn over old content without
//! clearing it first.
//!
//! Positions saturate at the terminal's edge rather than overflowing, the same rule as adding
//! [`Point`]s. Layouts are checked, so it can't happen today, but a bad origin would otherwise
//! panic in a debug build instead of drawing clipped.

use std::io::Write;

use super::model::{Border, LineWeight, Point, Size};
use crossterm::{
    QueueableCommand,
    cursor::MoveTo,
    style::{Attributes, Color, ContentStyle, Print},
};
use unicode_width::UnicodeWidthChar;

/// Label is used to render a single line of text inside a box of fixed size, optionally with
/// borders. The content is centered both horizontally and vertically and truncated if it does
/// not fit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Label {
    /// Text, drawn on the middle row.
    content: String,
    /// Outer size of the label including borders.
    size: Size,
    /// Color of the text.
    foreground: Color,
    /// Text attributes, such as bold or underline. `Attributes` is a set of flags packed into one
    /// integer, so it is `Copy` and holds any combination.
    attributes: Attributes,
    /// Border on each side, drawn inside `size`.
    border: Border,
}

impl Label {
    /// Text color until another is set: the terminal's own, so it reads on any theme.
    const DEFAULT_FOREGROUND: Color = Color::Reset;
    /// No border until one is set.
    const DEFAULT_BORDER: Border = Border::empty();

    /// Create a new label with the specified content and outer size (borders included).
    #[must_use]
    pub fn new(content: &str, size: Size) -> Self {
        Self {
            content: content.into(),
            size,
            foreground: Self::DEFAULT_FOREGROUND,
            attributes: Attributes::none(),
            border: Self::DEFAULT_BORDER,
        }
    }

    /// Set the text color.
    #[must_use]
    pub fn with_foreground(mut self, foreground: Color) -> Self {
        self.foreground = foreground;
        self
    }

    /// Set the border.
    #[must_use]
    pub fn with_border(mut self, border: Border) -> Self {
        self.border = border;
        self
    }

    /// Replace the text, its color and attributes, and the border color together, so nothing is
    /// left over from before. The size never changes.
    pub fn update(
        &mut self,
        content: &str,
        foreground: Color,
        attributes: Attributes,
        border_color: Color,
    ) {
        self.content = content.into();
        self.foreground = foreground;
        self.attributes = attributes;
        self.update_border_color(border_color);
    }

    /// Replace the text.
    pub fn update_content(&mut self, content: &str) {
        self.content = content.into();
    }

    /// Replace the text color.
    pub fn update_foreground(&mut self, foreground: Color) {
        self.foreground = foreground;
    }

    /// Replace the text attributes.
    pub fn update_attributes(&mut self, attributes: Attributes) {
        self.attributes = attributes;
    }

    /// Recolor the border. Only the sides set by `with_border` are drawn, so the border's shape,
    /// and with it the size of the content area, never changes after construction.
    pub fn update_border_color(&mut self, color: Color) {
        self.border = self.border.with_color(color);
    }

    /// Change the border's line weight. Like the color, it changes only how the existing sides are
    /// drawn, never their number, so the size of the content area stays the same.
    pub fn update_border_weight(&mut self, weight: LineWeight) {
        self.border = self.border.with_weight(weight);
    }

    /// Get the text. Only tests read it back; components set it and draw.
    #[cfg(test)]
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get the text color. Only tests read it back; components set it and draw.
    #[cfg(test)]
    #[must_use]
    pub fn foreground(&self) -> Color {
        self.foreground
    }

    /// Get the text attributes. Only tests read them back; components set them and draw.
    #[cfg(test)]
    #[must_use]
    pub fn attributes(&self) -> Attributes {
        self.attributes
    }

    /// Get the border. Only tests read it back; components set it and draw.
    #[cfg(test)]
    #[must_use]
    pub fn border(&self) -> Border {
        self.border
    }

    /// Get the outer size of the label including borders.
    #[must_use]
    pub fn size(&self) -> Size {
        self.size
    }

    /// Get the size of the area inside the borders.
    fn inner_size(&self) -> Size {
        let border_width = self.border.left_border_width() + self.border.right_border_width();
        let border_height = self.border.top_border_height() + self.border.bottom_border_height();
        Size::new(
            self.size.width.saturating_sub(border_width),
            self.size.height.saturating_sub(border_height),
        )
    }

    /// Does the label have room for its borders?
    fn fits_borders(&self) -> bool {
        let border_width = self.border.left_border_width() + self.border.right_border_width();
        let border_height = self.border.top_border_height() + self.border.bottom_border_height();
        self.size.width >= border_width && self.size.height >= border_height
    }

    /// Draw the label with its top-left corner at `location`.
    /// Every cell of the label is written, so drawing over stale content needs no clearing.
    ///
    /// # Errors
    ///
    /// Will return `std::io::Error` on rendering error.
    pub fn draw(&self, writer: &mut dyn Write, location: Point) -> std::io::Result<()> {
        // Nothing sensible can be drawn if the borders alone do not fit.
        if !self.fits_borders() {
            return Ok(());
        }
        self.draw_top_border(writer, location)?;
        self.draw_content_area(writer, location)?;
        self.draw_bottom_border(writer, location)
    }

    /// Draw the top border and its corners, if there is a top border.
    fn draw_top_border(&self, writer: &mut dyn Write, location: Point) -> std::io::Result<()> {
        let Some(color) = self.border.top else {
            return Ok(());
        };
        writer.queue(MoveTo(location.x, location.y))?;
        if let Some(corner) = self.border.top_left_corner() {
            writer.queue(Print(corner))?;
        }
        writer.queue(Print(
            self.border
                .horizontal_line(usize::from(self.inner_size().width), color),
        ))?;
        if let Some(corner) = self.border.top_right_corner() {
            writer.queue(Print(corner))?;
        }
        Ok(())
    }

    /// Draw every row of the area inside the borders, with the content on the middle row.
    fn draw_content_area(&self, writer: &mut dyn Write, location: Point) -> std::io::Result<()> {
        let inner = self.inner_size();
        let top = location.y.saturating_add(self.border.top_border_height());
        // Middle row; only meaningful when there is at least one row.
        let middle = inner.height.saturating_sub(1) / 2;
        let left_line = self
            .border
            .left
            .map(|color| self.border.vertical_line(color));
        let right_line = self
            .border
            .right
            .map(|color| self.border.vertical_line(color));
        for row in 0..inner.height {
            writer.queue(MoveTo(location.x, top.saturating_add(row)))?;
            if let Some(line) = &left_line {
                writer.queue(Print(line))?;
            }
            if row == middle {
                self.draw_content(writer, inner.width)?;
            } else {
                draw_spaces(writer, inner.width)?;
            }
            if let Some(line) = &right_line {
                writer.queue(Print(line))?;
            }
        }
        Ok(())
    }

    /// Draw the content centered within the given width, padded with spaces on both sides.
    fn draw_content(&self, writer: &mut dyn Write, width: u16) -> std::io::Result<()> {
        let (content, content_width) = fit(&self.content, width);
        // Cannot underflow: `fit` never returns a width larger than requested.
        let left = (width - content_width) / 2;
        let right = width - content_width - left;
        draw_spaces(writer, left)?;
        if !content.is_empty() {
            let style = ContentStyle {
                foreground_color: Some(self.foreground),
                attributes: self.attributes,
                // The remaining fields (background, underline color) stay unset.
                ..ContentStyle::default()
            };
            writer.queue(Print(style.apply(content)))?;
        }
        draw_spaces(writer, right)
    }

    /// Draw the bottom border and its corners, if there is a bottom border.
    fn draw_bottom_border(&self, writer: &mut dyn Write, location: Point) -> std::io::Result<()> {
        let Some(color) = self.border.bottom else {
            return Ok(());
        };
        // Cannot underflow: `draw` checked that the borders fit, so a bottom border means a height
        // of at least 1.
        let y = location.y.saturating_add(self.size.height - 1);
        writer.queue(MoveTo(location.x, y))?;
        if let Some(corner) = self.border.bottom_left_corner() {
            writer.queue(Print(corner))?;
        }
        writer.queue(Print(
            self.border
                .horizontal_line(usize::from(self.inner_size().width), color),
        ))?;
        if let Some(corner) = self.border.bottom_right_corner() {
            writer.queue(Print(corner))?;
        }
        Ok(())
    }
}

/// Draw the given number of spaces, overwriting whatever was there.
fn draw_spaces(writer: &mut dyn Write, count: u16) -> std::io::Result<()> {
    if count > 0 {
        writer.queue(Print(" ".repeat(usize::from(count))))?;
    }
    Ok(())
}

/// Truncate the content to the given display width.
/// Returns the truncated content along with its display width, which never exceeds `width`.
fn fit(content: &str, width: u16) -> (String, u16) {
    let mut fitted = String::new();
    let mut fitted_width: u16 = 0;
    for ch in content.chars() {
        // `width()` is `None` for control characters; they take no space.
        let ch_width = u16::try_from(ch.width().unwrap_or(0)).unwrap_or(u16::MAX);
        match fitted_width.checked_add(ch_width) {
            Some(w) if w <= width => {
                fitted.push(ch);
                fitted_width = w;
            }
            // Stop at the first character that does not fit, so that a wide character at the
            // edge does not let narrower ones after it sneak in out of order.
            _ => break,
        }
    }
    (fitted, fitted_width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::style::{Attribute, Color};

    #[test]
    fn new_label_is_initialized_with_defaults() {
        let content = "Hello World 🖥️";
        let size = Size::new(20, 3);
        let label = Label::new(content, size);
        assert_eq!(label.content(), content);
        assert_eq!(label.foreground(), Label::DEFAULT_FOREGROUND);
        assert!(label.attributes().is_empty());
        assert_eq!(label.border(), Label::DEFAULT_BORDER);
        assert_eq!(label.size(), size);
    }

    #[test]
    fn with_foreground_sets_new_foreground() {
        let color = Color::Blue;
        let label = Label::new("Hello", Size::new(5, 1)).with_foreground(color);
        assert_eq!(label.foreground(), color);
    }

    #[test]
    fn with_border_sets_new_border() {
        let border = Border::all(Color::Green);
        let label = Label::new("Hello", Size::new(7, 3)).with_border(border);
        assert_eq!(label.border(), border);
    }

    #[test]
    fn inner_size_excludes_borders() {
        let label = Label::new("Hello", Size::new(7, 3)).with_border(Border::all(Color::Green));
        assert_eq!(label.inner_size(), Size::new(5, 1));
    }

    #[test]
    fn update_content_replaces_content() {
        let mut label = Label::new("Hello", Size::new(7, 1));
        label.update_content("Namaste");
        assert_eq!(label.content(), "Namaste");
    }

    #[test]
    fn update_foreground_replaces_foreground() {
        let mut label = Label::new("Hello", Size::new(5, 1)).with_foreground(Color::Green);
        label.update_foreground(Color::Red);
        assert_eq!(label.foreground(), Color::Red);
    }

    #[test]
    fn update_border_color_recolors_the_border() {
        let mut label = Label::new("Hello", Size::new(7, 3)).with_border(Border::all(Color::Green));
        label.update_border_color(Color::Blue);
        assert_eq!(label.border(), Border::all(Color::Blue));
    }

    #[test]
    fn update_border_color_keeps_the_sides() {
        let mut label =
            Label::new("Hello", Size::new(3, 1)).with_border(Border::left(Color::Green));
        let inner_size = label.inner_size();
        label.update_border_color(Color::Blue);
        assert_eq!(label.border(), Border::left(Color::Blue));
        assert_eq!(label.inner_size(), inner_size);
    }

    #[test]
    fn update_border_color_adds_no_border() {
        let mut label = Label::new("Hello", Size::new(5, 1));
        label.update_border_color(Color::Blue);
        assert_eq!(label.border(), Border::empty());
    }

    #[test]
    fn update_replaces_all_state() {
        let mut label = Label::new("Hello", Size::new(9, 3)).with_border(Border::all(Color::Red));
        let attributes = Attributes::from(Attribute::Underlined);
        label.update("Namaste", Color::Green, attributes, Color::Cyan);
        assert_eq!(label.content(), "Namaste");
        assert_eq!(label.foreground(), Color::Green);
        assert_eq!(label.attributes(), attributes);
        assert_eq!(label.border(), Border::all(Color::Cyan));
    }

    #[test]
    fn fit_keeps_content_that_fits() {
        assert_eq!(fit("Hello", 5), ("Hello".into(), 5));
        assert_eq!(fit("Hi", 5), ("Hi".into(), 2));
    }

    #[test]
    fn fit_truncates_by_display_width() {
        assert_eq!(fit("Hello World", 5), ("Hello".into(), 5));
        // Each of these characters is two columns wide.
        assert_eq!(fit("日本語", 5), ("日本".into(), 4));
        assert_eq!(fit("日本語", 1), (String::new(), 0));
    }

    #[test]
    fn label_with_border_draws_cleanly() {
        let label = Label::new("Hello World", Size::new(17, 5))
            .with_foreground(Color::White)
            .with_border(Border::all(Color::DarkGrey));
        let mut writer = Vec::new();
        label.draw(&mut writer, Point::ZERO).unwrap();
        let mut expected = String::new();
        // Top border.
        expected.push_str("\u{1b}[1;1H\u{1b}[38;5;8m┌\u{1b}[39m");
        expected.push_str("\u{1b}[38;5;8m───────────────\u{1b}[39m");
        expected.push_str("\u{1b}[38;5;8m┐\u{1b}[39m");
        // Blank row above the content.
        expected.push_str("\u{1b}[2;1H\u{1b}[38;5;8m│\u{1b}[39m");
        expected.push_str("               ");
        expected.push_str("\u{1b}[38;5;8m│\u{1b}[39m");
        // Content row, centered with two spaces on each side.
        expected.push_str("\u{1b}[3;1H\u{1b}[38;5;8m│\u{1b}[39m");
        expected.push_str("  \u{1b}[38;5;15mHello World\u{1b}[39m  ");
        expected.push_str("\u{1b}[38;5;8m│\u{1b}[39m");
        // Blank row below the content.
        expected.push_str("\u{1b}[4;1H\u{1b}[38;5;8m│\u{1b}[39m");
        expected.push_str("               ");
        expected.push_str("\u{1b}[38;5;8m│\u{1b}[39m");
        // Bottom border.
        expected.push_str("\u{1b}[5;1H\u{1b}[38;5;8m└\u{1b}[39m");
        expected.push_str("\u{1b}[38;5;8m───────────────\u{1b}[39m");
        expected.push_str("\u{1b}[38;5;8m┘\u{1b}[39m");
        assert_eq!(std::str::from_utf8(&writer).unwrap(), expected);
    }

    #[test]
    fn heavy_border_draws_heavy_lines() {
        let mut label = Label::new("A", Size::new(3, 3)).with_border(Border::all(Color::DarkGrey));
        label.update_border_weight(LineWeight::Heavy);
        let mut writer = Vec::new();
        label.draw(&mut writer, Point::ZERO).unwrap();
        let mut expected = String::new();
        expected.push_str("\u{1b}[1;1H\u{1b}[38;5;8m┏\u{1b}[39m");
        expected.push_str("\u{1b}[38;5;8m━\u{1b}[39m");
        expected.push_str("\u{1b}[38;5;8m┓\u{1b}[39m");
        expected.push_str("\u{1b}[2;1H\u{1b}[38;5;8m┃\u{1b}[39m");
        expected.push_str("\u{1b}[39mA\u{1b}[39m");
        expected.push_str("\u{1b}[38;5;8m┃\u{1b}[39m");
        expected.push_str("\u{1b}[3;1H\u{1b}[38;5;8m┗\u{1b}[39m");
        expected.push_str("\u{1b}[38;5;8m━\u{1b}[39m");
        expected.push_str("\u{1b}[38;5;8m┛\u{1b}[39m");
        assert_eq!(std::str::from_utf8(&writer).unwrap(), expected);
    }

    #[test]
    fn label_without_border_centers_content() {
        let label = Label::new("Hi", Size::new(7, 1)).with_foreground(Color::White);
        let mut writer = Vec::new();
        label.draw(&mut writer, Point::new(3, 4)).unwrap();
        let expected = "\u{1b}[5;4H  \u{1b}[38;5;15mHi\u{1b}[39m   ";
        assert_eq!(std::str::from_utf8(&writer).unwrap(), expected);
    }

    #[test]
    fn label_draws_content_with_its_attributes() {
        let mut label = Label::new("Hi", Size::new(2, 1)).with_foreground(Color::White);
        let attributes = Attributes::none()
            .with(Attribute::Bold)
            .with(Attribute::Underlined);
        label.update_attributes(attributes);
        assert_eq!(label.attributes(), attributes);
        let mut writer = Vec::new();
        label.draw(&mut writer, Point::ZERO).unwrap();
        // Bold is `ESC[1m` and underline `ESC[4m`. The text ends with a full reset, `ESC[0m`,
        // which also turns the attributes off, instead of resetting only the color.
        let expected = "\u{1b}[1;1H\u{1b}[38;5;15m\u{1b}[1m\u{1b}[4mHi\u{1b}[0m";
        assert_eq!(std::str::from_utf8(&writer).unwrap(), expected);
    }

    #[test]
    fn label_truncates_content_that_is_too_wide() {
        let label = Label::new("Hello World", Size::new(5, 1)).with_foreground(Color::White);
        let mut writer = Vec::new();
        label.draw(&mut writer, Point::ZERO).unwrap();
        let expected = "\u{1b}[1;1H\u{1b}[38;5;15mHello\u{1b}[39m";
        assert_eq!(std::str::from_utf8(&writer).unwrap(), expected);
    }

    #[test]
    fn label_fills_blank_content_with_spaces() {
        let label = Label::new("", Size::new(3, 2));
        let mut writer = Vec::new();
        label.draw(&mut writer, Point::ZERO).unwrap();
        let expected = "\u{1b}[1;1H   \u{1b}[2;1H   ";
        assert_eq!(std::str::from_utf8(&writer).unwrap(), expected);
    }

    #[test]
    fn label_too_small_for_its_borders_draws_nothing() {
        let label = Label::new("Hello", Size::new(1, 1)).with_border(Border::all(Color::Red));
        let mut writer = Vec::new();
        label.draw(&mut writer, Point::ZERO).unwrap();
        assert_eq!(writer, b"");
    }
}
