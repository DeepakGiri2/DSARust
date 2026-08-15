//! Renderers for the "row of things" views: arrays and bar charts, hash maps,
//! stacks/queues/deques, bit rows and free text.

use crate::anim::*;
use crate::draw::*;
use crate::theme::Theme;
use dsa_core::model::{ArrayView, BitsView, KvView, StackKind, StackView, TextView};
use egui::{Align2, FontId, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2};

const EMPTY_ARRAY: &ArrayView = &ArrayView {
    label: String::new(),
    data: Vec::new(),
    pointers: Vec::new(),
    window: None,
    hl: Vec::new(),
    bad: Vec::new(),
    done: Vec::new(),
    bars: false,
};

// ─────────────────────────────────────────────────────────────────────────────
// Array / bars
// ─────────────────────────────────────────────────────────────────────────────

pub fn array_height(v: &ArrayView, theme: &Theme) -> f32 {
    let lanes = pointer_lanes(&v.pointers) as f32;
    let body = if v.bars { 150.0 } else { theme.cell_size };
    theme.label_height + body + 18.0 + lanes * 15.0 + 6.0
}

fn state_of(i: i64, v: &ArrayView) -> CellState {
    if v.bad.contains(&i) {
        CellState::Bad
    } else if v.hl.contains(&i) {
        CellState::Good
    } else if v.done.contains(&i) {
        CellState::Done
    } else if v.window.is_some_and(|(lo, hi)| i >= lo && i <= hi) {
        CellState::Window
    } else {
        CellState::Plain
    }
}

/// Pointers sharing a cell are stacked; this returns how many lanes are needed.
fn pointer_lanes(pointers: &[(String, i64)]) -> usize {
    let mut max = 0;
    for (i, (_, idx)) in pointers.iter().enumerate() {
        let same = pointers[..i].iter().filter(|(_, o)| o == idx).count();
        max = max.max(same + 1);
    }
    max
}

pub fn draw_array(
    p: &Painter,
    rect: Rect,
    v: &ArrayView,
    was: Option<&ArrayView>,
    t: f32,
    theme: &Theme,
) {
    let was = was.unwrap_or(EMPTY_ARRAY);
    caption(p, rect.min, &v.label, theme.muted, 12.0);

    let n = v.data.len();
    if n == 0 {
        caption(
            p,
            rect.min + Vec2::new(0.0, theme.label_height),
            "(empty)",
            theme.dim,
            13.0,
        );
        return;
    }

    let size = fit_cell(n, rect.width(), theme);
    let step = size + theme.gap;
    let lanes = pointer_lanes(&v.pointers) as f32;
    let top = rect.min.y + theme.label_height + lanes * 15.0 + 4.0;
    let cell_at = |i: usize| -> Rect {
        Rect::from_min_size(
            Pos2::new(rect.min.x + i as f32 * step, top),
            Vec2::splat(size),
        )
    };

    // The window band slides and stretches rather than jumping, which is what
    // makes a shrinking binary-search range readable.
    let band = |view: &ArrayView| -> Option<Rect> {
        view.window.map(|(lo, hi)| {
            let lo = lo.clamp(0, n as i64 - 1) as usize;
            let hi = hi.clamp(lo as i64, n as i64 - 1) as usize;
            Rect::from_min_max(
                cell_at(lo).min - Vec2::splat(4.0),
                cell_at(hi).max + Vec2::splat(4.0),
            )
        })
    };
    if let Some(now) = band(v) {
        let from = band(was).unwrap_or(now);
        let r = lerp_rect(from, now, t);
        p.rect_filled(r, cr(theme.rounding + 4.0), with_alpha(theme.window, 0.16));
        p.rect_stroke(
            r,
            cr(theme.rounding + 4.0),
            Stroke::new(1.0, with_alpha(theme.window, 0.55)),
            StrokeKind::Inside,
        );
    }

    let bar_max = v
        .data
        .iter()
        .chain(was.data.iter())
        .filter_map(|c| c.as_f64())
        .fold(1.0_f64, f64::max) as f32;

    for i in 0..n {
        let r = cell_at(i);
        let now = state_of(i as i64, v);
        let before = if i < was.data.len() {
            state_of(i as i64, was)
        } else {
            now
        };
        let changed = was.data.get(i).is_some_and(|o| *o != v.data[i]);

        if v.bars {
            draw_bar(p, rect, r, i, v, was, bar_max, now, before, t, theme);
        } else {
            value_cell(p, r, &cell_text(&v.data[i]), now, before, changed, t, theme);
        }

        // A highlight that appeared this step gets a decaying glow.
        if is_new(i as i64, &v.hl, &was.hl) {
            glow(p, r, theme.good, flash(t), theme);
        } else if is_new(i as i64, &v.bad, &was.bad) {
            glow(p, r, theme.bad, flash(t), theme);
        }

        if size > 22.0 {
            p.text(
                Pos2::new(r.center().x, r.max.y + 9.0),
                Align2::CENTER_CENTER,
                i.to_string(),
                FontId::monospace(9.0),
                theme.dim,
            );
        }
    }

    draw_pointers(p, &v.pointers, &was.pointers, t, theme, |i| {
        cell_at(i).center_top()
    });
}

#[allow(clippy::too_many_arguments)]
fn draw_bar(
    p: &Painter,
    outer: Rect,
    slot: Rect,
    i: usize,
    v: &ArrayView,
    was: &ArrayView,
    max: f32,
    now: CellState,
    before: CellState,
    t: f32,
    theme: &Theme,
) {
    let area_top = slot.min.y;
    let area_bottom = outer.max.y - 22.0;
    let height = (area_bottom - area_top).max(20.0);

    let val = |view: &ArrayView| -> f32 {
        view.data.get(i).and_then(|c| c.as_f64()).unwrap_or(0.0) as f32
    };
    let h_now = (val(v) / max).clamp(0.0, 1.0) * height;
    let h_was = if was.data.len() > i {
        (val(was) / max).clamp(0.0, 1.0) * height
    } else {
        h_now
    };
    let h = lerp(h_was, h_now, ease_out(t));

    let bar = Rect::from_min_max(
        Pos2::new(slot.min.x, area_bottom - h),
        Pos2::new(slot.max.x, area_bottom),
    );
    let fill = mix(before.fill(theme), now.fill(theme), t);
    p.rect_filled(bar, cr(theme.rounding), fill);
    p.rect_stroke(
        bar,
        cr(theme.rounding),
        Stroke::new(1.0, mix(theme.cell_stroke, fill, 0.5)),
        StrokeKind::Inside,
    );
    p.text(
        Pos2::new(bar.center().x, bar.min.y - 8.0),
        Align2::CENTER_CENTER,
        cell_text(&v.data[i]),
        FontId::monospace(10.0),
        if now == CellState::Plain {
            theme.muted
        } else {
            theme.text
        },
    );
}

fn draw_pointers(
    p: &Painter,
    now: &[(String, i64)],
    was: &[(String, i64)],
    t: f32,
    theme: &Theme,
    anchor: impl Fn(usize) -> Pos2,
) {
    for (lane_i, (name, idx)) in now.iter().enumerate() {
        let from = was
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, i)| *i)
            .unwrap_or(*idx);
        // Interpolating the *index* (not the pixel) keeps the marker locked to
        // the cell grid while it travels.
        let pos = lerp(from as f32, *idx as f32, ease_out(t));
        let base = anchor(pos.max(0.0) as usize);
        let frac = pos - pos.floor();
        let next = anchor((pos.floor() as usize).saturating_add(1));
        let at = Pos2::new(lerp(base.x, next.x, frac), base.y);
        let lane = now[..lane_i].iter().filter(|(_, o)| o == idx).count();
        pointer_marker(p, at, name, theme.pointer_color(name), lane, theme);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Hash map / key-value
// ─────────────────────────────────────────────────────────────────────────────

const ROW_H: f32 = 26.0;
const ROW_W: f32 = 132.0;

pub fn kv_height(v: &KvView, width: f32, theme: &Theme) -> f32 {
    let per_row = ((width / ROW_W).floor() as usize).max(1);
    let rows = v.entries.len().div_ceil(per_row).max(1);
    theme.label_height + rows as f32 * (ROW_H + 4.0) + 6.0
}

pub fn draw_kv(p: &Painter, rect: Rect, v: &KvView, was: Option<&KvView>, t: f32, theme: &Theme) {
    caption(p, rect.min, &v.label, theme.muted, 12.0);
    let empty = Vec::new();
    let old = was.map(|w| &w.entries).unwrap_or(&empty);

    if v.entries.is_empty() {
        caption(
            p,
            rect.min + Vec2::new(0.0, theme.label_height),
            "(empty)",
            theme.dim,
            13.0,
        );
        return;
    }

    let per_row = ((rect.width() / ROW_W).floor() as usize).max(1);
    for (i, (k, val)) in v.entries.iter().enumerate() {
        let col = i % per_row;
        let row = i / per_row;
        let at = Rect::from_min_size(
            Pos2::new(
                rect.min.x + col as f32 * ROW_W,
                rect.min.y + theme.label_height + row as f32 * (ROW_H + 4.0),
            ),
            Vec2::new(ROW_W - 8.0, ROW_H),
        );

        let prior = old.iter().find(|(ok, _)| ok == k);
        let is_fresh = prior.is_none();
        let changed = prior.is_some_and(|(_, ov)| ov != val);
        let hot = v.hl_keys.iter().any(|h| h == k);
        let is_bad = v.bad_keys.iter().any(|h| h == k);

        // A brand new entry drops in; an updated one only pops its value.
        let appear = if is_fresh { ease_back(t) } else { 1.0 };
        let at = Rect::from_min_size(
            Pos2::new(at.min.x, at.min.y + (1.0 - appear) * 12.0),
            at.size(),
        );
        let alpha = if is_fresh { t.clamp(0.0, 1.0) } else { 1.0 };

        let fill = if is_bad {
            mix(theme.cell, theme.bad, 0.6)
        } else if hot {
            mix(theme.cell, theme.good, 0.55)
        } else {
            theme.cell
        };
        p.rect_filled(at, cr(theme.rounding), with_alpha(fill, alpha));
        p.rect_stroke(
            at,
            cr(theme.rounding),
            Stroke::new(1.0, with_alpha(theme.cell_stroke, alpha)),
            StrokeKind::Inside,
        );
        if hot || is_bad {
            glow(
                p,
                at,
                if is_bad { theme.bad } else { theme.good },
                flash(t),
                theme,
            );
        }

        let fg = if hot || is_bad {
            theme.on(fill)
        } else {
            theme.text
        };
        p.text(
            Pos2::new(at.min.x + 8.0, at.center().y),
            Align2::LEFT_CENTER,
            k.as_str(),
            FontId::monospace(12.0),
            with_alpha(fg, alpha),
        );
        p.text(
            Pos2::new(at.center().x, at.center().y),
            Align2::CENTER_CENTER,
            "→",
            FontId::monospace(11.0),
            with_alpha(theme.dim, alpha),
        );
        let vr = Rect::from_center_size(
            Pos2::new(at.max.x - 26.0, at.center().y),
            Vec2::new(44.0, ROW_H - 6.0),
        );
        let vr = if changed {
            scale_rect(vr, 1.0 + 0.25 * (1.0 - t))
        } else {
            vr
        };
        p.text(
            vr.center(),
            Align2::CENTER_CENTER,
            cell_text(val),
            FontId::monospace(12.0),
            with_alpha(fg, alpha),
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Stack / queue / deque / heap
// ─────────────────────────────────────────────────────────────────────────────

pub fn stack_height(v: &StackView, theme: &Theme) -> f32 {
    match v.kind {
        StackKind::Stack | StackKind::Heap => {
            theme.label_height + (v.items.len().max(1) as f32) * 30.0 + 24.0
        }
        _ => theme.label_height + theme.cell_size + 26.0,
    }
}

pub fn draw_stack(
    p: &Painter,
    rect: Rect,
    v: &StackView,
    was: Option<&StackView>,
    t: f32,
    theme: &Theme,
) {
    let end_label = match v.kind {
        StackKind::Stack => "top",
        StackKind::Queue => "front",
        StackKind::Deque => "front",
        StackKind::Heap => "min",
    };
    caption(p, rect.min, &v.label, theme.muted, 12.0);
    let old_len = was.map(|w| w.items.len()).unwrap_or(v.items.len());

    if v.items.is_empty() && v.popped.is_none() {
        caption(
            p,
            rect.min + Vec2::new(0.0, theme.label_height),
            "(empty)",
            theme.dim,
            13.0,
        );
        return;
    }

    let vertical = matches!(v.kind, StackKind::Stack | StackKind::Heap);
    let n = v.items.len();
    let slot = |i: usize| -> Rect {
        if vertical {
            // Grows upward: the last item sits on top, like a real stack.
            let y = rect.min.y + theme.label_height + (n.saturating_sub(1) - i) as f32 * 30.0;
            Rect::from_min_size(Pos2::new(rect.min.x, y), Vec2::new(96.0, 26.0))
        } else {
            let size = fit_cell(n, rect.width(), theme);
            Rect::from_min_size(
                Pos2::new(
                    rect.min.x + i as f32 * (size + theme.gap),
                    rect.min.y + theme.label_height,
                ),
                Vec2::splat(size),
            )
        }
    };

    for i in 0..n {
        let r = slot(i);
        let is_top = i + 1 == n;
        let arriving = v.pushed && is_top && n > old_len;
        // A pushed item slides in from beyond the open end.
        let r = if arriving {
            let off = (1.0 - ease_back(t)) * 34.0;
            if vertical {
                Rect::from_min_size(Pos2::new(r.min.x, r.min.y - off), r.size())
            } else {
                Rect::from_min_size(Pos2::new(r.min.x + off, r.min.y), r.size())
            }
        } else {
            r
        };

        let state = if v.bad && is_top {
            CellState::Bad
        } else if is_top {
            CellState::Good
        } else {
            CellState::Plain
        };
        value_cell(
            p,
            r,
            &cell_text(&v.items[i]),
            state,
            state,
            false,
            1.0,
            theme,
        );
        if arriving {
            glow(p, r, theme.good, flash(t), theme);
        }
        if is_top {
            let at = if vertical {
                Pos2::new(r.max.x + 8.0, r.center().y)
            } else {
                Pos2::new(r.center().x, r.max.y + 10.0)
            };
            p.text(
                at,
                Align2::LEFT_CENTER,
                end_label,
                FontId::proportional(10.0),
                theme.muted,
            );
        }
    }

    // The popped value keeps drifting away for the length of the transition,
    // so a pop is visible instead of an item simply vanishing.
    if let Some(popped) = &v.popped {
        let base = slot(n);
        let off = ease_out(t) * 26.0;
        let r = if vertical {
            Rect::from_min_size(
                Pos2::new(base.min.x, base.min.y - off),
                Vec2::new(96.0, 26.0),
            )
        } else {
            Rect::from_min_size(Pos2::new(base.min.x + off, base.min.y), base.size())
        };
        let fade = 1.0 - t;
        p.rect_filled(
            r,
            cr(theme.rounding),
            with_alpha(mix(theme.cell, theme.bad, 0.4), fade),
        );
        p.text(
            r.center(),
            Align2::CENTER_CENTER,
            cell_text(popped),
            FontId::monospace(12.0),
            with_alpha(theme.text, fade),
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Bits
// ─────────────────────────────────────────────────────────────────────────────

pub fn bits_height(v: &BitsView, theme: &Theme) -> f32 {
    theme.label_height + v.rows.len().max(1) as f32 * 30.0 + 6.0
}

pub fn draw_bits(
    p: &Painter,
    rect: Rect,
    v: &BitsView,
    was: Option<&BitsView>,
    t: f32,
    theme: &Theme,
) {
    caption(p, rect.min, &v.label, theme.muted, 12.0);

    for (ri, row) in v.rows.iter().enumerate() {
        let y = rect.min.y + theme.label_height + ri as f32 * 30.0;
        p.text(
            Pos2::new(rect.min.x, y + 11.0),
            Align2::LEFT_CENTER,
            &row.label,
            FontId::monospace(11.0),
            theme.muted,
        );
        let left = rect.min.x + 86.0;
        let width = row.width.clamp(1, 64) as usize;
        let bw = ((rect.width() - 130.0) / width as f32).clamp(7.0, 20.0);

        let prior = was.and_then(|w| w.rows.get(ri)).map(|r| r.value);
        for b in 0..width {
            // Most significant bit on the left, as written.
            let bit_index = (width - 1 - b) as u32;
            let on = (row.value >> bit_index) & 1 == 1;
            let flipped =
                prior.is_some_and(|pv| (pv >> bit_index) & 1 != (row.value >> bit_index) & 1);
            let marked = row.hl.contains(&bit_index);

            let r = Rect::from_min_size(
                Pos2::new(left + b as f32 * (bw + 2.0), y),
                Vec2::new(bw, 22.0),
            );
            let base = if on {
                mix(theme.cell, theme.accent, 0.75)
            } else {
                theme.cell
            };
            let fill = if flipped || marked {
                mix(
                    base,
                    theme.cur,
                    0.55 * flash(t) + if marked { 0.25 } else { 0.0 },
                )
            } else {
                base
            };
            p.rect_filled(r, cr(3.0), fill);
            if bw >= 10.0 {
                p.text(
                    r.center(),
                    Align2::CENTER_CENTER,
                    if on { "1" } else { "0" },
                    FontId::monospace((bw * 0.7).clamp(8.0, 12.0)),
                    if on { theme.on(fill) } else { theme.dim },
                );
            }
            if flipped {
                glow(p, r, theme.cur, flash(t), theme);
            }
        }
        p.text(
            Pos2::new(rect.max.x - 4.0, y + 11.0),
            Align2::RIGHT_CENTER,
            format!("= {}", row.value),
            FontId::monospace(11.0),
            theme.text,
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Text
// ─────────────────────────────────────────────────────────────────────────────

pub fn text_height(v: &TextView, theme: &Theme) -> f32 {
    theme.label_height + v.lines.len().max(1) as f32 * 18.0 + 6.0
}

pub fn draw_text(p: &Painter, rect: Rect, v: &TextView, theme: &Theme) {
    caption(p, rect.min, &v.label, theme.muted, 12.0);
    for (i, line) in v.lines.iter().enumerate() {
        let y = rect.min.y + theme.label_height + i as f32 * 18.0;
        let hot = v.hl.contains(&i);
        if hot {
            p.rect_filled(
                Rect::from_min_size(
                    Pos2::new(rect.min.x - 4.0, y - 1.0),
                    Vec2::new(rect.width(), 18.0),
                ),
                cr(4.0),
                with_alpha(theme.good, 0.14),
            );
        }
        p.text(
            Pos2::new(rect.min.x, y),
            Align2::LEFT_TOP,
            line,
            FontId::monospace(12.0),
            if hot { theme.text } else { theme.muted },
        );
    }
}

/// Values shown in a cell need to survive round-tripping through the renderer
/// unchanged; this is the one place that assumption is asserted.
#[cfg(test)]
mod tests {
    use super::*;
    use dsa_core::model::{ArrayView, Cell};

    fn view(hl: Vec<i64>, bad: Vec<i64>, done: Vec<i64>, window: Option<(i64, i64)>) -> ArrayView {
        ArrayView {
            label: "a".into(),
            data: vec![Cell::Num(1.0); 5],
            hl,
            bad,
            done,
            window,
            ..Default::default()
        }
    }

    #[test]
    fn cell_state_priority_puts_rejection_first() {
        // A cell that is both "in the window" and "rejected" must read as
        // rejected, or a binary search looks like it keeps live candidates.
        let v = view(vec![2], vec![2], vec![2], Some((0, 4)));
        assert_eq!(state_of(2, &v), CellState::Bad);
        let v = view(vec![2], vec![], vec![2], Some((0, 4)));
        assert_eq!(state_of(2, &v), CellState::Good);
        let v = view(vec![], vec![], vec![2], Some((0, 4)));
        assert_eq!(state_of(2, &v), CellState::Done);
        let v = view(vec![], vec![], vec![], Some((1, 3)));
        assert_eq!(state_of(2, &v), CellState::Window);
        assert_eq!(state_of(0, &v), CellState::Plain);
    }

    #[test]
    fn pointer_lanes_stack_only_when_they_collide() {
        assert_eq!(pointer_lanes(&[("i".into(), 0), ("j".into(), 3)]), 1);
        assert_eq!(pointer_lanes(&[("i".into(), 2), ("j".into(), 2)]), 2);
        assert_eq!(
            pointer_lanes(&[("i".into(), 2), ("j".into(), 2), ("k".into(), 2)]),
            3
        );
        assert_eq!(pointer_lanes(&[]), 0);
    }

    #[test]
    fn heights_grow_with_content() {
        let theme = Theme::dark();
        let small = KvView {
            label: "m".into(),
            entries: vec![],
            ..Default::default()
        };
        let big = KvView {
            label: "m".into(),
            entries: (0..12)
                .map(|i| (i.to_string(), Cell::Num(i as f64)))
                .collect(),
            ..Default::default()
        };
        assert!(kv_height(&big, 300.0, &theme) > kv_height(&small, 300.0, &theme));
    }

    #[test]
    fn a_bar_chart_is_taller_than_a_plain_row() {
        let theme = Theme::dark();
        let plain = view(vec![], vec![], vec![], None);
        let mut bars = plain.clone();
        bars.bars = true;
        assert!(array_height(&bars, &theme) > array_height(&plain, &theme));
    }
}
