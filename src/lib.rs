//! A hue ring and triangle color selector for Iced.
//!
//! Retain [`Selection`] in application state and replace it on each change.
//! This preserves the chosen hue when the color becomes black, white, or gray.

mod selection;
mod wheel;

pub use selection::Selection;
pub use wheel::ColorTriangle;

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
pub const fn color_triangle<Message, F: Fn(Selection) -> Message>(
    selection: Selection,
    on_change: F,
) -> ColorTriangle<F> {
    ColorTriangle::new(selection, on_change)
}
