//! Interpolation helpers — the difference between a slideshow and a debugger
//! you can actually follow.
//!
//! Every renderer is handed the previous step's view alongside the current
//! one and a progress value `t`. Anything positional (a pointer index, a bar
//! height, a cursor cell) is interpolated; anything categorical (a cell turned
//! green) is cross-faded, with a brief overshoot so the eye is drawn to what
//! *changed* rather than having to diff two frames itself.

use dsa_core::model::VizView;
use egui::{Color32, Pos2, Rect, Vec2};

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

pub fn lerp_pos(a: Pos2, b: Pos2, t: f32) -> Pos2 {
    Pos2::new(lerp(a.x, b.x, t), lerp(a.y, b.y, t))
}

pub fn lerp_rect(a: Rect, b: Rect, t: f32) -> Rect {
    Rect::from_min_max(lerp_pos(a.min, b.min, t), lerp_pos(a.max, b.max, t))
}

/// Straight RGBA mix. Good enough for UI accents and avoids the gamma games
/// that make two-colour fades look muddy in the middle.
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(
        f(a.r(), b.r()),
        f(a.g(), b.g()),
        f(a.b(), b.b()),
        f(a.a(), b.a()),
    )
}

pub fn with_alpha(c: Color32, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (a.clamp(0.0, 1.0) * 255.0) as u8)
}

/// A short bright pulse at the start of a transition, decaying to nothing.
/// Used for "this just happened" emphasis: a stored map key, a flipped bit.
pub fn flash(t: f32) -> f32 {
    (1.0 - t.clamp(0.0, 1.0)).powi(2)
}

/// Slight overshoot then settle — a push landing on a stack, a bar growing.
pub fn ease_back(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

pub fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

/// Grow a rect about its centre — the "pop" for a value that just changed.
pub fn scale_rect(r: Rect, s: f32) -> Rect {
    Rect::from_center_size(r.center(), r.size() * s)
}

pub fn inflate(r: Rect, by: f32) -> Rect {
    Rect::from_min_max(r.min - Vec2::splat(by), r.max + Vec2::splat(by))
}

/// Pair each current view with its counterpart from the previous step.
///
/// Views only animate against a view of the same kind *and* label, so an
/// author who keeps labels stable gets motion for free, and one who rebuilds a
/// label every step gets a clean cut instead of nonsense tweening.
pub fn pair<'a>(
    prev: Option<&'a [VizView]>,
    cur: &'a [VizView],
) -> Vec<(&'a VizView, Option<&'a VizView>)> {
    let prev = prev.unwrap_or(&[]);
    let mut used = vec![false; prev.len()];
    cur.iter()
        .map(|c| {
            let key = c.anim_key();
            let found = prev
                .iter()
                .enumerate()
                .find(|(i, p)| !used[*i] && p.anim_key() == key)
                .map(|(i, p)| {
                    used[i] = true;
                    p
                });
            (c, found)
        })
        .collect()
}

/// `true` when `idx` is newly present in `now` compared with `was` — the cue
/// for a flash rather than a steady highlight.
pub fn is_new(idx: i64, now: &[i64], was: &[i64]) -> bool {
    now.contains(&idx) && !was.contains(&idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dsa_core::model::{ArrayView, KvView};

    fn arr(label: &str) -> VizView {
        VizView::Array(ArrayView {
            label: label.into(),
            ..Default::default()
        })
    }
    fn kv(label: &str) -> VizView {
        VizView::Kv(KvView {
            label: label.into(),
            ..Default::default()
        })
    }

    #[test]
    fn views_pair_by_kind_and_label() {
        let prev = vec![arr("nums"), kv("seen")];
        let cur = vec![arr("nums"), kv("seen")];
        let pairs = pair(Some(&prev), &cur);
        assert!(pairs.iter().all(|(_, p)| p.is_some()));
    }

    #[test]
    fn a_renamed_view_does_not_tween_against_the_old_one() {
        let prev = vec![arr("nums")];
        let cur = vec![arr("nums (target = 9)")];
        let pairs = pair(Some(&prev), &cur);
        assert!(pairs[0].1.is_none());
    }

    #[test]
    fn a_view_kind_change_does_not_pair() {
        let prev = vec![arr("x")];
        let cur = vec![kv("x")];
        assert!(pair(Some(&prev), &cur)[0].1.is_none());
    }

    #[test]
    fn duplicate_labels_pair_one_to_one() {
        let prev = vec![arr("a"), arr("a")];
        let cur = vec![arr("a"), arr("a"), arr("a")];
        let pairs = pair(Some(&prev), &cur);
        assert!(pairs[0].1.is_some());
        assert!(pairs[1].1.is_some());
        assert!(pairs[2].1.is_none(), "third has nothing left to pair with");
    }

    #[test]
    fn the_first_step_has_nothing_to_pair_against() {
        let cur = vec![arr("a")];
        assert!(pair(None, &cur)[0].1.is_none());
    }

    #[test]
    fn easing_curves_hit_their_endpoints() {
        for f in [ease_out as fn(f32) -> f32, ease_back] {
            assert!((f(0.0) - 0.0).abs() < 1e-3, "f(0) should be 0");
            assert!((f(1.0) - 1.0).abs() < 1e-3, "f(1) should be 1");
        }
        assert!(
            ease_back(0.75) > 1.0,
            "ease_back should overshoot before settling"
        );
        assert_eq!(flash(1.0), 0.0);
        assert_eq!(flash(0.0), 1.0);
    }

    #[test]
    fn mix_is_symmetric_at_the_endpoints() {
        let a = Color32::from_rgb(0, 0, 0);
        let b = Color32::from_rgb(255, 255, 255);
        assert_eq!(mix(a, b, 0.0), a);
        assert_eq!(mix(a, b, 1.0), b);
        assert_eq!(mix(a, b, 0.5).r(), 128);
    }

    #[test]
    fn new_index_detection() {
        assert!(is_new(3, &[1, 3], &[1]));
        assert!(!is_new(1, &[1, 3], &[1]));
        assert!(!is_new(9, &[1], &[]));
    }
}
