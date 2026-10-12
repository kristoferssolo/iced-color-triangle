//! A hue ring and triangle color selector for Iced.
//!
//! Retain [`Selection`] in application state and replace it on each change.
//! This preserves the chosen hue when the color becomes black, white, or gray.
//!
//! # Example
//!
//! This application stores each selection received from the widget. Convert it
//! to [`iced_core::Color`] when using the selected color elsewhere in your UI.
//!
//! ```no_run
//! use iced::{Color, Element};
//! use iced_color_triangle::{Selection, color_triangle};
//!
//! struct Picker {
//!     selection: Selection,
//! }
//!
//! #[derive(Debug, Clone, Copy)]
//! enum Message {
//!     Pick(Selection),
//! }
//!
//! impl Picker {
//!     fn new() -> Self {
//!         Self {
//!             selection: Color::from_rgb8(255, 64, 64).into(),
//!         }
//!     }
//!
//!     fn update(&mut self, Message::Pick(selection): Message) {
//!         self.selection = selection;
//!     }
//!
//!     fn view(&self) -> Element<'_, Message> {
//!         color_triangle(self.selection, Message::Pick).into()
//!     }
//! }
//!
//! fn main() -> iced::Result {
//!     iced::application(Picker::new, Picker::update, Picker::view)
//!         .title("Color picker")
//!         .run()
//! }
//! ```
//!
//! Enable Iced's `canvas` feature and your preferred renderer and runtime in
//! your application. The widget currently accepts mouse input.

mod color_triangle;
mod selection;
mod wheel;

pub use color_triangle::ColorTriangle;
pub use selection::Selection;

/// Creates a hue ring and triangle selector, 240×240 logical pixels by default.
///
/// Use [`ColorTriangle::width`], [`ColorTriangle::height`], or [`ColorTriangle::size`]
/// to resize it; drawing and input scale together.
///
/// The callback receives candidate selections during dragging and on release.
/// The caller decides when to apply the color, using [`iced_core::Color::from`].
/// Dragging beyond a triangle edge clamps the selection to that edge.
/// The triangle rotates so its colored corner points toward the hue-ring handle.
/// The widget preserves alpha and works with any Iced canvas renderer and theme.
///
/// # Example
///
/// Map selections to an application message and convert the widget to an element:
///
/// ```
/// use iced::{Element, Length};
/// use iced_color_triangle::{Selection, color_triangle};
///
/// #[derive(Clone)]
/// enum Message {
///     Pick(Selection),
/// }
///
/// fn view(selection: Selection) -> Element<'static, Message> {
///     color_triangle(selection, Message::Pick)
///         .width(Length::Fill)
///         .height(300)
///         .into()
/// }
/// ```
pub const fn color_triangle<Message, F: Fn(Selection) -> Message>(
    selection: Selection,
    on_change: F,
) -> ColorTriangle<F> {
    ColorTriangle::new(selection, on_change)
}
