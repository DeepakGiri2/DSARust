//! The visual language, ported from `src/styles.css`.
//!
//! The web version's look is carried over deliberately: same palette, same
//! pill/segment/chip vocabulary, same spacing rhythm. These helpers exist so
//! every screen builds its controls the same way instead of each one
//! re-deriving what a "mini button" looks like.

use egui::{
    epaint::Shadow, pos2, text::LayoutJob, vec2, Align, Color32, CornerRadius, FontId, Mesh, Pos2,
    Rect, Response, RichText, Sense, Shape, Stroke, TextFormat, Ui, Vec2,
};

// --- :root, verbatim -------------------------------------------------------
pub const BG: Color32 = Color32::from_rgb(0x0b, 0x0e, 0x14);
pub const PANEL: Color32 = Color32::from_rgb(0x11, 0x15, 0x1f);
pub const PANEL2: Color32 = Color32::from_rgb(0x16, 0x1b, 0x28);
pub const BORDER: Color32 = Color32::from_rgb(0x23, 0x2a, 0x3b);
pub const TEXT: Color32 = Color32::from_rgb(0xd6, 0xdb, 0xe8);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x8b, 0x93, 0xa7);
pub const ACCENT: Color32 = Color32::from_rgb(0x7c, 0x6c, 0xff);
pub const ACCENT2: Color32 = Color32::from_rgb(0x22, 0xd3, 0xee);
pub const GREEN: Color32 = Color32::from_rgb(0x34, 0xd3, 0x99);
pub const RED: Color32 = Color32::from_rgb(0xf8, 0x71, 0x71);
pub const AMBER: Color32 = Color32::from_rgb(0xfb, 0xbf, 0x24);

/// The pink is not part of the ported stylesheet. It exists only so the three
/// lights in the backdrop read as a *spectrum* rather than a violet-to-cyan
/// ramp — two lights of neighbouring hue look like one badly lit one.
pub const PINK: Color32 = Color32::from_rgb(0xf4, 0x72, 0xb6);

/// The lights behind the page, in paint order.
pub const AURORA: [Color32; 3] = [ACCENT, ACCENT2, PINK];

/// `#34d39922` — the 13% tint the difficulty pills use as a background.
pub fn tint(c: Color32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 0x22)
}

/// The same colour at an arbitrary alpha, kept unpremultiplied at the call
/// site so a reader can see the alpha they asked for.
pub fn alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

pub fn difficulty_color(d: dsa_core::problem::Difficulty) -> Color32 {
    match d {
        dsa_core::problem::Difficulty::Easy => GREEN,
        dsa_core::problem::Difficulty::Medium => AMBER,
        dsa_core::problem::Difficulty::Hard => RED,
    }
}

/// Console lines carry a kind, and the kinds mean genuinely different things:
/// call and return are scaffolding, a result is the payoff a trace was built to
/// reach. Painting them all one colour — which is what the console did — buries
/// the `log(text, "result")` that every trace ends on among the enter/leave
/// noise.
pub fn log_color(kind: dsa_core::model::LogKind) -> Color32 {
    use dsa_core::model::LogKind::*;
    match kind {
        Call => ACCENT,
        Return => ACCENT2,
        Result => GREEN,
        Log => TEXT,
    }
}

/// The gutter glyph for a console line, standing in for the `-> ` / `<- `
/// prefixes the recorder writes into the text itself.
pub fn log_glyph(kind: dsa_core::model::LogKind) -> &'static str {
    use dsa_core::model::LogKind::*;
    match kind {
        Call => "→",
        Return => "←",
        Result => "✔",
        Log => "·",
    }
}

/// The line without the recorder's arrow prefix, since [`log_glyph`] now draws
/// that in its own colour and showing both reads as a stutter.
pub fn log_text(entry: &dsa_core::model::LogEntry) -> &str {
    entry
        .text
        .strip_prefix("-> ")
        .or_else(|| entry.text.strip_prefix("<- "))
        .unwrap_or(&entry.text)
}

/// Apply the palette to egui's own widget visuals so scrollbars, text fields
/// and menus match the hand-painted parts.
pub fn install(ctx: &egui::Context) {
    install_fonts(ctx);

    // The palette is a dark one, ported from a dark stylesheet. Without this,
    // egui follows the OS preference: on a machine set to Light it switches to
    // its own light theme slot — which this app never fills in — and the page
    // comes out white behind hand-painted dark widgets.
    ctx.set_theme(egui::ThemePreference::Dark);

    let mut v = egui::Visuals::dark();
    // Panels are translucent so the backdrop's lights tint every screen, not
    // just the list. Nested panels stack their alpha, which is the effect you
    // want anyway: the deeper a surface, the more solid it reads.
    v.panel_fill = alpha(BG, 0xd0);
    v.window_fill = PANEL;
    v.extreme_bg_color = PANEL2;
    v.faint_bg_color = PANEL2;
    v.override_text_color = Some(TEXT);
    v.widgets.noninteractive.bg_fill = PANEL;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.inactive.bg_fill = PANEL2;
    v.widgets.inactive.weak_bg_fill = PANEL2;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.hovered.bg_fill = PANEL2;
    v.widgets.hovered.weak_bg_fill = PANEL2;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.active.bg_fill = ACCENT;
    v.widgets.active.weak_bg_fill = ACCENT;
    v.selection.bg_fill = ACCENT.gamma_multiply(0.45);
    v.selection.stroke = Stroke::new(1.0, TEXT);
    v.window_corner_radius = CornerRadius::same(10);
    v.widgets.inactive.corner_radius = CornerRadius::same(6);
    v.widgets.hovered.corner_radius = CornerRadius::same(6);
    v.widgets.active.corner_radius = CornerRadius::same(6);
    // A widget that grows a hair under the pointer is the cheapest possible
    // "this is clickable" signal, and it costs no layout.
    v.widgets.hovered.expansion = 1.0;
    v.widgets.active.expansion = 1.0;
    // The default shadows are near-black and read as smudges over a dark
    // background; tinting them with the accent makes a raised surface look lit.
    v.window_shadow = Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(0x66),
    };
    v.popup_shadow = v.window_shadow;
    ctx.set_visuals(v);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.button_padding = Vec2::new(10.0, 4.0);
    // The default scrollbar is a wide grey slab that fights the palette.
    style.spacing.scroll.bar_width = 9.0;
    style.spacing.scroll.floating = true;
    style.spacing.scroll.foreground_color = false;
    ctx.set_style(style);
}

/// Let proportional text fall back to Hack.
///
/// egui's proportional family is Ubuntu-Light and two emoji fonts; Hack is
/// registered but reachable only from the monospace family. Ubuntu-Light has no
/// arrows, no geometric shapes and no box drawing, so every `←`, `→`, `●` and
/// `▸` in a label — the back button, the transport hints, the list markers —
/// rendered as an empty box. Appending Hack *after* the emoji fonts keeps emoji
/// coming from the fonts drawn for them and only catches what nothing else has.
fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .push("Hack".to_owned());
    ctx.set_fonts(fonts);
}

// ─────────────────────────────────────────────────────────────────────────────
// The page backdrop
// ─────────────────────────────────────────────────────────────────────────────

/// A vertical two-stop gradient as a single mesh.
///
/// egui has no gradient brush, but it does interpolate vertex colours, so a
/// two-triangle mesh *is* a gradient — and one shape rather than the fifty
/// stacked strips the obvious version needs.
pub fn vertical_gradient(painter: &egui::Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    painter.add(Shape::mesh(mesh));
}

/// A soft round light: a triangle fan with a coloured centre and a fully
/// transparent rim.
///
/// Stacking translucent circles is the usual trick and it bands visibly on a
/// dark background — each ring edge is a step. A fan interpolates, so the
/// falloff is smooth, and it is one shape instead of twenty.
pub fn radial_glow(painter: &egui::Painter, center: Pos2, radius: f32, color: Color32) {
    const SEGMENTS: u32 = 48;
    let mut mesh = Mesh::default();
    mesh.colored_vertex(center, color);
    for i in 0..=SEGMENTS {
        let a = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        // Transparent *black*: egui blends premultiplied, so fading toward
        // (0,0,0,0) is the only rim that does not darken the halo on its way out.
        mesh.colored_vertex(center + Vec2::angled(a) * radius, Color32::TRANSPARENT);
    }
    for i in 1..=SEGMENTS {
        mesh.add_triangle(0, i, i + 1);
    }
    painter.add(Shape::mesh(mesh));
}

/// Paint the page background: a wash, a hairline grid that dissolves as it
/// falls, and three slow coloured lights.
///
/// Call it *first* in `update`, before any panel is added: it fills the whole
/// window and never scrolls, so the content slides over a fixed sky. The
/// panels above it are translucent (see [`install`]), which is what lets the
/// lights tint the debugger and the practice editor too rather than stopping
/// at the home screen.
///
/// `animated` drives the drift. With it off the lights are frozen at a fixed
/// phase rather than removed, so the page looks the same, minus the repaint.
pub fn backdrop(ctx: &egui::Context, animated: bool) {
    let rect = ctx.content_rect();
    if rect.width() < 2.0 || rect.height() < 2.0 {
        return;
    }
    // The background layer, before any panel has added its own frame: panels
    // are drawn into the same layer and shapes are painted in insertion order,
    // so everything the app puts on screen this frame lands on top of this.
    let painter = ctx.layer_painter(egui::LayerId::background());
    let t = if animated {
        ctx.input(|i| i.time) as f32
    } else {
        11.0
    };
    let (w, h) = (rect.width(), rect.height());

    // 1. Lift the top of the page off pure black so the header has some air.
    vertical_gradient(
        &painter,
        rect,
        Color32::from_rgb(0x13, 0x16, 0x26),
        Color32::from_rgb(0x0a, 0x0c, 0x12),
    );

    // 2. A hairline lattice. Constant alpha across the whole page would read as
    //    graph paper; the fade in step 3 turns it into a horizon instead.
    let grid = alpha(ACCENT, 22);
    let step = 76.0;
    let mut x = rect.left() + step;
    while x < rect.right() {
        painter.line_segment(
            [pos2(x, rect.top()), pos2(x, rect.bottom())],
            Stroke::new(1.0, grid),
        );
        x += step;
    }
    let mut y = rect.top() + step;
    while y < rect.bottom() {
        painter.line_segment(
            [pos2(rect.left(), y), pos2(rect.right(), y)],
            Stroke::new(1.0, grid),
        );
        y += step;
    }

    // 3. Dissolve everything above into flat background on the way down, so
    //    long lists end on something calm and text stays legible over it.
    vertical_gradient(&painter, rect, Color32::TRANSPARENT, BG);

    // 4. Three lights, drifting on periods that do not divide each other — the
    //    pattern never visibly loops. Painted last so they survive the fade.
    let blobs: [(f32, f32, f32, f32, f32, u8, usize); 4] = [
        (0.10, -0.04, 0.62, 0.021, 0.013, 52, 0),
        (0.92, 0.10, 0.50, 0.017, 0.011, 46, 1),
        (0.62, 0.86, 0.56, 0.009, 0.015, 30, 2),
        (0.34, 0.44, 0.34, 0.013, 0.019, 22, 1),
    ];
    for (fx, fy, fr, sx, sy, a, idx) in blobs {
        let center = rect.left_top()
            + vec2(
                w * (fx + 0.06 * (t * sx * std::f32::consts::TAU).sin()),
                h * (fy + 0.09 * (t * sy * std::f32::consts::TAU).cos()),
            );
        radial_glow(&painter, center, w.max(h) * fr, alpha(AURORA[idx], a));
    }
}

/// `.hero h1 em` — the accent→cyan gradient on "Visualized". egui has no
/// gradient fill for text, so the ramp is applied per character.
pub fn gradient_heading(ui: &mut Ui, plain: &str, gradient: &str, size: f32) {
    let mut job = LayoutJob::default();
    let font = FontId::proportional(size);
    job.append(
        plain,
        0.0,
        TextFormat {
            font_id: font.clone(),
            color: TEXT,
            ..Default::default()
        },
    );
    let n = gradient.chars().count().max(1) as f32;
    for (i, ch) in gradient.chars().enumerate() {
        let t = i as f32 / (n - 1.0).max(1.0);
        let c = Color32::from_rgb(
            lerp_u8(ACCENT.r(), ACCENT2.r(), t),
            lerp_u8(ACCENT.g(), ACCENT2.g(), t),
            lerp_u8(ACCENT.b(), ACCENT2.b(), t),
        );
        job.append(
            &ch.to_string(),
            0.0,
            TextFormat {
                font_id: font.clone(),
                color: c,
                ..Default::default()
            },
        );
    }
    ui.label(job);
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t.clamp(0.0, 1.0)).round() as u8
}

/// `.seg` — a segmented control. Returns true when the selection changed.
///
/// The inner `horizontal` is load-bearing, not decoration: a `Frame` inherits
/// the caller's layout direction, so a segmented control built in a vertical
/// panel — the AI assistant's mode picker, for one — silently came out as a
/// stack of three buttons sitting on top of the control above it.
///
/// It inherits *direction* as well, which is why the options are walked
/// backwards under a right-to-left layout. The problem header lays its controls
/// out right-to-left so they hug the window's right edge, and that reversed
/// every segmented control inside it: the language picker read "Java Go C++"
/// and the tabs read "Visualize Practice". Reversing the walk rather than
/// forcing the layout keeps the control hugging its contents — a forced
/// `left_to_right` claims the whole remaining width and shoves its neighbours
/// off the strip.
pub fn seg<T: PartialEq + Clone>(ui: &mut Ui, current: &mut T, options: &[(T, &str)]) -> bool {
    let mut changed = false;
    egui::Frame::default()
        .fill(PANEL2)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::same(1))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                let reversed = ui.layout().prefer_right_to_left();
                let order: Vec<&(T, &str)> = if reversed {
                    options.iter().rev().collect()
                } else {
                    options.iter().collect()
                };
                for (value, label) in order {
                    let on = *current == *value;
                    if seg_button(ui, label, on).clicked() && !on {
                        *current = value.clone();
                        changed = true;
                    }
                }
            });
        });
    changed
}

fn seg_button(ui: &mut Ui, label: &str, on: bool) -> Response {
    let text =
        RichText::new(label)
            .size(13.0)
            .strong()
            .color(if on { Color32::WHITE } else { TEXT_DIM });
    let btn = egui::Button::new(text)
        .fill(if on { ACCENT } else { Color32::TRANSPARENT })
        .stroke(Stroke::NONE)
        .corner_radius(CornerRadius::same(7))
        .min_size(Vec2::new(0.0, 24.0));
    ui.add(btn)
}

/// `.mini-btn` — the quiet bordered button used for secondary actions.
pub fn mini_btn(ui: &mut Ui, label: &str) -> Response {
    ui.add(
        egui::Button::new(RichText::new(label).size(12.0).color(TEXT_DIM))
            .fill(PANEL2)
            .stroke(Stroke::new(1.0, BORDER))
            .corner_radius(CornerRadius::same(6)),
    )
}

/// A `.mini-btn` that can light up, like the `📄 question` and `💬 AI assist`
/// toggles.
pub fn toggle_btn(ui: &mut Ui, label: &str, on: bool) -> Response {
    ui.add(
        egui::Button::new(RichText::new(label).size(12.0).color(if on {
            Color32::WHITE
        } else {
            TEXT_DIM
        }))
        .fill(if on { ACCENT } else { PANEL2 })
        .stroke(Stroke::new(1.0, if on { ACCENT } else { BORDER }))
        .corner_radius(CornerRadius::same(6)),
    )
}

/// `.apply-btn` — the primary action.
pub fn apply_btn(ui: &mut Ui, label: &str, enabled: bool) -> Response {
    ui.add_enabled(
        enabled,
        egui::Button::new(
            RichText::new(label)
                .size(13.0)
                .strong()
                .color(Color32::WHITE),
        )
        .fill(ACCENT)
        .stroke(Stroke::NONE)
        .corner_radius(CornerRadius::same(7)),
    )
}

/// `.diff` — the rounded difficulty pill.
pub fn diff_pill(ui: &mut Ui, difficulty: dsa_core::problem::Difficulty) {
    let c = difficulty_color(difficulty);
    let label = match difficulty {
        dsa_core::problem::Difficulty::Easy => "Easy",
        dsa_core::problem::Difficulty::Medium => "Medium",
        dsa_core::problem::Difficulty::Hard => "Hard",
    };
    pill(ui, label, c, tint(c));
}

pub fn pill(ui: &mut Ui, label: &str, fg: Color32, bg: Color32) {
    egui::Frame::default()
        .fill(bg)
        .corner_radius(CornerRadius::same(255))
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(label).size(11.0).strong().color(fg));
        });
}

/// `.side-head` — the small label above a panel section.
///
/// Every side panel in the app uses this, so the accent tick is the cheapest
/// way to give STDIN / OUTPUT / TESTS / call stack a shared vocabulary: at a
/// glance you can see where one section ends and the next begins, which a run
/// of identical grey words could not do.
pub fn side_head(ui: &mut Ui, label: &str) {
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let (tick, _) = ui.allocate_exact_size(vec2(3.0, 11.0), Sense::hover());
        ui.painter()
            .rect_filled(tick, CornerRadius::same(2), alpha(ACCENT2, 0xcc));
        ui.label(
            RichText::new(label)
                .size(10.5)
                .strong()
                .color(alpha(TEXT_DIM, 0xff)),
        );
    });
    ui.add_space(3.0);
}

/// `.side-empty` — the muted italic placeholder inside an empty panel.
pub fn side_empty(ui: &mut Ui, text: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(text).size(12.0).italics().color(TEXT_DIM));
}

/// A bordered container, the shape most panels in the reference use.
pub fn card(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    egui::Frame::default()
        .fill(alpha(PANEL, 0xe6))
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::same(12))
        .show(ui, add);
}

/// The strip a screen's controls sit on: translucent, so the backdrop tints it,
/// with a hairline underneath to separate it from the work area.
pub fn toolbar(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    egui::Frame::default()
        .fill(alpha(PANEL, 0xcc))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(10, 7))
        .show(ui, |ui| {
            ui.horizontal_wrapped(add);
        });
}

/// Monospace block used for code samples and program output.
pub fn code_block(ui: &mut Ui, text: &str, color: Color32) {
    egui::Frame::default()
        .fill(alpha(PANEL2, 0xf0))
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(egui::Margin::same(9))
        .show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down(Align::LEFT), |ui| {
                ui.label(RichText::new(text).monospace().size(12.0).color(color));
            });
        });
}

/// A whole-row clickable surface, used by the problem list.
///
/// `accent` colours the edge marker and the hover border — the list passes the
/// difficulty colour, which turns the left margin into a scannable stripe of
/// green/amber/red without adding a column.
///
/// Built with `Frame::begin`/`end` rather than `show` because the hover styling
/// has to be decided *after* the contents are laid out, which is the only
/// moment the row's rectangle is known.
pub fn row_frame(ui: &mut Ui, accent: Color32, add: impl FnOnce(&mut Ui)) -> Response {
    let mut prepared = egui::Frame::default()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(CornerRadius::same(9))
        .inner_margin(egui::Margin {
            left: 14,
            right: 12,
            top: 8,
            bottom: 8,
        })
        .begin(ui);

    {
        let cui = &mut prepared.content_ui;
        cui.horizontal(|ui| {
            ui.set_min_width(ui.available_width());
            ui.set_min_height(19.0);
            add(ui);
        });
    }

    let outer = prepared.content_ui.min_rect() + prepared.frame.total_margin();
    let hovered = ui.rect_contains_pointer(outer);
    if hovered {
        prepared.frame.fill = PANEL2;
        prepared.frame.stroke = Stroke::new(1.0, alpha(accent, 0xaa));
        prepared.frame.shadow = Shadow {
            offset: [0, 2],
            blur: 12,
            spread: 0,
            color: alpha(accent, 0x2a),
        };
    }
    let response = prepared.end(ui);

    // The edge marker sits inside the frame's left margin, drawn after it so it
    // reads as part of the border rather than a first column.
    let bar = Rect::from_min_max(
        pos2(outer.left() + 1.0, outer.top() + 5.0),
        pos2(outer.left() + 4.0, outer.bottom() - 5.0),
    );
    ui.painter().rect_filled(
        bar,
        CornerRadius::same(2),
        alpha(accent, if hovered { 0xff } else { 0x80 }),
    );

    ui.interact(outer, response.id.with("row"), Sense::click())
}

/// A category heading: a two-tone bar, the name and a count chip. Call it
/// inside a `horizontal`, and finish the line with [`head_rule`].
pub fn section_head(ui: &mut Ui, title: &str, count: usize) {
    let (bar, _) = ui.allocate_exact_size(vec2(4.0, 20.0), Sense::hover());
    let p = ui.painter();
    // Two halves rather than a gradient mesh: at four points wide the ramp
    // would not be visible anyway, and this keeps the accent pair on screen.
    p.rect_filled(
        Rect::from_min_max(bar.min, pos2(bar.max.x, bar.center().y)),
        CornerRadius {
            nw: 2,
            ne: 2,
            sw: 0,
            se: 0,
        },
        ACCENT,
    );
    p.rect_filled(
        Rect::from_min_max(pos2(bar.min.x, bar.center().y), bar.max),
        CornerRadius {
            nw: 0,
            ne: 0,
            sw: 2,
            se: 2,
        },
        ACCENT2,
    );

    ui.add_space(2.0);
    ui.label(RichText::new(title).size(19.0).strong().color(TEXT));
    pill(ui, &count.to_string(), TEXT_DIM, PANEL2);
}

/// The hairline that finishes a section heading, run from where the heading's
/// widgets stopped out to the right edge — so the sections read as bands
/// rather than a wall of identical rows.
///
/// `from_x` and `y` come from the heading row itself; asking the *parent* for
/// its available rect would measure the whole rest of the scroll area and put
/// the line somewhere down the page.
pub fn head_rule(ui: &Ui, from_x: f32, y: f32) {
    let right = ui.max_rect().right();
    if right - from_x > 32.0 {
        ui.painter().line_segment(
            [pos2(from_x + 8.0, y), pos2(right, y)],
            Stroke::new(1.0, alpha(ACCENT, 0x40)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_palette_matches_the_stylesheet() {
        // These are the values the web version ships; drifting from them is
        // what makes a port stop looking like the thing it ported.
        assert_eq!(BG, Color32::from_rgb(0x0b, 0x0e, 0x14));
        assert_eq!(ACCENT, Color32::from_rgb(0x7c, 0x6c, 0xff));
        assert_eq!(ACCENT2, Color32::from_rgb(0x22, 0xd3, 0xee));
        assert_eq!(GREEN, Color32::from_rgb(0x34, 0xd3, 0x99));
    }

    #[test]
    fn difficulty_colours_follow_the_pills() {
        use dsa_core::problem::Difficulty::*;
        assert_eq!(difficulty_color(Easy), GREEN);
        assert_eq!(difficulty_color(Medium), AMBER);
        assert_eq!(difficulty_color(Hard), RED);
    }

    #[test]
    fn the_pill_tint_is_the_css_alpha() {
        // egui stores colours premultiplied, so the channels come out scaled
        // by the alpha — the composite over a dark panel is what CSS's
        // `#34d39922` produces.
        let t = tint(GREEN);
        assert_eq!(t.a(), 0x22);
        assert!(t.r() < GREEN.r() && t.g() < GREEN.g());
        assert!(t.g() > t.r(), "green stays the dominant channel");
        assert!(tint(RED).r() > tint(RED).g(), "red stays red");
    }

    #[test]
    fn every_log_kind_gets_its_own_colour() {
        use dsa_core::model::LogKind::*;
        // The bug this replaced painted all four the same, which hid the
        // `log(text, "result")` line every trace ends on.
        let all = [
            log_color(Log),
            log_color(Call),
            log_color(Return),
            log_color(Result),
        ];
        for (i, a) in all.iter().enumerate() {
            for b in all.iter().skip(i + 1) {
                assert_ne!(a, b, "log kinds must be distinguishable at a glance");
            }
        }
        assert_eq!(log_color(Result), GREEN, "a result is the payoff");
    }

    #[test]
    fn the_glyph_replaces_the_recorders_arrow_prefix() {
        use dsa_core::model::{LogEntry, LogKind};
        let call = LogEntry {
            step: 0,
            text: "-> enter dfs(1,2)".into(),
            kind: LogKind::Call,
        };
        let ret = LogEntry {
            step: 3,
            text: "<- dfs returns 9".into(),
            kind: LogKind::Return,
        };
        // Otherwise the row reads "→ -> enter dfs(1,2)".
        assert_eq!(log_text(&call), "enter dfs(1,2)");
        assert_eq!(log_text(&ret), "dfs returns 9");
        assert_eq!(log_glyph(LogKind::Call), "→");
        assert_eq!(log_glyph(LogKind::Return), "←");
    }

    #[test]
    fn an_ordinary_log_line_is_left_alone() {
        use dsa_core::model::{LogEntry, LogKind};
        let plain = LogEntry {
            step: 1,
            text: "2 + 1 = 3".into(),
            kind: LogKind::Log,
        };
        assert_eq!(log_text(&plain), "2 + 1 = 3");
    }

    #[test]
    fn the_gradient_ramp_spans_accent_to_accent_two() {
        assert_eq!(lerp_u8(ACCENT.r(), ACCENT2.r(), 0.0), ACCENT.r());
        assert_eq!(lerp_u8(ACCENT.r(), ACCENT2.r(), 1.0), ACCENT2.r());
        let mid = lerp_u8(0, 100, 0.5);
        assert_eq!(mid, 50);
    }
}
