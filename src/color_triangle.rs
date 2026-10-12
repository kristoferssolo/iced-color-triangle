use crate::{
    Selection,
    wheel::{SIDE, Wheel},
};
use iced_core::{Element, Length};
use iced_widget::{canvas::Canvas, graphics::geometry::Renderer};

/// A hue ring and triangle color selector.
///
/// Created with [`color_triangle`](crate::color_triangle). Defaults to 240×240 logical pixels.
/// The wheel scales to the largest square that fits its bounds and stays centered.
#[must_use]
pub struct ColorTriangle<F> {
    wheel: Wheel<F>,
    width: Length,
    height: Length,
}

impl<F> ColorTriangle<F> {
    pub(crate) const fn new(selection: Selection, on_change: F) -> Self {
        Self {
            wheel: Wheel::new(selection, on_change),
            width: Length::Fixed(SIDE),
            height: Length::Fixed(SIDE),
        }
    }

    /// Sets the widget width
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the widget height
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets both width and height
    pub fn size(self, size: impl Into<Length>) -> Self {
        let size = size.into();
        self.width(size).height(size)
    }
}

impl<'a, Message: 'a, Theme: 'a, R, F> From<ColorTriangle<F>> for Element<'a, Message, Theme, R>
where
    R: Renderer + 'static,
    F: Fn(Selection) -> Message + 'a,
{
    fn from(triangle: ColorTriangle<F>) -> Self {
        Canvas::new(triangle.wheel)
            .width(triangle.width)
            .height(triangle.height)
            .into()
    }
}
