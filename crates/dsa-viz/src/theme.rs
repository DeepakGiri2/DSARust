//! Colours and metrics.
//!
//! One palette drives every renderer so a green cell means the same thing in
//! an array, a grid and a graph. Both themes are defined here rather than
//! derived from egui's visuals, because the semantic colours (found / rejected
//! / current / settled) need to stay legible and distinct regardless of what
//! the surrounding UI looks like.

use egui::Color32;

#[derive(Clone, Debug)]
pub struct Theme {
    pub dark: bool,

    pub bg: Color32,
    pub panel: Color32,
    pub cell: Color32,
    pub cell_stroke: Color32,
    pub text: Color32,
    pub muted: Color32,

    /// Pointers and cursors.
    pub accent: Color32,
    /// Found / matched / good.
    pub good: Color32,
    /// Rejected / mismatch.
    pub bad: Color32,
    /// The cell under the cursor right now.
    pub cur: Color32,
    /// Sliding-window / live-range band.
    pub window: Color32,
    /// Finished, no longer interesting.
    pub dim: Color32,

    /// Distinct colours for named pointers, assigned in first-seen order.
    pub pointer_palette: Vec<Color32>,

    pub cell_size: f32,
    pub gap: f32,
    pub rounding: f32,
    pub label_height: f32,
}

impl Theme {
    pub fn dark() -> Self {
        // Same values as the web version's :root, so the canvas and the shell
        // around it are visibly one product.
        Self {
            dark: true,
            bg: Color32::from_rgb(0x0b, 0x0e, 0x14),
            panel: Color32::from_rgb(0x11, 0x15, 0x1f),
            cell: Color32::from_rgb(0x16, 0x1b, 0x28),
            cell_stroke: Color32::from_rgb(0x23, 0x2a, 0x3b),
            text: Color32::from_rgb(0xd6, 0xdb, 0xe8),
            muted: Color32::from_rgb(0x8b, 0x93, 0xa7),
            accent: Color32::from_rgb(0x7c, 0x6c, 0xff),
            good: Color32::from_rgb(0x34, 0xd3, 0x99),
            bad: Color32::from_rgb(0xf8, 0x71, 0x71),
            cur: Color32::from_rgb(0xfb, 0xbf, 0x24),
            window: Color32::from_rgb(0x22, 0xd3, 0xee),
            dim: Color32::from_rgb(0x4a, 0x51, 0x63),
            pointer_palette: vec![
                Color32::from_rgb(0x7c, 0x6c, 0xff),
                Color32::from_rgb(0xff, 0x8f, 0x5a),
                Color32::from_rgb(0x34, 0xd3, 0x99),
                Color32::from_rgb(0x22, 0xd3, 0xee),
                Color32::from_rgb(0xfb, 0xbf, 0x24),
                Color32::from_rgb(0xd8, 0x7c, 0xff),
            ],
            cell_size: 44.0,
            gap: 6.0,
            rounding: 6.0,
            label_height: 20.0,
        }
    }

    pub fn light() -> Self {
        Self {
            dark: false,
            bg: Color32::from_rgb(0xfa, 0xfb, 0xfd),
            panel: Color32::from_rgb(0xff, 0xff, 0xff),
            cell: Color32::from_rgb(0xed, 0xf0, 0xf6),
            cell_stroke: Color32::from_rgb(0xc9, 0xd1, 0xe0),
            text: Color32::from_rgb(0x1b, 0x20, 0x2c),
            muted: Color32::from_rgb(0x66, 0x6f, 0x84),
            accent: Color32::from_rgb(0x14, 0x6c, 0xd8),
            good: Color32::from_rgb(0x0e, 0x9f, 0x63),
            bad: Color32::from_rgb(0xd6, 0x33, 0x3f),
            cur: Color32::from_rgb(0xb5, 0x7d, 0x00),
            window: Color32::from_rgb(0x71, 0x4a, 0xe0),
            dim: Color32::from_rgb(0xa8, 0xb1, 0xc2),
            pointer_palette: vec![
                Color32::from_rgb(0x14, 0x6c, 0xd8),
                Color32::from_rgb(0xd4, 0x62, 0x0d),
                Color32::from_rgb(0x0e, 0x9f, 0x63),
                Color32::from_rgb(0x8b, 0x3d, 0xd6),
                Color32::from_rgb(0xb5, 0x7d, 0x00),
                Color32::from_rgb(0x0d, 0x91, 0x9e),
            ],
            cell_size: 44.0,
            gap: 6.0,
            rounding: 6.0,
            label_height: 20.0,
        }
    }

    /// Stable colour for a named pointer. Hashing the name (rather than using
    /// its position) keeps `i` the same colour even when `j` appears later.
    pub fn pointer_color(&self, name: &str) -> Color32 {
        let h = name.bytes().fold(2166136261u32, |acc, b| {
            (acc ^ b as u32).wrapping_mul(16777619)
        });
        self.pointer_palette[(h as usize) % self.pointer_palette.len()]
    }

    /// Text that stays readable on top of `bg`.
    pub fn on(&self, bg: Color32) -> Color32 {
        let l = 0.299 * bg.r() as f32 + 0.587 * bg.g() as f32 + 0.114 * bg.b() as f32;
        if l > 140.0 {
            Color32::from_rgb(0x10, 0x14, 0x1c)
        } else {
            Color32::from_rgb(0xf2, 0xf5, 0xfa)
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_colours_are_stable_per_name() {
        let t = Theme::dark();
        assert_eq!(t.pointer_color("i"), t.pointer_color("i"));
        assert_ne!(t.pointer_color("i"), t.pointer_color("j"));
    }

    #[test]
    fn label_contrast_flips_with_background_luminance() {
        let t = Theme::dark();
        assert_eq!(t.on(Color32::WHITE).r(), 0x10);
        assert_eq!(t.on(Color32::BLACK).r(), 0xf2);
    }
}
