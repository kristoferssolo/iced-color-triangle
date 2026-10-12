use crate::{
    selection::hue_color,
    wheel::geometry::{
        CENTER, HEIGHT, RING_RADIUS, RING_WIDTH, hue_angle, local_point, ring_point,
        triangle_rotation,
    },
};
use iced_core::{Color, Point, Radians, Vector};
use iced_widget::{
    canvas::{self, Frame, Path, Stroke, path::Arc},
    graphics::geometry::Renderer,
};

pub(super) fn draw_ring<R: Renderer>(frame: &mut Frame<R>, scale: f32) {
    for segment in 0..180u16 {
        let start = f32::from(segment) * 2.0;
        let path = Path::new(|builder| {
            builder.arc(Arc {
                center: CENTER,
                radius: RING_RADIUS,
                start_angle: Radians(hue_angle(start)),
                end_angle: Radians(hue_angle(start + 3.0)),
            });
        });
        let gradient = canvas::gradient::Linear::new(ring_point(start), ring_point(start + 3.0))
            .add_stop(0.0, hue_color(start))
            .add_stop(1.0, hue_color(start + 3.0));
        frame.stroke(
            &path,
            Stroke {
                style: canvas::Style::Gradient(gradient.into()),
                width: RING_WIDTH * scale,
                ..Stroke::default()
            },
        );
    }
}

pub(super) fn draw_triangle<R: Renderer>(frame: &mut Frame<R>, hue: f32) {
    frame.with_save(|frame| {
        frame.translate(Vector::new(CENTER.x, CENTER.y));
        frame.rotate(triangle_rotation(hue));
        frame.translate(Vector::new(-CENTER.x, -CENTER.y));
        draw_triangle_rows(frame, hue);
    });
}

fn draw_triangle_rows<R: Renderer>(frame: &mut Frame<R>, hue: f32) {
    let color = hue_color(hue);
    // 1.5 px strips advance by 0.5 px, so their antialiased edges overlap without gaps
    for row in 0..252u16 {
        let top = f32::from(row) * 0.5 / HEIGHT;
        let bottom = (top + 1.5 / HEIGHT).min(1.0);
        let span = (top + 0.25 / HEIGHT).min(1.0);
        let strip = Path::new(|builder| {
            builder.move_to(local_point(-top, top));
            builder.line_to(local_point(top, top));
            builder.line_to(local_point(bottom, bottom));
            builder.line_to(local_point(-bottom, bottom));
            builder.close();
        });
        let gradient =
            canvas::gradient::Linear::new(local_point(-span, span), local_point(span, span))
                .add_stop(0.0, Color::from_rgb(span, span, span))
                .add_stop(
                    1.0,
                    Color::from_rgb(color.r * span, color.g * span, color.b * span),
                );
        frame.fill(&strip, gradient);
    }
}

pub(super) fn marker<R: Renderer>(frame: &mut Frame<R>, point: Point) {
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
