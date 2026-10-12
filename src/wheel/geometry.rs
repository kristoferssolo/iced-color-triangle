use crate::wheel::Drag;
use iced_core::Point;

// Wheel geometry is defined in a 240×240 space and scaled to fit the canvas
pub const SIDE: f32 = 240.0;
pub(super) const CENTER: Point = Point::new(120.0, 120.0);
pub(super) const RING_RADIUS: f32 = 101.0;
pub(super) const RING_WIDTH: f32 = 18.0;
const TOP: f32 = 36.0;
const BOTTOM: f32 = 162.0;
pub(super) const HEIGHT: f32 = BOTTOM - TOP;
const HALF_BASE: f32 = 72.75;
// Rotation round-trips drift exact corners slightly outside the triangle
const HIT_TOLERANCE: f32 = 0.01;

pub(super) fn hit(point: Point, hue: f32) -> Option<Drag> {
    let distance = point.distance(CENTER);
    if (RING_RADIUS - RING_WIDTH / 2.0..=RING_RADIUS + RING_WIDTH / 2.0).contains(&distance) {
        return Some(Drag::Hue);
    }
    let (across, span) = to_local(point, hue);
    let margin = HIT_TOLERANCE / HEIGHT;
    ((-margin..=1.0 + margin).contains(&span) && across.abs() <= span + HIT_TOLERANCE / HALF_BASE)
        .then_some(Drag::Triangle)
}

/// Returns the `(chroma, white)` mix at `point`, clamped to the triangle
pub(super) fn triangle_mix(point: Point, hue: f32) -> (f32, f32) {
    let (across, span) = to_local(point, hue);
    let span = span.clamp(0.0, 1.0);
    let across = across.max(-span).min(span);
    (span.midpoint(across), span.midpoint(-across))
}

pub(super) fn triangle_point(chroma: f32, white: f32, hue: f32) -> Point {
    rotate_point(
        local_point(chroma - white, chroma + white),
        triangle_rotation(hue),
    )
}

/// Returns a point in the unrotated triangle.
///
/// `span` runs from the black apex (0) to the base (1), which equals `chroma + white`.
/// `across` runs from the white corner (`-span`) to the colored corner (`span`),
/// which equals `chroma - white`.
pub(super) const fn local_point(across: f32, span: f32) -> Point {
    Point::new(
        HALF_BASE.mul_add(across, CENTER.x),
        HEIGHT.mul_add(span, TOP),
    )
}

/// Inverts [`local_point`] for a point in the rotated triangle, returning `(across, span)`
fn to_local(point: Point, hue: f32) -> (f32, f32) {
    let point = rotate_point(point, -triangle_rotation(hue));
    ((point.x - CENTER.x) / HALF_BASE, (point.y - TOP) / HEIGHT)
}

/// Returns the ring angle, in radians, at which `hue` is drawn
pub(super) fn hue_angle(hue: f32) -> f32 {
    (hue - 180.0).to_radians()
}

/// Returns the hue drawn at the ring angle of `point`, in `0..=360`
pub(super) fn point_hue(point: Point) -> f32 {
    (point.y - CENTER.y).atan2(point.x - CENTER.x).to_degrees() + 180.0
}

pub(super) fn triangle_rotation(hue: f32) -> f32 {
    hue_angle(hue) - (BOTTOM - CENTER.y).atan2(HALF_BASE)
}

pub(super) fn rotate_point(point: Point, angle: f32) -> Point {
    let (sin, cos) = angle.sin_cos();
    let x = point.x - CENTER.x;
    let y = point.y - CENTER.y;
    Point::new(
        cos.mul_add(x, (-sin).mul_add(y, CENTER.x)),
        sin.mul_add(x, cos.mul_add(y, CENTER.y)),
    )
}

pub(super) fn ring_point(hue: f32) -> Point {
    let angle = hue_angle(hue);
    Point::new(
        RING_RADIUS.mul_add(angle.cos(), CENTER.x),
        RING_RADIUS.mul_add(angle.sin(), CENTER.y),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_none;

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
}
