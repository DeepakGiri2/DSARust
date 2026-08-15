//! Shared drawing primitives, so a "cell" looks and behaves identically
//! whether it is an array slot, a grid square or a stack entry.

use crate::anim::{inflate, mix, scale_rect, with_alpha};
use crate::theme::Theme;
use dsa_core::model::Cell;
use egui::{Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2};

pub fn cr(v: f32) -> CornerRadius {
    CornerRadius::same(v.clamp(0.0, 255.0) as u8)
}

/// What a cell is currently *meaning*. Renderers compute this for the previous
/// and the current step and cross-fade between the two.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CellState {
    #[default]
    Plain,
    /// Finished / no longer a candidate.
    Done,
    /// Matched, chosen, part of the answer.
    Good,
    /// Rejected, mismatched.
    Bad,
    /// Under the cursor right now.
    Cur,
    /// Inside the live window.
    Window,
}

impl CellState {
    pub fn fill(self, theme: &Theme) -> Color32 {
        match self {
            CellState::Plain => theme.cell,
            CellState::Done => mix(theme.cell, theme.bg, 0.55),
            CellState::Good => mix(theme.cell, theme.good, 0.75),
            CellState::Bad => mix(theme.cell, theme.bad, 0.7),
            CellState::Cur => mix(theme.cell, theme.cur, 0.55),
            CellState::Window => mix(theme.cell, theme.window, 0.35),
        }
    }

    pub fn text(self, theme: &Theme) -> Color32 {
        match self {
            CellState::Done => theme.dim,
            CellState::Good | CellState::Bad | CellState::Cur => theme.on(self.fill(theme)),
            _ => theme.text,
        }
    }
}

/// Draw one value cell, cross-fading from `was` to `now` and popping briefly
/// when the value itself changed.
#[allow(clippy::too_many_arguments)]
pub fn value_cell(
    p: &Painter,
    rect: Rect,
    text: &str,
    now: CellState,
    was: CellState,
    changed: bool,
    t: f32,
    theme: &Theme,
) {
    let fill = mix(was.fill(theme), now.fill(theme), t);
    // A changed value grows past its final size and settles back, which reads
    // as "look here" without any colour change at all.
    let rect = if changed {
        scale_rect(rect, 1.0 + 0.18 * (1.0 - t))
    } else {
        rect
    };

    p.rect_filled(rect, cr(theme.rounding), fill);
    let stroke_c = if now == CellState::Plain {
        theme.cell_stroke
    } else {
        mix(theme.cell_stroke, now.fill(theme), 0.6)
    };
    p.rect_stroke(
        rect,
        cr(theme.rounding),
        Stroke::new(1.0, stroke_c),
        StrokeKind::Inside,
    );

    let size = (rect.height() * 0.42).clamp(9.0, 17.0);
    p.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::monospace(size),
        mix(was.text(theme), now.text(theme), t),
    );
}

/// A soft glow behind a cell — used for "this just happened".
pub fn glow(p: &Painter, rect: Rect, color: Color32, strength: f32, theme: &Theme) {
    if strength <= 0.01 {
        return;
    }
    for i in 0..3 {
        let grow = 2.0 + i as f32 * 3.0;
        p.rect_filled(
            inflate(rect, grow),
            cr(theme.rounding + grow),
            with_alpha(color, 0.10 * strength),
        );
    }
}

pub fn caption(p: &Painter, at: Pos2, text: &str, color: Color32, size: f32) {
    p.text(
        at,
        Align2::LEFT_TOP,
        text,
        FontId::proportional(size),
        color,
    );
}

pub fn mono_centered(p: &Painter, at: Pos2, text: &str, color: Color32, size: f32) {
    p.text(
        at,
        Align2::CENTER_CENTER,
        text,
        FontId::monospace(size),
        color,
    );
}

/// A named pointer marker: a coloured tab with the name, pointing down at a
/// cell. Multiple pointers on the same cell are stacked so none is hidden.
pub fn pointer_marker(
    p: &Painter,
    tip: Pos2,
    name: &str,
    color: Color32,
    lane: usize,
    theme: &Theme,
) {
    let lift = 4.0 + lane as f32 * 15.0;
    let w = (name.len() as f32 * 7.0 + 12.0).max(20.0);
    let h = 14.0;
    let body = Rect::from_center_size(Pos2::new(tip.x, tip.y - lift - h * 0.5), Vec2::new(w, h));
    p.rect_filled(body, cr(4.0), color);
    p.add(egui::Shape::convex_polygon(
        vec![
            Pos2::new(tip.x - 4.0, body.max.y),
            Pos2::new(tip.x + 4.0, body.max.y),
            Pos2::new(tip.x, body.max.y + 4.0),
        ],
        color,
        Stroke::NONE,
    ));
    mono_centered(p, body.center(), name, theme.on(color), 10.0);
}

/// Straight arrow with a solid head, used by lists and graphs.
pub fn arrow(p: &Painter, from: Pos2, to: Pos2, color: Color32, width: f32) {
    let dir = to - from;
    let len = dir.length();
    if len < 1.0 {
        return;
    }
    let unit = dir / len;
    let head = 7.0_f32.min(len * 0.4);
    let base = to - unit * head;
    p.line_segment([from, base], Stroke::new(width, color));
    let normal = Vec2::new(-unit.y, unit.x);
    p.add(egui::Shape::convex_polygon(
        vec![to, base + normal * head * 0.5, base - normal * head * 0.5],
        color,
        Stroke::NONE,
    ));
}

/// Fit `count` cells into `avail` pixels, shrinking (never growing) to fit.
pub fn fit_cell(count: usize, avail: f32, theme: &Theme) -> f32 {
    if count == 0 {
        return theme.cell_size;
    }
    let ideal = theme.cell_size + theme.gap;
    let needed = count as f32 * ideal;
    if needed <= avail {
        theme.cell_size
    } else {
        ((avail / count as f32) - theme.gap).max(10.0)
    }
}

pub fn cell_text(c: &Cell) -> String {
    let s = c.to_string();
    // Long strings would overflow their box; the tooltip in the app shows the
    // full value, the cell shows enough to recognise it.
    if s.chars().count() > 6 {
        format!("{}…", s.chars().take(5).collect::<String>())
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_shrink_to_fit_but_never_grow() {
        let t = Theme::dark();
        assert_eq!(
            fit_cell(3, 1000.0, &t),
            t.cell_size,
            "plenty of room -> default size"
        );
        let tight = fit_cell(40, 400.0, &t);
        assert!(tight < t.cell_size);
        assert!(tight >= 10.0, "never collapses to nothing");
        assert_eq!(fit_cell(0, 100.0, &t), t.cell_size);
    }

    #[test]
    fn long_values_are_elided() {
        assert_eq!(cell_text(&Cell::Num(12.0)), "12");
        assert_eq!(cell_text(&Cell::Str("abcdefgh".into())), "abcde…");
        assert_eq!(cell_text(&Cell::Str("abc".into())), "abc");
    }

    #[test]
    fn states_stay_distinguishable_from_plain() {
        let t = Theme::dark();
        for s in [
            CellState::Good,
            CellState::Bad,
            CellState::Cur,
            CellState::Window,
            CellState::Done,
        ] {
            assert_ne!(
                s.fill(&t),
                CellState::Plain.fill(&t),
                "{s:?} must not look plain"
            );
        }
    }
}
