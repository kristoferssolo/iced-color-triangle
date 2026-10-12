use claims::assert_some;
use iced_color_triangle::{Selection, color_triangle};
use iced_core::{
    Color, Element, Font, Layout, Pixels, Rectangle, Size, Theme,
    layout::Limits,
    mouse::Cursor,
    renderer::{Renderer as _, Style},
    widget::Tree,
};
use iced_tiny_skia::{
    Layer, Renderer,
    graphics::{Viewport, damage},
};
use tiny_skia::{Mask, Pixmap};

const BOUNDS: Rectangle = Rectangle {
    x: 0.0,
    y: 0.0,
    width: 240.0,
    height: 240.0,
};
const BACKGROUND: Color = Color::from_rgb8(40, 42, 44);

fn paint(renderer: &mut Renderer, tree: &mut Tree, selection: Selection) {
    paint_within(renderer, tree, selection, BOUNDS.size());
}

fn paint_within(renderer: &mut Renderer, tree: &mut Tree, selection: Selection, limit: Size) {
    let mut element: Element<'_, Selection, Theme, Renderer> =
        color_triangle(selection, std::convert::identity).into();
    tree.diff(element.as_widget());
    let node = element
        .as_widget_mut()
        .layout(tree, renderer, &Limits::new(Size::ZERO, limit));
    renderer.reset(BOUNDS);
    element.as_widget().draw(
        tree,
        renderer,
        &Theme::Dark,
        &Style {
            text_color: Color::WHITE,
        },
        Layout::new(&node),
        Cursor::Unavailable,
        &BOUNDS,
    );
}

#[test]
fn successive_redraws_match_a_fresh_frame() {
    for physical_side in [240, 300, 480] {
        let scale = match physical_side {
            300 => 1.25,
            480 => 2.0,
            _ => 1.0,
        };
        let viewport = Viewport::with_physical_size(Size::new(physical_side, physical_side), scale);
        let mut incremental = assert_some!(Pixmap::new(physical_side, physical_side));
        let mut expected = assert_some!(Pixmap::new(physical_side, physical_side));
        let mut mask = assert_some!(Mask::new(physical_side, physical_side));
        let mut renderer = Renderer::new(Font::DEFAULT, Pixels(16.0));
        let initial = Selection::from(Color::from_rgb8(20, 30, 180));
        let element: Element<'_, Selection, Theme, Renderer> =
            color_triangle(initial, std::convert::identity).into();
        let mut tree = Tree::new(element.as_widget());
        let mut previous: Option<Vec<Layer>> = None;
        for step in 0..4u8 {
            let selection = initial
                .with_lightness(f32::from(step).mul_add(0.06, 0.12))
                .with_hue(f32::from(step).mul_add(40.0, 240.0));
            paint(&mut renderer, &mut tree, selection);
            let regions = previous.as_ref().map_or_else(
                || vec![BOUNDS],
                |previous| {
                    damage::group(
                        damage::diff(
                            previous,
                            renderer.layers(),
                            |layer| vec![layer.bounds],
                            Layer::damage,
                        ),
                        BOUNDS,
                    )
                },
            );
            previous = Some(renderer.layers().to_vec());
            renderer.draw(
                &mut incremental.as_mut(),
                &mut mask,
                &viewport,
                &regions,
                BACKGROUND,
            );
            renderer.draw(
                &mut expected.as_mut(),
                &mut mask,
                &viewport,
                &[BOUNDS],
                BACKGROUND,
            );
            let different = incremental
                .data()
                .iter()
                .zip(expected.data())
                .filter(|(a, b)| a != b)
                .count();
            assert_eq!(
                different, 0,
                "scale {scale}, step {step}: incremental damage left stale pixels"
            );
        }
    }
}

#[test]
fn rotated_triangle_has_no_background_gaps() {
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(16.0));
    let initial = Selection::from(Color::from_rgb8(20, 30, 180));
    let element: Element<'_, Selection, Theme, Renderer> =
        color_triangle(initial, std::convert::identity).into();
    let mut tree = Tree::new(element.as_widget());
    for hue in [0.0, 60.0, 120.0, 180.0, 240.0, 300.0] {
        for side in [240_u32, 300, 480] {
            let scale = match side {
                300 => 1.25,
                480 => 2.0,
                _ => 1.0,
            };
            let selection = initial
                .with_hue(hue)
                .with_saturation(1.0)
                .with_lightness(0.5);
            paint(&mut renderer, &mut tree, selection);
            let mut pixmap = assert_some!(Pixmap::new(side, side));
            let mut mask = assert_some!(Mask::new(side, side));
            let viewport = Viewport::with_physical_size(Size::new(side, side), scale);
            renderer.draw(
                &mut pixmap.as_mut(),
                &mut mask,
                &viewport,
                &[BOUNDS],
                BACKGROUND,
            );
            let center = side.midpoint(0);
            for y in center.saturating_sub(8)..=center.saturating_add(8) {
                for x in center.saturating_sub(8)..=center.saturating_add(8) {
                    let pixel = assert_some!(pixmap.pixel(x, y));
                    assert!(
                        pixel.red().min(pixel.green()).min(pixel.blue()) >= 50,
                        "background gap at ({x}, {y}), hue {hue}, scale {scale}"
                    );
                }
            }
        }
    }
}

#[test]
fn layout_constraints_scale_the_drawing() {
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(16.0));
    let selection = Selection::from(Color::from_rgb8(255, 0, 0));
    let element: Element<'_, Selection, Theme, Renderer> =
        color_triangle(selection, std::convert::identity).into();
    let mut tree = Tree::new(element.as_widget());
    paint_within(&mut renderer, &mut tree, selection, Size::new(120.0, 120.0));
    let mut pixmap = assert_some!(Pixmap::new(240, 240));
    let mut mask = assert_some!(Mask::new(240, 240));
    let viewport = Viewport::with_physical_size(Size::new(240, 240), 1.0);
    renderer.draw(
        &mut pixmap.as_mut(),
        &mut mask,
        &viewport,
        &[BOUNDS],
        BACKGROUND,
    );
    // The pixmap stores BGRA, so compare against a known background pixel
    let background = assert_some!(pixmap.pixel(0, 0));
    let is_background = |x, y| assert_some!(pixmap.pixel(x, y)) == background;
    // The ring's right edge shows hue 180 (cyan) at half the default radius
    let ring = assert_some!(pixmap.pixel(110, 60));
    let (low, high) = (ring.red().min(ring.blue()), ring.red().max(ring.blue()));
    assert!(
        ring.green() > 200 && high > 200 && low < 50,
        "ring missing at its scaled radius: {ring:?}"
    );
    assert!(
        !is_background(60, 60),
        "triangle missing at the scaled center"
    );
    assert!(
        is_background(221, 120),
        "drawing extends past the constrained bounds"
    );
}

fn render_at(side: u16, selection: Selection) -> Pixmap {
    let length = f32::from(side);
    let bounds = Rectangle::with_size(Size::new(length, length));
    let mut renderer = Renderer::new(Font::DEFAULT, Pixels(16.0));
    let mut element: Element<'_, Selection, Theme, Renderer> =
        color_triangle(selection, std::convert::identity)
            .size(length)
            .into();
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &Limits::new(Size::ZERO, bounds.size()),
    );
    renderer.reset(bounds);
    element.as_widget().draw(
        &tree,
        &mut renderer,
        &Theme::Dark,
        &Style {
            text_color: Color::WHITE,
        },
        Layout::new(&node),
        Cursor::Unavailable,
        &bounds,
    );
    let mut pixmap = assert_some!(Pixmap::new(side.into(), side.into()));
    let mut mask = assert_some!(Mask::new(side.into(), side.into()));
    let viewport = Viewport::with_physical_size(Size::new(side.into(), side.into()), 1.0);
    renderer.draw(
        &mut pixmap.as_mut(),
        &mut mask,
        &viewport,
        &[bounds],
        BACKGROUND,
    );
    pixmap
}

#[test]
fn ring_thickness_scales_with_the_widget() {
    for side in [120_u16, 240, 360] {
        let pixmap = render_at(side, Selection::from(Color::from_rgb8(255, 0, 0)));
        let scale = f32::from(side) / 240.0;
        // The ring spans radii 92..=110; nothing is drawn between 86 and the ring
        let (gap, inner, outer) = (86.0 * scale, 92.0 * scale, 110.0 * scale);
        let row = u32::from(side / 2);
        // The pixmap stores BGRA, so compare against a known background pixel
        let background = assert_some!(pixmap.pixel(0, 0));
        // Walk the ring's right side, skipping a 1.5 px antialiased margin at each edge
        for x in side / 2..side {
            let radius = f32::from(x) + 0.5 - f32::from(side) / 2.0;
            let drawn = assert_some!(pixmap.pixel(x.into(), row)) != background;
            if (inner + 1.5..=outer - 1.5).contains(&radius) {
                assert!(drawn, "side {side}: ring missing at radius {radius}");
            } else if (gap..=inner - 1.5).contains(&radius) || radius >= outer + 1.5 {
                assert!(!drawn, "side {side}: ring drawn at radius {radius}");
            }
        }
    }
}
