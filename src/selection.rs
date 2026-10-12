use iced_core::Color;
use palette::{FromColor, Hsl, Srgb};

/// A selected color that retains hue and saturation at black and white.
///
/// Hue is expressed in degrees; saturation, lightness, and alpha are in `0..=1`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Selection {
    pub(crate) hue: f32,
    saturation: f32,
    lightness: f32,
    alpha: f32,
}

impl From<Color> for Selection {
    fn from(color: Color) -> Self {
        let hsl = Hsl::from_color(Srgb::new(
            channel(color.r),
            channel(color.g),
            channel(color.b),
        ));
        Self {
            hue: wrap_hue(hsl.hue.into_positive_degrees()),
            saturation: hsl.saturation,
            lightness: hsl.lightness,
            alpha: channel(color.a),
        }
    }
}

impl From<Selection> for Color {
    fn from(selection: Selection) -> Self {
        let rgb = Srgb::from_color(Hsl::new(
            selection.hue,
            selection.saturation,
            selection.lightness,
        ));
        Self::from_rgba(rgb.red, rgb.green, rgb.blue, selection.alpha)
    }
}

impl Selection {
    /// Returns hue in degrees, in `0..360`
    #[must_use]
    pub const fn hue(self) -> f32 {
        self.hue
    }

    /// Returns HSL saturation, in `0..=`
    #[must_use]
    pub const fn saturation(self) -> f32 {
        self.saturation
    }

    /// Returns HSL lightness, in `0..=1`
    #[must_use]
    pub const fn lightness(self) -> f32 {
        self.lightness
    }

    /// Replaces hue, wrapping finite values into `0..360` and ignoring non-finite values
    #[must_use]
    pub fn with_hue(mut self, hue: f32) -> Self {
        if hue.is_finite() {
            self.hue = wrap_hue(hue);
        }
        self
    }

    /// Replaces saturation, clamping finite values into `0..=1` and ignoring non-finite values
    #[must_use]
    pub const fn with_saturation(mut self, saturation: f32) -> Self {
        if saturation.is_finite() {
            self.saturation = saturation.clamp(0.0, 1.0);
        }
        self
    }

    /// Replaces lightness, clamping finite values into `0..=1` and ignoring non-finite values
    #[must_use]
    pub const fn with_lightness(mut self, lightness: f32) -> Self {
        if lightness.is_finite() {
            self.lightness = lightness.clamp(0.0, 1.0);
        }
        self
    }

    pub(crate) fn mix(self) -> (f32, f32) {
        let chroma = (1.0 - 2.0f32.mul_add(self.lightness, -1.0).abs()) * self.saturation;
        (chroma, self.lightness - chroma / 2.0)
    }

    pub(crate) fn with_mix(mut self, chroma: f32, white: f32) -> Self {
        self.lightness = (chroma / 2.0 + white).clamp(0.0, 1.0);
        let span = 1.0 - 2.0f32.mul_add(self.lightness, -1.0).abs();
        if span > f32::EPSILON {
            self.saturation = (chroma / span).clamp(0.0, 1.0);
        }
        self
    }
}

pub fn hue_color(hue: f32) -> Color {
    let rgb = Srgb::from_color(Hsl::new(hue, 1.0, 0.5));
    Color::from_rgb(rgb.red, rgb.green, rgb.blue)
}

fn wrap_hue(hue: f32) -> f32 {
    let hue = hue.rem_euclid(360.0);
    if hue < 360.0 { hue } else { 0.0 }
}

const fn channel(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    const TEAL: Color = Color::from_rgba(0.2, 0.6, 0.5, 0.25);

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1e-4,
            "expected {expected}, got {actual}"
        );
    }

    #[rstest]
    #[case([0, 0, 0, 255])]
    #[case([255, 255, 255, 255])]
    #[case([128, 128, 128, 64])]
    #[case([255, 0, 0, 255])]
    #[case([0, 255, 0, 128])]
    #[case([0, 0, 255, 0])]
    #[case([12, 200, 99, 255])]
    #[case([250, 3, 140, 17])]
    fn round_trips_rgba8(#[case] rgba: [u8; 4]) {
        let [r, g, b, a] = rgba;
        let color = Color::from_rgba8(r, g, b, f32::from(a) / 255.0);
        assert_eq!(Color::from(Selection::from(color)).into_rgba8(), rgba);
    }

    #[rstest]
    #[case(Color::from_rgb(1.0, 0.0, 0.0), 0.0)]
    #[case(Color::from_rgb(1.0, 1.0, 0.0), 60.0)]
    #[case(Color::from_rgb(0.0, 1.0, 0.0), 120.0)]
    #[case(Color::from_rgb(0.0, 0.0, 1.0), 240.0)]
    #[case(Color::from_rgb(1.0, 0.0, 0.5), 330.0)]
    fn reads_hue_in_degrees(#[case] color: Color, #[case] hue: f32) {
        assert_close(Selection::from(color).hue(), hue);
    }

    #[rstest]
    #[case(0.0, 0.0)]
    #[case(359.5, 359.5)]
    #[case(360.0, 0.0)]
    #[case(725.0, 5.0)]
    #[case(-30.0, 330.0)]
    #[case(-720.0, 0.0)]
    #[case(-1e-6, 0.0)]
    fn wraps_hue(#[case] hue: f32, #[case] expected: f32) {
        let hue = Selection::from(TEAL).with_hue(hue).hue();
        assert!((0.0..360.0).contains(&hue), "{hue} is outside 0..360");
        assert_close(hue, expected);
    }

    #[rstest]
    fn ignores_non_finite_components(
        #[values(f32::NAN, f32::INFINITY, f32::NEG_INFINITY)] value: f32,
    ) {
        let selection = Selection::from(TEAL);
        assert_eq!(selection.with_hue(value), selection);
        assert_eq!(selection.with_saturation(value), selection);
        assert_eq!(selection.with_lightness(value), selection);
    }

    #[rstest]
    #[case(-0.5, 0.0)]
    #[case(0.25, 0.25)]
    #[case(1.5, 1.0)]
    fn clamps_saturation_and_lightness(#[case] value: f32, #[case] expected: f32) {
        let selection = Selection::from(TEAL);
        assert_close(selection.with_saturation(value).saturation(), expected);
        assert_close(selection.with_lightness(value).lightness(), expected);
    }

    #[rstest]
    #[case(f32::NAN, 0.0)]
    #[case(f32::INFINITY, 0.0)]
    #[case(f32::NEG_INFINITY, 0.0)]
    #[case(-1.0, 0.0)]
    #[case(2.0, 1.0)]
    fn sanitizes_color_channels(#[case] value: f32, #[case] expected: f32) {
        let color = Color::from(Selection::from(Color {
            r: value,
            g: value,
            b: value,
            a: value,
        }));
        for channel in [color.r, color.g, color.b, color.a] {
            assert_close(channel, expected);
        }
    }

    #[rstest]
    fn preserves_alpha(
        #[values(0.0, 0.25, 0.5, 1.0)] alpha: f32,
        #[values(
            |s: Selection| s,
            |s: Selection| s.with_hue(200.0),
            |s: Selection| s.with_saturation(0.1),
            |s: Selection| s.with_lightness(0.0),
            |s: Selection| s.with_lightness(1.0),
            |s: Selection| s.with_mix(0.3, 0.2),
        )]
        edit: fn(Selection) -> Selection,
    ) {
        let selection = edit(Selection::from(Color { a: alpha, ..TEAL }));
        assert_close(Color::from(selection).a, alpha);
    }

    #[rstest]
    #[case::black_by_lightness(|s: Selection| s.with_lightness(0.0), Color::BLACK)]
    #[case::white_by_lightness(|s: Selection| s.with_lightness(1.0), Color::WHITE)]
    #[case::black_by_mix(|s: Selection| s.with_mix(0.0, 0.0), Color::BLACK)]
    #[case::white_by_mix(|s: Selection| s.with_mix(0.0, 1.0), Color::WHITE)]
    fn retains_hue_and_saturation_through_black_and_white(
        #[case] extreme: fn(Selection) -> Selection,
        #[case] color: Color,
    ) {
        let original = Selection::from(TEAL);
        let selection = extreme(original);
        assert_eq!(
            Color::from(selection).into_rgba8(),
            Color { a: TEAL.a, ..color }.into_rgba8()
        );
        assert_close(selection.hue(), original.hue());
        assert_close(selection.saturation(), original.saturation());
        let restored = selection.with_lightness(original.lightness());
        assert_eq!(Color::from(restored).into_rgba8(), TEAL.into_rgba8());
    }
}
