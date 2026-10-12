mod draw;
mod geometry;
mod transform;

pub use geometry::SIDE;

use crate::{
    Selection,
    wheel::{
        draw::{draw_ring, draw_triangle, marker},
        geometry::{CENTER, hit, point_hue, ring_point, triangle_mix, triangle_point},
        transform::{Transform, wheel_point},
    },
};
use iced_core::{
    Event, Point, Rectangle,
    mouse::{self, Button, Cursor},
};
use iced_widget::{
    Action,
    canvas::{Cache, Geometry, Program},
    graphics::geometry::Renderer,
};
use std::cell::Cell;

pub struct Wheel<F> {
    selection: Selection,
    on_change: F,
}

impl<F> Wheel<F> {
    pub const fn new(selection: Selection, on_change: F) -> Self {
        Self {
            selection,
            on_change,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Drag {
    Hue,
    Triangle,
}

pub struct State<R: Renderer> {
    drag: Option<Drag>,
    // The ring depends only on size, so hue changes rebuild just the triangle
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
                state.drag = hit(point, self.selection.hue());
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
        if state.hue.get().to_bits() != self.selection.hue().to_bits() {
            state.triangle.clear();
            state.hue.set(self.selection.hue());
        }
        let ring = state.ring.draw(renderer, bounds.size(), |frame| {
            transform.apply(frame);
            draw_ring(frame, transform.scale);
        });
        let triangle = state.triangle.draw(renderer, bounds.size(), |frame| {
            transform.apply(frame);
            draw_triangle(frame, self.selection.hue());
        });
        if state.selection.get() != Some(self.selection) {
            state.markers.clear();
            state.selection.set(Some(self.selection));
        }
        // Cached geometry gives Iced the complete canvas bounds for damage.
        // Live strokes omit their thickness and can leave old marker pixels behind.
        let markers = state.markers.draw(renderer, bounds.size(), |frame| {
            // Markers keep a constant on-screen size, so only their centers are scaled
            marker(frame, transform.to_canvas(ring_point(self.selection.hue())));
            let (chroma, white) = self.selection.mix();
            marker(
                frame,
                transform.to_canvas(triangle_point(chroma, white, self.selection.hue())),
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
                .and_then(|point| hit(point, self.selection.hue()))
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
                selection = selection.with_hue(point_hue(point));
            }
            Drag::Triangle => {
                let (chroma, white) = triangle_mix(point, self.selection.hue());
                selection = selection.with_mix(chroma, white);
            }
        }
        Some(Action::publish((self.on_change)(selection)).and_capture())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        State, Wheel,
        geometry::{
            CENTER, RING_RADIUS, RING_WIDTH, SIDE, hue_angle, ring_point, rotate_point,
            triangle_point, triangle_rotation,
        },
        transform::Transform,
    };
    use crate::Selection;
    use claims::{assert_none, assert_some};
    use iced_core::{
        Color, Event, Point, Rectangle, Size,
        mouse::{self, Button, Cursor},
    };
    use iced_widget::Renderer;
    use iced_widget::canvas::Program;
    use rstest::rstest;

    const PRESS: Event = Event::Mouse(mouse::Event::ButtonPressed(Button::Left));
    const RELEASE: Event = Event::Mouse(mouse::Event::ButtonReleased(Button::Left));

    type TestWheel = Wheel<fn(Selection) -> Selection>;

    fn new_wheel(selection: impl Into<Selection>) -> TestWheel {
        Wheel::new(selection.into(), std::convert::identity)
    }

    fn dispatch(
        wheel: &mut TestWheel,
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
        wheel: &mut TestWheel,
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
        let mut wheel = new_wheel(Color::from_rgba8(0, 0, 255, 0.5));
        let mut state = State::default();
        assert_some!(dispatch(
            &mut wheel,
            &mut state,
            &PRESS,
            Point::new(120.0, 120.0)
        ));
        let hue = wheel.selection.hue();
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
        let mut wheel = new_wheel(Color::BLACK);
        let mut state = State::default();
        assert_some!(dispatch(&mut wheel, &mut state, &PRESS, ring_point(0.0)));
        let position = ring_point(120.0);
        let selection = assert_some!(dispatch(
            &mut wheel,
            &mut state,
            &Event::Mouse(mouse::Event::CursorMoved { position }),
            ring_point(0.0)
        ));
        assert!((selection.hue() - 120.0).abs() < 0.001);
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
            let mut wheel =
                new_wheel(Selection::from(Color::from_rgba8(0, 0, 255, 0.5)).with_hue(hue));
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
    #[case(Size::ZERO)]
    #[case(Size::new(0.0, 240.0))]
    #[case(Size::new(f32::INFINITY, f32::INFINITY))]
    fn rejects_degenerate_bounds(#[case] size: Size) {
        assert_none!(Transform::fit(size));
        let mut wheel = new_wheel(Color::WHITE);
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
        let mut wheel = new_wheel(Selection::from(Color::from_rgba8(0, 0, 255, 0.5)).with_hue(hue));
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
        assert!((selected.hue() - (hue + 90.0).rem_euclid(360.0)).abs() < 0.001);
        assert_some!(dispatch_in(
            bounds, &mut wheel, &mut state, &RELEASE, CENTER
        ));
        // On a shrunken canvas, the 240×240 ring radius lies outside the scaled ring
        if transform.scale < 0.9 {
            let (center, ring) = (screen(CENTER), ring_point(hue + 90.0));
            let unscaled = Point::new(center.x + ring.x - CENTER.x, center.y + ring.y - CENTER.y);
            assert_none!(dispatch_in(
                bounds, &mut wheel, &mut state, &PRESS, unscaled
            ));
        }
    }

    #[rstest]
    fn clicks_near_both_ring_edges_at_every_size(
        #[values(120.0, 240.0, 360.0)] side: f32,
        #[values(0.0, 135.0, 270.0)] hue: f32,
    ) {
        let bounds = Rectangle::new(Point::new(8.0, 12.0), Size::new(side, side));
        let transform = assert_some!(Transform::fit(bounds.size()));
        let center = transform.to_canvas(CENTER);
        let angle = hue_angle(hue);
        let at = |radius: f32| {
            Point::new(
                radius.mul_add(angle.cos(), center.x + bounds.x),
                radius.mul_add(angle.sin(), center.y + bounds.y),
            )
        };
        let inner = (RING_RADIUS - RING_WIDTH / 2.0) * transform.scale;
        let outer = (RING_RADIUS + RING_WIDTH / 2.0) * transform.scale;
        // Half a pixel inside each edge hits the ring; half a pixel outside misses
        for (radius, hits) in [
            (inner + 0.5, true),
            (outer - 0.5, true),
            (inner - 0.5, false),
            (outer + 0.5, false),
        ] {
            let mut wheel = new_wheel(Color::WHITE);
            let mut state = State::default();
            let selected = dispatch_in(bounds, &mut wheel, &mut state, &PRESS, at(radius));
            assert_eq!(
                selected.is_some(),
                hits,
                "side {side}, hue {hue}: radius {radius} between {inner} and {outer}"
            );
            if let Some(selected) = selected {
                assert!((selected.hue() - hue).abs() < 0.01);
            }
        }
    }

    #[test]
    fn ignores_empty_space_and_cancels_a_drag_on_focus_loss() {
        let mut wheel = new_wheel(Color::WHITE);
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
