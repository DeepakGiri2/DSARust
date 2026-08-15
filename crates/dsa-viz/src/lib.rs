//! The animated visualization layer.
//!
//! Given the current step's views, the previous step's views and a progress
//! value in `0..=1`, this crate draws the in-between frame. Nothing here knows
//! about problems, files or the debugger — it draws two snapshots and the
//! motion between them, which is what turns a sequence of pictures into
//! something you can actually watch an algorithm happen in.

pub mod anim;
pub mod draw;
pub mod linear;
pub mod linked;
pub mod theme;

pub use anim::pair;
pub use draw::CellState;
pub use theme::Theme;

use dsa_core::model::VizView;
use egui::{Sense, Ui, Vec2};

/// Height a view needs at the given width.
pub fn view_height(view: &VizView, width: f32, theme: &Theme) -> f32 {
    match view {
        VizView::Array(v) => linear::array_height(v, theme),
        VizView::Kv(v) => linear::kv_height(v, width, theme),
        VizView::Stack(v) => linear::stack_height(v, theme),
        VizView::Bits(v) => linear::bits_height(v, theme),
        VizView::Text(v) => linear::text_height(v, theme),
        VizView::List(_) => linked::list_height(theme),
        VizView::Tree(v) => linked::tree_height(v, theme),
        VizView::Grid(v) => linked::grid_height(v, width, theme),
        VizView::Graph(_) => linked::graph_height(),
    }
}

/// Draw every view of the current step, tweening from the previous step.
///
/// `t` is the eased transition progress: `0.0` shows the previous frame,
/// `1.0` the settled current one.
pub fn show_views(ui: &mut Ui, cur: &[VizView], prev: Option<&[VizView]>, t: f32, theme: &Theme) {
    let width = ui.available_width();
    if cur.is_empty() {
        ui.add_space(8.0);
        ui.weak("This step draws no picture.");
        return;
    }

    for (view, before) in pair(prev, cur) {
        let h = view_height(view, width, theme);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, h), Sense::hover());
        if !ui.is_rect_visible(rect) {
            continue;
        }
        let p = ui.painter_at(rect);

        match (view, before) {
            (VizView::Array(v), b) => linear::draw_array(&p, rect, v, as_array(b), t, theme),
            (VizView::Kv(v), b) => linear::draw_kv(&p, rect, v, as_kv(b), t, theme),
            (VizView::Stack(v), b) => linear::draw_stack(&p, rect, v, as_stack(b), t, theme),
            (VizView::Bits(v), b) => linear::draw_bits(&p, rect, v, as_bits(b), t, theme),
            (VizView::Text(v), _) => linear::draw_text(&p, rect, v, theme),
            (VizView::List(v), b) => linked::draw_list(&p, rect, v, as_list(b), t, theme),
            (VizView::Tree(v), b) => linked::draw_tree(&p, rect, v, as_tree(b), t, theme),
            (VizView::Grid(v), b) => linked::draw_grid(&p, rect, v, as_grid(b), t, theme),
            (VizView::Graph(v), b) => linked::draw_graph(&p, rect, v, as_graph(b), t, theme),
        }
        ui.add_space(10.0);
    }
}

// `pair` already guarantees the kinds match; these just unwrap that guarantee.
macro_rules! caster {
    ($name:ident, $variant:ident, $ty:ty) => {
        fn $name(v: Option<&VizView>) -> Option<&$ty> {
            match v {
                Some(VizView::$variant(x)) => Some(x),
                _ => None,
            }
        }
    };
}

use dsa_core::model::{
    ArrayView, BitsView, GraphView, GridView, KvView, LinkedListView, StackView, TreeView,
};
caster!(as_array, Array, ArrayView);
caster!(as_kv, Kv, KvView);
caster!(as_stack, Stack, StackView);
caster!(as_bits, Bits, BitsView);
caster!(as_list, List, LinkedListView);
caster!(as_tree, Tree, TreeView);
caster!(as_grid, Grid, GridView);
caster!(as_graph, Graph, GraphView);

#[cfg(test)]
mod tests {
    use super::*;
    use dsa_core::model::{ArrayView, Cell, GridView};

    #[test]
    fn every_view_kind_reports_a_usable_height() {
        let theme = Theme::dark();
        let views = vec![
            VizView::Array(ArrayView {
                label: "a".into(),
                data: vec![Cell::Num(1.0)],
                ..Default::default()
            }),
            VizView::Kv(Default::default()),
            VizView::Stack(Default::default()),
            VizView::List(Default::default()),
            VizView::Tree(Default::default()),
            VizView::Grid(GridView {
                label: "g".into(),
                data: vec![vec![Cell::Num(0.0); 3]; 3],
                ..Default::default()
            }),
            VizView::Graph(Default::default()),
            VizView::Bits(Default::default()),
            VizView::Text(Default::default()),
        ];
        for v in &views {
            let h = view_height(v, 600.0, &theme);
            assert!(h > 10.0 && h < 700.0, "{:?} -> {h}", v.anim_key());
        }
    }

    #[test]
    fn casters_refuse_a_mismatched_kind() {
        let kv = VizView::Kv(Default::default());
        assert!(as_array(Some(&kv)).is_none());
        assert!(as_kv(Some(&kv)).is_some());
        assert!(as_array(None).is_none());
    }
}
