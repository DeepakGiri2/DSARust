//! Renderers for the structural views: linked lists, trees, grids and graphs.
//!
//! These all share one trick. Rather than special-casing every kind of
//! structural change, each renderer lays out the *previous* step and the
//! *current* step independently and interpolates each node's position by id.
//! A swapped pair of subtrees, a node appended to a list, an edge relaxed in a
//! graph — they all animate for free, because the layout moved and the drawing
//! followed it.

use crate::anim::*;
use crate::draw::*;
use crate::theme::Theme;
use dsa_core::model::{GraphView, GridView, LinkedListView, TreeView};
use egui::{Align2, FontId, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2};
use std::collections::BTreeMap;

type Layout = BTreeMap<i64, Pos2>;

fn at(layout: &Layout, was: &Layout, id: i64, t: f32) -> Pos2 {
    let now = layout.get(&id).copied().unwrap_or(Pos2::ZERO);
    match was.get(&id) {
        Some(before) => lerp_pos(*before, now, ease_out(t)),
        // A node that did not exist before appears in place rather than
        // flying in from the origin.
        None => now,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Linked list
// ─────────────────────────────────────────────────────────────────────────────

pub fn list_height(theme: &Theme) -> f32 {
    theme.label_height + 88.0
}

fn list_layout(v: &LinkedListView, rect: Rect, theme: &Theme) -> (Layout, f32) {
    let n = v.nodes.len().max(1);
    let node_w = ((rect.width() - 40.0) / n as f32 - 26.0).clamp(26.0, 62.0);
    let step = node_w + 34.0;
    let y = rect.min.y + theme.label_height + 44.0;
    let mut out = Layout::new();
    for (i, node) in v.nodes.iter().enumerate() {
        out.insert(
            node.id,
            Pos2::new(rect.min.x + 8.0 + i as f32 * step + node_w * 0.5, y),
        );
    }
    (out, node_w)
}

pub fn draw_list(
    p: &Painter,
    rect: Rect,
    v: &LinkedListView,
    was: Option<&LinkedListView>,
    t: f32,
    theme: &Theme,
) {
    caption(p, rect.min, &v.label, theme.muted, 12.0);
    let (layout, node_w) = list_layout(v, rect, theme);
    let old_layout = was
        .map(|w| list_layout(w, rect, theme).0)
        .unwrap_or_default();

    // Arrows first so nodes draw over their endpoints.
    for node in &v.nodes {
        let from = at(&layout, &old_layout, node.id, t);
        let flipped = v.reversed.contains(&node.id);
        let color = if flipped { theme.good } else { theme.muted };
        match node.next {
            Some(next_id) if layout.contains_key(&next_id) => {
                let to = at(&layout, &old_layout, next_id, t);
                let (a, b) = if to.x >= from.x {
                    (
                        from + Vec2::new(node_w * 0.5 + 2.0, 0.0),
                        to - Vec2::new(node_w * 0.5 + 6.0, 0.0),
                    )
                } else {
                    // A backwards arrow bows under the row so it is not hidden
                    // behind the nodes it passes.
                    (
                        from + Vec2::new(-node_w * 0.5 - 2.0, 6.0),
                        to + Vec2::new(node_w * 0.5 + 6.0, 6.0),
                    )
                };
                arrow(p, a, b, color, if flipped { 2.2 } else { 1.4 });
            }
            _ => {
                let a = from + Vec2::new(node_w * 0.5 + 2.0, 0.0);
                let b = a + Vec2::new(20.0, 0.0);
                arrow(p, a, b, color, 1.4);
                p.text(
                    b + Vec2::new(4.0, 0.0),
                    Align2::LEFT_CENTER,
                    "nil",
                    FontId::monospace(10.0),
                    theme.dim,
                );
            }
        }
    }

    for node in &v.nodes {
        let c = at(&layout, &old_layout, node.id, t);
        let r = Rect::from_center_size(c, Vec2::new(node_w, 32.0));
        let flipped = v.reversed.contains(&node.id);
        let was_flipped = was.is_some_and(|w| w.reversed.contains(&node.id));
        let state = if flipped {
            CellState::Good
        } else {
            CellState::Plain
        };
        let before = if was_flipped {
            CellState::Good
        } else {
            CellState::Plain
        };
        value_cell(p, r, &cell_text(&node.val), state, before, false, t, theme);
        if flipped && !was_flipped {
            glow(p, r, theme.good, flash(t), theme);
        }
    }

    // Pointer tabs travel between nodes with the same easing as array cursors.
    for (lane_i, (name, target)) in v.pointers.iter().enumerate() {
        let from = was
            .and_then(|w| {
                w.pointers
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, id)| *id)
            })
            .unwrap_or(*target);
        let a = from.and_then(|id| layout.get(&id).copied());
        let b = target.and_then(|id| layout.get(&id).copied());
        let nil_spot = Pos2::new(rect.max.x - 26.0, rect.min.y + theme.label_height + 44.0);
        let start = a.unwrap_or(nil_spot);
        let end = b.unwrap_or(nil_spot);
        let tip = lerp_pos(start, end, ease_out(t)) - Vec2::new(0.0, 18.0);
        let lane = v.pointers[..lane_i]
            .iter()
            .filter(|(_, o)| o == target)
            .count();
        pointer_marker(p, tip, name, theme.pointer_color(name), lane, theme);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tree
// ─────────────────────────────────────────────────────────────────────────────

pub fn tree_height(v: &TreeView, theme: &Theme) -> f32 {
    let depth = tree_depth(v);
    theme.label_height + (depth.max(1) as f32) * 58.0 + 30.0
}

fn tree_depth(v: &TreeView) -> usize {
    fn walk(v: &TreeView, id: Option<i64>, seen: &mut Vec<i64>) -> usize {
        let Some(id) = id else { return 0 };
        if seen.contains(&id) {
            return 0;
        }
        seen.push(id);
        let Some(n) = v.nodes.iter().find(|n| n.id == id) else {
            return 0;
        };
        1 + walk(v, n.left, seen).max(walk(v, n.right, seen))
    }
    walk(v, v.root, &mut Vec::new())
}

/// In-order x, depth y — the layout everyone draws binary trees with, and the
/// one that makes a BST read left-to-right in sorted order.
fn tree_layout(v: &TreeView, rect: Rect, theme: &Theme) -> Layout {
    let mut order: Vec<(i64, usize)> = Vec::new();
    let mut seen: Vec<i64> = Vec::new();
    fn walk(
        v: &TreeView,
        id: Option<i64>,
        depth: usize,
        order: &mut Vec<(i64, usize)>,
        seen: &mut Vec<i64>,
    ) {
        let Some(id) = id else { return };
        if seen.contains(&id) || depth > 24 {
            return; // malformed content must not hang the UI
        }
        seen.push(id);
        let Some(n) = v.nodes.iter().find(|n| n.id == id) else {
            return;
        };
        walk(v, n.left, depth + 1, order, seen);
        order.push((id, depth));
        walk(v, n.right, depth + 1, order, seen);
    }
    walk(v, v.root, 0, &mut order, &mut seen);

    let n = order.len().max(1);
    let usable = (rect.width() - 40.0).max(60.0);
    let step = (usable / n as f32).min(78.0);
    let left = rect.min.x + 20.0 + (usable - step * n as f32).max(0.0) * 0.5;
    let top = rect.min.y + theme.label_height + 22.0;

    order
        .into_iter()
        .enumerate()
        .map(|(slot, (id, depth))| {
            (
                id,
                Pos2::new(left + (slot as f32 + 0.5) * step, top + depth as f32 * 58.0),
            )
        })
        .collect()
}

pub fn draw_tree(
    p: &Painter,
    rect: Rect,
    v: &TreeView,
    was: Option<&TreeView>,
    t: f32,
    theme: &Theme,
) {
    caption(p, rect.min, &v.label, theme.muted, 12.0);
    if v.nodes.is_empty() {
        caption(
            p,
            rect.min + Vec2::new(0.0, theme.label_height),
            "(empty tree)",
            theme.dim,
            13.0,
        );
        return;
    }

    let layout = tree_layout(v, rect, theme);
    let old = was.map(|w| tree_layout(w, rect, theme)).unwrap_or_default();
    let radius = 16.0;

    for node in &v.nodes {
        let Some(_) = layout.get(&node.id) else {
            continue;
        };
        let from = at(&layout, &old, node.id, t);
        for child in [node.left, node.right].into_iter().flatten() {
            if !layout.contains_key(&child) {
                continue;
            }
            let to = at(&layout, &old, child, t);
            let on_path = v.path.contains(&node.id) && v.path.contains(&child);
            let color = if on_path {
                theme.accent
            } else {
                theme.cell_stroke
            };
            let dir = (to - from).normalized();
            p.line_segment(
                [from + dir * radius, to - dir * radius],
                Stroke::new(if on_path { 2.2 } else { 1.3 }, color),
            );
        }
    }

    // The swap marker arcs between the two nodes that traded places.
    if let Some((a, b)) = v.swap {
        if let (Some(_), Some(_)) = (layout.get(&a), layout.get(&b)) {
            let pa = at(&layout, &old, a, t);
            let pb = at(&layout, &old, b, t);
            let mid = Pos2::new((pa.x + pb.x) * 0.5, pa.y.max(pb.y) + 26.0);
            let pts: Vec<Pos2> = (0..=16)
                .map(|i| {
                    let s = i as f32 / 16.0;
                    let one = lerp_pos(pa, mid, s);
                    let two = lerp_pos(mid, pb, s);
                    lerp_pos(one, two, s)
                })
                .collect();
            p.add(egui::Shape::line(
                pts,
                Stroke::new(2.0, with_alpha(theme.cur, 0.35 + 0.45 * flash(t))),
            ));
        }
    }

    for node in &v.nodes {
        if !layout.contains_key(&node.id) {
            continue;
        }
        let c = at(&layout, &old, node.id, t);
        let is_cur = v.cur == Some(node.id);
        let was_cur = was.is_some_and(|w| w.cur == Some(node.id));
        let done = v.done.contains(&node.id);
        let on_path = v.path.contains(&node.id);

        let fill_now = if is_cur {
            mix(theme.cell, theme.cur, 0.6)
        } else if done {
            mix(theme.cell, theme.good, 0.45)
        } else if on_path {
            mix(theme.cell, theme.accent, 0.4)
        } else {
            theme.cell
        };
        let fill_was = if was_cur {
            mix(theme.cell, theme.cur, 0.6)
        } else {
            fill_now
        };
        let fill = mix(fill_was, fill_now, t);

        if is_cur && !was_cur {
            p.circle_filled(
                c,
                radius + 6.0 * flash(t) + 3.0,
                with_alpha(theme.cur, 0.22 * flash(t)),
            );
        }
        p.circle_filled(c, radius, fill);
        p.circle_stroke(
            c,
            radius,
            Stroke::new(1.2, mix(theme.cell_stroke, fill, 0.5)),
        );
        mono_centered(p, c, &cell_text(&node.val), theme.on(fill), 12.0);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Grid
// ─────────────────────────────────────────────────────────────────────────────

pub fn grid_height(v: &GridView, width: f32, theme: &Theme) -> f32 {
    let rows = v.data.len().max(1);
    let cols = v.data.first().map(|r| r.len()).unwrap_or(1).max(1);
    let size = grid_cell_size(rows, cols, width, theme);
    theme.label_height + rows as f32 * (size + 3.0) + 8.0
}

fn grid_cell_size(rows: usize, cols: usize, width: f32, theme: &Theme) -> f32 {
    let by_width = (width - 24.0) / cols as f32 - 3.0;
    // Keep tall grids from pushing everything else off the screen.
    let by_height = 320.0 / rows as f32 - 3.0;
    by_width.min(by_height).clamp(14.0, theme.cell_size)
}

pub fn draw_grid(
    p: &Painter,
    rect: Rect,
    v: &GridView,
    was: Option<&GridView>,
    t: f32,
    theme: &Theme,
) {
    caption(p, rect.min, &v.label, theme.muted, 12.0);
    let rows = v.data.len();
    if rows == 0 {
        return;
    }
    let cols = v.data[0].len();
    let size = grid_cell_size(rows, cols, rect.width(), theme);
    let step = size + 3.0;
    let top = rect.min.y + theme.label_height;

    let cell_rect = |r: usize, c: usize| {
        Rect::from_min_size(
            Pos2::new(rect.min.x + c as f32 * step, top + r as f32 * step),
            Vec2::splat(size),
        )
    };
    let state = |view: &GridView, rc: (i64, i64)| -> CellState {
        if view.bad.contains(&rc) {
            CellState::Bad
        } else if view.hl.contains(&rc) {
            CellState::Good
        } else if view.path.contains(&rc) {
            CellState::Window
        } else if view.done.contains(&rc) {
            CellState::Done
        } else {
            CellState::Plain
        }
    };

    for r in 0..rows {
        for c in 0..v.data[r].len() {
            let rc = (r as i64, c as i64);
            let now = state(v, rc);
            let before = was.map(|w| state(w, rc)).unwrap_or(now);
            let changed = was
                .and_then(|w| w.data.get(r).and_then(|row| row.get(c)))
                .is_some_and(|old| *old != v.data[r][c]);
            value_cell(
                p,
                cell_rect(r, c),
                &cell_text(&v.data[r][c]),
                now,
                before,
                changed,
                t,
                theme,
            );
        }
    }

    // The cursor outline slides from cell to cell.
    if let Some((r, c)) = v.cur {
        let from = was.and_then(|w| w.cur).unwrap_or((r, c));
        let a = cell_rect(from.0.max(0) as usize, from.1.max(0) as usize);
        let b = cell_rect(r.max(0) as usize, c.max(0) as usize);
        let cur = inflate(lerp_rect(a, b, ease_out(t)), 2.0);
        p.rect_stroke(
            cur,
            cr(theme.rounding),
            Stroke::new(2.0, theme.cur),
            StrokeKind::Outside,
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Graph
// ─────────────────────────────────────────────────────────────────────────────

pub fn graph_height() -> f32 {
    270.0
}

fn graph_layout(v: &GraphView, rect: Rect, theme: &Theme) -> Layout {
    let n = v.nodes.len().max(1);
    let cx = rect.center().x;
    let cy = rect.min.y + theme.label_height + (rect.height() - theme.label_height) * 0.5;
    let radius = ((rect.height() - theme.label_height) * 0.5 - 30.0)
        .min(rect.width() * 0.5 - 40.0)
        .max(40.0);

    v.nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let pos = match (node.x, node.y) {
                (Some(x), Some(y)) => Pos2::new(
                    rect.min.x + 30.0 + x * (rect.width() - 60.0),
                    rect.min.y
                        + theme.label_height
                        + 20.0
                        + y * (rect.height() - theme.label_height - 50.0),
                ),
                // Deterministic circle: the same graph always looks the same,
                // which matters when stepping back and forth.
                _ => {
                    let a =
                        std::f32::consts::TAU * i as f32 / n as f32 - std::f32::consts::FRAC_PI_2;
                    Pos2::new(cx + radius * a.cos(), cy + radius * a.sin())
                }
            };
            (node.id, pos)
        })
        .collect()
}

pub fn draw_graph(
    p: &Painter,
    rect: Rect,
    v: &GraphView,
    was: Option<&GraphView>,
    t: f32,
    theme: &Theme,
) {
    caption(p, rect.min, &v.label, theme.muted, 12.0);
    let layout = graph_layout(v, rect, theme);
    let old = was
        .map(|w| graph_layout(w, rect, theme))
        .unwrap_or_default();
    let radius = 17.0;

    for e in &v.edges {
        let (Some(_), Some(_)) = (layout.get(&e.from), layout.get(&e.to)) else {
            continue;
        };
        let a = at(&layout, &old, e.from, t);
        let b = at(&layout, &old, e.to, t);
        let active = v.active.contains(&(e.from, e.to)) || v.active.contains(&(e.to, e.from));
        let color = if active {
            theme.accent
        } else {
            with_alpha(theme.cell_stroke, 0.9)
        };
        let dir = (b - a).normalized();
        let (s, t_end) = (a + dir * radius, b - dir * radius);
        if e.directed {
            arrow(p, s, t_end, color, if active { 2.4 } else { 1.3 });
        } else {
            p.line_segment(
                [s, t_end],
                Stroke::new(if active { 2.4 } else { 1.3 }, color),
            );
        }
        if active {
            // A dot runs along the edge in the direction of travel — this is
            // what makes a traversal legible rather than a colour change.
            let travel = lerp_pos(s, t_end, ease_out(t));
            p.circle_filled(travel, 4.0, theme.accent);
        }
        if let Some(w) = &e.weight {
            let mid = lerp_pos(s, t_end, 0.5);
            p.text(
                mid,
                Align2::CENTER_CENTER,
                w.to_string(),
                FontId::monospace(10.0),
                theme.muted,
            );
        }
    }

    for node in &v.nodes {
        let c = at(&layout, &old, node.id, t);
        let is_cur = v.cur == Some(node.id);
        let fill = if v.bad.contains(&node.id) {
            mix(theme.cell, theme.bad, 0.7)
        } else if is_cur {
            mix(theme.cell, theme.cur, 0.6)
        } else if v.done.contains(&node.id) {
            mix(theme.cell, theme.good, 0.5)
        } else if v.frontier.contains(&node.id) {
            mix(theme.cell, theme.accent, 0.45)
        } else {
            theme.cell
        };
        let was_cur = was.is_some_and(|w| w.cur == Some(node.id));
        if is_cur && !was_cur {
            p.circle_filled(
                c,
                radius + 8.0 * flash(t),
                with_alpha(theme.cur, 0.25 * flash(t)),
            );
        }
        p.circle_filled(c, radius, fill);
        p.circle_stroke(
            c,
            radius,
            Stroke::new(1.2, mix(theme.cell_stroke, fill, 0.5)),
        );
        mono_centered(p, c, &node.label, theme.on(fill), 12.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dsa_core::model::{Cell, GraphNodeV, TreeNodeV};

    fn tree(spec: &[(i64, Option<i64>, Option<i64>)]) -> TreeView {
        TreeView {
            label: "t".into(),
            nodes: spec
                .iter()
                .map(|(id, l, r)| TreeNodeV {
                    id: *id,
                    val: Cell::Num(*id as f64),
                    left: *l,
                    right: *r,
                })
                .collect(),
            root: Some(0),
            ..Default::default()
        }
    }

    const RECT: Rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(600.0, 400.0));

    #[test]
    fn in_order_layout_places_the_left_child_left_of_its_parent() {
        let t = tree(&[(0, Some(1), Some(2)), (1, None, None), (2, None, None)]);
        let l = tree_layout(&t, RECT, &Theme::dark());
        assert!(l[&1].x < l[&0].x);
        assert!(l[&2].x > l[&0].x);
        assert!(l[&1].y > l[&0].y, "children sit below their parent");
    }

    #[test]
    fn a_swapped_tree_produces_swapped_positions() {
        let before = tree(&[(0, Some(1), Some(2)), (1, None, None), (2, None, None)]);
        let after = tree(&[(0, Some(2), Some(1)), (1, None, None), (2, None, None)]);
        let theme = Theme::dark();
        let a = tree_layout(&before, RECT, &theme);
        let b = tree_layout(&after, RECT, &theme);
        assert!(a[&1].x < a[&2].x);
        assert!(
            b[&1].x > b[&2].x,
            "the children traded x positions, so they animate across"
        );
    }

    #[test]
    fn a_cyclic_tree_does_not_hang_the_layout() {
        // Content is data and data can be wrong; the renderer must survive it.
        let bad = TreeView {
            label: "t".into(),
            nodes: vec![
                TreeNodeV {
                    id: 0,
                    val: Cell::Num(0.0),
                    left: Some(1),
                    right: None,
                },
                TreeNodeV {
                    id: 1,
                    val: Cell::Num(1.0),
                    left: Some(0),
                    right: None,
                },
            ],
            root: Some(0),
            ..Default::default()
        };
        let l = tree_layout(&bad, RECT, &Theme::dark());
        assert_eq!(l.len(), 2);
        assert_eq!(tree_depth(&bad), 2);
    }

    #[test]
    fn interpolated_position_starts_at_the_old_spot_and_ends_at_the_new_one() {
        let mut old = Layout::new();
        let mut now = Layout::new();
        old.insert(7, Pos2::new(0.0, 0.0));
        now.insert(7, Pos2::new(100.0, 0.0));
        assert_eq!(at(&now, &old, 7, 0.0).x, 0.0);
        assert_eq!(at(&now, &old, 7, 1.0).x, 100.0);
        // A node with no previous position appears where it belongs.
        assert_eq!(at(&now, &Layout::new(), 7, 0.0).x, 100.0);
    }

    #[test]
    fn graph_layout_is_deterministic_and_honours_explicit_coordinates() {
        let v = GraphView {
            label: "g".into(),
            nodes: vec![
                GraphNodeV {
                    id: 0,
                    label: "a".into(),
                    x: Some(0.0),
                    y: Some(0.0),
                },
                GraphNodeV {
                    id: 1,
                    label: "b".into(),
                    x: None,
                    y: None,
                },
            ],
            ..Default::default()
        };
        let theme = Theme::dark();
        let a = graph_layout(&v, RECT, &theme);
        let b = graph_layout(&v, RECT, &theme);
        assert_eq!(a, b, "stepping back and forth must not reshuffle the graph");
        assert!(a[&0].x < a[&1].x);
    }

    #[test]
    fn grid_cells_shrink_for_large_grids() {
        let theme = Theme::dark();
        let small = grid_cell_size(3, 3, 600.0, &theme);
        let wide = grid_cell_size(3, 40, 600.0, &theme);
        let tall = grid_cell_size(40, 3, 600.0, &theme);
        assert!(wide < small && tall < small);
        assert!(wide >= 14.0 && tall >= 14.0);
    }
}
