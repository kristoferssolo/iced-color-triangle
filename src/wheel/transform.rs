use crate::wheel::SIDE;
use iced_core::{Point, Rectangle, Size, Vector, mouse::Cursor};
use iced_widget::{canvas::Frame, graphics::geometry::Renderer};

/// Maps the 240×240 wheel space onto a centered square in the canvas
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Transform {
    offset: Vector,
    pub(super) scale: f32,
}

impl Transform {
    pub(super) fn fit(size: Size) -> Option<Self> {
        let fitted = size.width.min(size.height);
        (fitted.is_finite() && fitted > 0.0).then(|| Self {
            offset: Vector::new((size.width - fitted) / 2.0, (size.height - fitted) / 2.0),
            scale: fitted / SIDE,
        })
    }

    pub(super) const fn to_canvas(self, point: Point) -> Point {
        Point::new(
            point.x.mul_add(self.scale, self.offset.x),
            point.y.mul_add(self.scale, self.offset.y),
        )
    }

    fn to_wheel(self, point: Point) -> Point {
        Point::new(
            (point.x - self.offset.x) / self.scale,
            (point.y - self.offset.y) / self.scale,
        )
    }

    pub(super) fn apply<R: Renderer>(self, frame: &mut Frame<R>) {
        frame.translate(self.offset);
        frame.scale(self.scale);
    }
}

/// Returns the cursor position in wheel space
pub(super) fn wheel_point(bounds: Rectangle, cursor: Cursor) -> Option<Point> {
    let transform = Transform::fit(bounds.size())?;
    Some(transform.to_wheel(cursor.position_from(bounds.position())?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_some;
    use rstest::rstest;

    #[rstest]
    #[case(Size::new(240.0, 240.0), Vector::new(0.0, 0.0), 1.0)]
    #[case(Size::new(120.0, 120.0), Vector::new(0.0, 0.0), 0.5)]
    #[case(Size::new(300.0, 200.0), Vector::new(50.0, 0.0), 200.0 / 240.0)]
    #[case(Size::new(100.0, 480.0), Vector::new(0.0, 190.0), 100.0 / 240.0)]
    fn fits_the_largest_centered_square(
        #[case] size: Size,
        #[case] offset: Vector,
        #[case] scale: f32,
    ) {
        let transform = assert_some!(Transform::fit(size));
        assert!((transform.offset.x - offset.x).abs() < 1e-4);
        assert!((transform.offset.y - offset.y).abs() < 1e-4);
        assert!((transform.scale - scale).abs() < 1e-6);
        let point = Point::new(37.0, 201.0);
        assert!(
            transform
                .to_wheel(transform.to_canvas(point))
                .distance(point)
                < 1e-3
        );
    }
}
