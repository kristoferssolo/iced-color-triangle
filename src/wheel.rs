use crate::{Selection, selection::hue_color};
use iced_core::{
    Color, Element, Event, Length, Point, Radians, Rectangle, Size, Vector,
    mouse::{self, Button, Cursor},
};
use iced_widget::{
    Action,
    canvas::{self, Cache, Frame, Geometry, Path, Program, Stroke, path::Arc},
    graphics::geometry::Renderer,
};
use std::cell::Cell;

// Wheel geometry is defined in a 240×240 space and scaled to fit the canvas.
const SIDE: f32 = 240.0;
const CENTER: Point = Point::new(120.0, 120.0);
const RING_RADIUS: f32 = 101.0;
const RING_WIDTH: f32 = 18.0;
const TOP: f32 = 36.0;
const BOTTOM: f32 = 162.0;
const HALF_BASE: f32 = 72.75;
// Rotation round-trips drift exact corners slightly outside the triangle.
const HIT_TOLERANCE: f32 = 0.01;

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
            wheel: Wheel {
                selection,
                on_change,
            },
            width: Length::Fixed(SIDE),
            height: Length::Fixed(SIDE),
        }
    }

    /// Sets the widget width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the widget height.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets both width and height.
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
        canvas::Canvas::new(triangle.wheel)
            .width(triangle.width)
            .height(triangle.height)
            .into()
    }
}

struct Wheel<F> {
    selection: Selection,
    on_change: F,
}

#[derive(Debug, Clone, Copy)]
enum Drag {
    Hue,
    Triangle,
}

/// Maps the 240×240 wheel space onto a centered square in the canvas.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Transform {
    offset: Vector,
    scale: f32,
}

impl Transform {
    fn fit(size: Size) -> Option<Self> {
        let fitted = size.width.min(size.height);
        (fitted.is_finite() && fitted > 0.0).then(|| Self {
            offset: Vector::new((size.width - fitted) / 2.0, (size.height - fitted) / 2.0),
            scale: fitted / SIDE,
        })
    }

    const fn to_canvas(self, point: Point) -> Point {
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

    fn apply<R: Renderer>(self, frame: &mut Frame<R>) {
        frame.translate(self.offset);
        frame.scale(self.scale);
    }
}

/// Returns the cursor position in wheel space.
fn wheel_point(bounds: Rectangle, cursor: Cursor) -> Option<Point> {
    let transform = Transform::fit(bounds.size())?;
    Some(transform.to_wheel(cursor.position_from(bounds.position())?))
}

struct State<R: Renderer> {
    drag: Option<Drag>,
    // The ring depends only on size, so hue changes rebuild just the triangle.
    ring: Cache<R>,
    triangle: Cache<R>,
    markers: Cache<R>,
    selection: Cell<Option<Selection>>,
    hue: Cell<f32>,
}

impl<R: Renderer> Default for State<R> {
    fn default() -> Self {
        Self {
            drag: None,
            ring: Cache::new(),
            triangle: Cache::new(),
            markers: Cache::new(),
            selection: Cell::new(None),
            hue: Cell::new(0.0),
        }
    }
}

impl<Message, Theme, R, F> Program<Message, Theme, R> for Wheel<F>
where
    R: Renderer + 'static,
    F: Fn(Selection) -> Message,
{
    type State = State<R>;

    fn update(
        &self,
        state: &mut State<R>,
        event: &Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Option<Action<Message>> {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(Button::Left)) => {
                let point = wheel_point(bounds, cursor)?;
                state.drag = hit(point, self.selection.hue);
                self.pick(state.drag?, point)
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let point = wheel_point(bounds, Cursor::Available(*position))?;
                self.pick(state.drag?, point)
            }
            Event::Mouse(mouse::Event::ButtonReleased(Button::Left)) => {
                let drag = state.drag.take()?;
                wheel_point(bounds, cursor)
                    .and_then(|point| self.pick(drag, point))
                    .or_else(|| Some(Action::request_redraw().and_capture()))
            }
            Event::Window(iced_core::window::Event::Unfocused) => {
                state.drag.take().map(|_| Action::request_redraw())
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        state: &State<R>,
        renderer: &R,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry<R>> {
        let Some(transform) = Transform::fit(bounds.size()) else {
            return Vec::new();
        };
        if state.hue.get().to_bits() != self.selection.hue.to_bits() {
            state.triangle.clear();
            state.hue.set(self.selection.hue);
        }
        let ring = state.ring.draw(renderer, bounds.size(), |frame| {
            transform.apply(frame);
            draw_ring(frame);
        });
        let triangle = state.triangle.draw(renderer, bounds.size(), |frame| {
            transform.apply(frame);
            draw_triangle(frame, self.selection.hue);
        });
        if state.selection.get() != Some(self.selection) {
            state.markers.clear();
            state.selection.set(Some(self.selection));
        }
        // Cached geometry gives Iced the complete canvas bounds for damage.
        // Live strokes omit their thickness and can leave old marker pixels behind.
        let markers = state.markers.draw(renderer, bounds.size(), |frame| {
            // Markers keep a constant on-screen size, so only their centers are scaled.
            marker(frame, transform.to_canvas(ring_point(self.selection.hue)));
            let (chroma, white) = self.selection.mix();
            marker(
                frame,
                transform.to_canvas(triangle_point(chroma, white, self.selection.hue)),
            );
        });
        vec![ring, triangle, markers]
    }

    fn mouse_interaction(
        &self,
        state: &State<R>,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> mouse::Interaction {
        if state.drag.is_some()
            || wheel_point(bounds, cursor)
                .and_then(|point| hit(point, self.selection.hue))
                .is_some()
        {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::None
        }
    }
}

impl<F> Wheel<F> {
    fn pick<Message>(&self, drag: Drag, point: Point) -> Option<Action<Message>>
    where
        F: Fn(Selection) -> Message,
    {
        if !point.x.is_finite() || !point.y.is_finite() {
            return None;
        }
        let mut selection = self.selection;
        match drag {
            Drag::Hue => {
                if point.distance(CENTER) < f32::EPSILON {
                    return Some(Action::request_redraw().and_capture());
                }
                selection.hue = ((point.y - CENTER.y).atan2(point.x - CENTER.x).to_degrees()
                    + 180.0)
                    .rem_euclid(360.0);
            }
            Drag::Triangle => {
                let (chroma, white) = triangle_mix(point, self.selection.hue);
                selection = selection.with_mix(chroma, white);
            }
        }
        Some(Action::publish((self.on_change)(selection)).and_capture())
    }
}

fn hit(point: Point, hue: f32) -> Option<Drag> {
    let distance = point.distance(CENTER);
    if (RING_RADIUS - RING_WIDTH / 2.0..=RING_RADIUS + RING_WIDTH / 2.0).contains(&distance) {
        return Some(Drag::Hue);
    }
    let point = rotate_point(point, -triangle_rotation(hue));
    let span = (point.y - TOP) / (BOTTOM - TOP);
    let margin = HIT_TOLERANCE / (BOTTOM - TOP);
    if (-margin..=1.0 + margin).contains(&span)
        && (point.x - CENTER.x).abs() <= HALF_BASE.mul_add(span, HIT_TOLERANCE)
    {
        return Some(Drag::Triangle);
    }
    None
}

fn triangle_mix(point: Point, hue: f32) -> (f32, f32) {
    let point = rotate_point(point, -triangle_rotation(hue));
    let span = ((point.y - TOP) / (BOTTOM - TOP)).clamp(0.0, 1.0);
    if span <= f32::EPSILON {
        return (0.0, 0.0);
    }
    let left = (-HALF_BASE).mul_add(span, CENTER.x);
    let fraction = ((point.x - left) / (2.0 * HALF_BASE * span)).clamp(0.0, 1.0);
    let chroma = fraction * span;
    (chroma, span - chroma)
}

fn triangle_point(chroma: f32, white: f32, hue: f32) -> Point {
    rotate_point(
        Point::new(
            HALF_BASE.mul_add(chroma - white, CENTER.x),
            (BOTTOM - TOP).mul_add(chroma + white, TOP),
        ),
        triangle_rotation(hue),
    )
}

fn triangle_rotation(hue: f32) -> f32 {
    (hue - 180.0).to_radians() - (BOTTOM - CENTER.y).atan2(HALF_BASE)
}

fn rotate_point(point: Point, angle: f32) -> Point {
    let (sin, cos) = angle.sin_cos();
    let x = point.x - CENTER.x;
    let y = point.y - CENTER.y;
    Point::new(
        cos.mul_add(x, (-sin).mul_add(y, CENTER.x)),
        sin.mul_add(x, cos.mul_add(y, CENTER.y)),
    )
}

fn ring_point(hue: f32) -> Point {
    let angle = (hue - 180.0).to_radians();
    Point::new(
        RING_RADIUS.mul_add(angle.cos(), CENTER.x),
        RING_RADIUS.mul_add(angle.sin(), CENTER.y),
    )
}

fn draw_ring<R: Renderer>(frame: &mut Frame<R>) {
    for segment in 0..180u16 {
        let start = f32::from(segment) * 2.0;
        let path = Path::new(|builder| {
            builder.arc(Arc {
                center: CENTER,
                radius: RING_RADIUS,
                start_angle: Radians((start - 180.0).to_radians()),
                end_angle: Radians((start - 177.0).to_radians()),
            });
        });
        let gradient = canvas::gradient::Linear::new(ring_point(start), ring_point(start + 3.0))
            .add_stop(0.0, hue_color(start))
            .add_stop(1.0, hue_color(start + 3.0));
        frame.stroke(
            &path,
            Stroke {
                style: canvas::Style::Gradient(gradient.into()),
                width: RING_WIDTH,
                ..Stroke::default()
            },
        );
    }
}

fn draw_triangle<R: Renderer>(frame: &mut Frame<R>, hue: f32) {
    frame.with_save(|frame| {
        frame.translate(Vector::new(CENTER.x, CENTER.y));
        frame.rotate(triangle_rotation(hue));
        frame.translate(Vector::new(-CENTER.x, -CENTER.y));
        draw_triangle_rows(frame, hue);
    });
}

fn draw_triangle_rows<R: Renderer>(frame: &mut Frame<R>, hue: f32) {
    let color = hue_color(hue);
    for row in 0..252u16 {
        let top = f32::from(row).mul_add(0.5, TOP);
        let bottom = (top + 1.5).min(BOTTOM);
        let top_span = (top - TOP) / (BOTTOM - TOP);
        let bottom_span = (bottom - TOP) / (BOTTOM - TOP);
        let span = ((top + 0.25 - TOP) / (BOTTOM - TOP)).clamp(0.0, 1.0);
        let left = (-HALF_BASE).mul_add(span, CENTER.x);
        let right = HALF_BASE.mul_add(span, CENTER.x);
        let strip = Path::new(|builder| {
            builder.move_to(Point::new((-HALF_BASE).mul_add(top_span, CENTER.x), top));
            builder.line_to(Point::new(HALF_BASE.mul_add(top_span, CENTER.x), top));
            builder.line_to(Point::new(HALF_BASE.mul_add(bottom_span, CENTER.x), bottom));
            builder.line_to(Point::new(
                (-HALF_BASE).mul_add(bottom_span, CENTER.x),
                bottom,
            ));
            builder.close();
        });
        let gradient = canvas::gradient::Linear::new(Point::new(left, top), Point::new(right, top))
            .add_stop(0.0, Color::from_rgb(span, span, span))
            .add_stop(
                1.0,
                Color::from_rgb(color.r * span, color.g * span, color.b * span),
            );
        frame.fill(&strip, gradient);
    }
}

fn marker<R: Renderer>(frame: &mut Frame<R>, point: Point) {
    let path = Path::circle(point, 4.0);
    frame.stroke(
        &path,
        Stroke::default().with_width(4.0).with_color(Color::BLACK),
    );
    frame.stroke(
        &path,
        Stroke::default().with_width(2.0).with_color(Color::WHITE),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_none, assert_some};
    use iced_widget::Renderer;
    use rstest::rstest;

    const PRESS: Event = Event::Mouse(mouse::Event::ButtonPressed(Button::Left));
    const RELEASE: Event = Event::Mouse(mouse::Event::ButtonReleased(Button::Left));

    fn dispatch(
        wheel: &mut Wheel<fn(Selection) -> Selection>,
        state: &mut State<Renderer>,
        event: &Event,
        cursor: Point,
    ) -> Option<Selection> {
        dispatch_in(
            Rectangle::with_size(Size::new(SIDE, SIDE)),
            wheel,
            state,
            event,
            cursor,
        )
    }

    fn dispatch_in(
        bounds: Rectangle,
        wheel: &mut Wheel<fn(Selection) -> Selection>,
        state: &mut State<Renderer>,
        event: &Event,
        cursor: Point,
    ) -> Option<Selection> {
        let selection =
            Program::<Selection>::update(wheel, state, event, bounds, Cursor::Available(cursor))?
                .into_inner()
                .0?;
        wheel.selection = selection;
        Some(selection)
    }

    #[test]
    fn clamps_triangle_drags_and_commits_the_release_position() {
        let mut wheel: Wheel<fn(Selection) -> Selection> = Wheel {
            on_change: std::convert::identity,
            selection: Color::from_rgba8(0, 0, 255, 0.5).into(),
        };
        let mut state = State::default();
        assert_some!(dispatch(
            &mut wheel,
            &mut state,
            &PRESS,
            Point::new(120.0, 120.0)
        ));
        let hue = wheel.selection.hue;
        let outside = rotate_point(Point::new(500.0, 500.0), triangle_rotation(hue));
        let selection = assert_some!(dispatch(
            &mut wheel,
            &mut state,
            &Event::Mouse(mouse::Event::CursorMoved { position: outside }),
            Point::new(120.0, 120.0)
        ));
        assert_eq!(Color::from(selection).into_rgba8(), [0, 0, 255, 128]);
        let selection = assert_some!(dispatch(
            &mut wheel,
            &mut state,
            &RELEASE,
            rotate_point(Point::new(-500.0, 500.0), triangle_rotation(hue))
        ));
        assert_eq!(Color::from(selection).into_rgba8(), [255, 255, 255, 128]);
        assert_none!(dispatch(
            &mut wheel,
            &mut state,
            &Event::Mouse(mouse::Event::CursorMoved { position: outside }),
            outside
        ));
    }

    #[test]
    fn remembers_hue_at_black_and_uses_each_motion_position() {
        let mut wheel: Wheel<fn(Selection) -> Selection> = Wheel {
            on_change: std::convert::identity,
            selection: Color::BLACK.into(),
        };
        let mut state = State::default();
        assert_some!(dispatch(&mut wheel, &mut state, &PRESS, ring_point(0.0)));
        let position = ring_point(120.0);
        let selection = assert_some!(dispatch(
            &mut wheel,
            &mut state,
            &Event::Mouse(mouse::Event::CursorMoved { position }),
            ring_point(0.0)
        ));
        assert!((selection.hue - 120.0).abs() < 0.001);
        assert_eq!(Color::from(selection), Color::BLACK);
        assert_some!(dispatch(&mut wheel, &mut state, &RELEASE, position));
        let selection = assert_some!(dispatch(
            &mut wheel,
            &mut state,
            &PRESS,
            triangle_point(1.0, 0.0, 120.0)
        ));
        assert_eq!(Color::from(selection).into_rgba8(), [0, 255, 0, 255]);
    }

    #[test]
    fn rotated_hit_testing_preserves_the_selected_color_mix() {
        for hue in [0.0, 60.0, 120.0, 180.0, 240.0, 300.0] {
            let mut wheel: Wheel<fn(Selection) -> Selection> = Wheel {
                selection: Selection::from(Color::from_rgba8(0, 0, 255, 0.5)).with_hue(hue),
                on_change: std::convert::identity,
            };
            let mut state = State::default();
            let selected = assert_some!(dispatch(
                &mut wheel,
                &mut state,
                &PRESS,
                triangle_point(0.35, 0.4, hue)
            ));
            let (chroma, white) = selected.mix();
            assert!((chroma - 0.35).abs() < 0.0001);
            assert!((white - 0.4).abs() < 0.0001);
            let [_, _, _, alpha] = Color::from(selected).into_rgba8();
            assert_eq!(alpha, 128);
            let corner = triangle_point(1.0, 0.0, hue) - CENTER;
            let handle = ring_point(hue) - CENTER;
            assert!(corner.x.mul_add(handle.y, -corner.y * handle.x).abs() < 0.01);
            assert!(corner.x.mul_add(handle.x, corner.y * handle.y) > 0.0);
        }
    }

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

    #[rstest]
    #[case(Size::ZERO)]
    #[case(Size::new(0.0, 240.0))]
    #[case(Size::new(f32::INFINITY, f32::INFINITY))]
    fn rejects_degenerate_bounds(#[case] size: Size) {
        assert_none!(Transform::fit(size));
        let mut wheel: Wheel<fn(Selection) -> Selection> = Wheel {
            on_change: std::convert::identity,
            selection: Color::WHITE.into(),
        };
        let mut state = State::default();
        assert_none!(dispatch_in(
            Rectangle::new(Point::ORIGIN, size),
            &mut wheel,
            &mut state,
            &PRESS,
            CENTER
        ));
    }

    #[rstest]
    fn scales_input_with_the_canvas(
        #[values(
            Rectangle::new(Point::new(0.0, 0.0), Size::new(120.0, 120.0)),
            Rectangle::new(Point::new(30.0, 10.0), Size::new(300.0, 200.0)),
            Rectangle::new(Point::new(5.0, 40.0), Size::new(480.0, 480.0))
        )]
        bounds: Rectangle,
        #[values(0.0, 120.0, 300.0)] hue: f32,
    ) {
        let transform = assert_some!(Transform::fit(bounds.size()));
        let screen = |point| {
            let point = transform.to_canvas(point);
            Point::new(point.x + bounds.x, point.y + bounds.y)
        };
        let mut wheel: Wheel<fn(Selection) -> Selection> = Wheel {
            selection: Selection::from(Color::from_rgba8(0, 0, 255, 0.5)).with_hue(hue),
            on_change: std::convert::identity,
        };
        let mut state = State::default();
        let selected = assert_some!(dispatch_in(
            bounds,
            &mut wheel,
            &mut state,
            &PRESS,
            screen(triangle_point(0.35, 0.4, hue))
        ));
        let (chroma, white) = selected.mix();
        assert!((chroma - 0.35).abs() < 0.0001);
        assert!((white - 0.4).abs() < 0.0001);
        assert_some!(dispatch_in(
            bounds, &mut wheel, &mut state, &RELEASE, CENTER
        ));
        let selected = assert_some!(dispatch_in(
            bounds,
            &mut wheel,
            &mut state,
            &PRESS,
            screen(ring_point(hue + 90.0))
        ));
        assert!((selected.hue - (hue + 90.0).rem_euclid(360.0)).abs() < 0.001);
        assert_some!(dispatch_in(
            bounds, &mut wheel, &mut state, &RELEASE, CENTER
        ));
        // On a shrunken canvas, the 240×240 ring radius lies outside the scaled ring.
        if transform.scale < 0.9 {
            let (center, ring) = (screen(CENTER), ring_point(hue + 90.0));
            let unscaled = Point::new(center.x + ring.x - CENTER.x, center.y + ring.y - CENTER.y);
            assert_none!(dispatch_in(
                bounds, &mut wheel, &mut state, &PRESS, unscaled
            ));
        }
    }

    #[test]
    fn hits_every_triangle_corner_at_every_hue() {
        for hue in (0..360u16).map(f32::from) {
            for (chroma, white) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)] {
                let corner = triangle_point(chroma, white, hue);
                assert!(
                    matches!(hit(corner, hue), Some(Drag::Triangle)),
                    "missed corner ({chroma}, {white}) at hue {hue}"
                );
                let outside = corner + (corner - CENTER) * (1.0 / corner.distance(CENTER));
                assert_none!(hit(outside, hue));
            }
        }
    }

    #[test]
    fn ignores_empty_space_and_cancels_a_drag_on_focus_loss() {
        let mut wheel: Wheel<fn(Selection) -> Selection> = Wheel {
            on_change: std::convert::identity,
            selection: Color::WHITE.into(),
        };
        let mut state = State::default();
        assert_none!(dispatch(
            &mut wheel,
            &mut state,
            &PRESS,
            rotate_point(Point::new(120.0, 200.0), triangle_rotation(0.0))
        ));
        assert_some!(dispatch(&mut wheel, &mut state, &PRESS, ring_point(240.0)));
        assert_none!(dispatch(
            &mut wheel,
            &mut state,
            &Event::Window(iced_core::window::Event::Unfocused),
            ring_point(240.0)
        ));
        assert_none!(dispatch(&mut wheel, &mut state, &RELEASE, ring_point(0.0)));
    }
}
