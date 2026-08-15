//! The debugger itself: a cursor over a recorded [`Trace`] plus the clock that
//! drives the animation between two adjacent steps.
//!
//! It is pure state — no timers, no GUI. The host calls [`Timeline::tick`] once
//! per frame with the elapsed seconds and reads [`Timeline::transition`] to
//! interpolate the picture. That keeps stepping logic unit-testable and makes
//! the same code usable from a terminal front-end or a headless exporter.

use crate::model::Trace;
use std::collections::BTreeSet;

/// Seconds a single visual transition takes when stepping by hand.
const MANUAL_ANIM: f32 = 0.22;
/// Longest a transition may take during playback, whatever the speed.
const MAX_PLAY_ANIM: f32 = 0.45;
/// Base dwell between auto-advanced steps at speed 1.0.
const BASE_DWELL: f32 = 0.9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepMode {
    In,
    Over,
    Out,
    Back,
}

#[derive(Clone, Debug)]
pub struct Timeline {
    len: usize,
    idx: usize,
    /// Step the current transition is animating *from*.
    from: usize,
    /// 0..=1 progress of that transition.
    t: f32,
    anim_secs: f32,
    playing: bool,
    dwell_left: f32,
    /// Playback multiplier; also shortens the transition so fast playback does
    /// not turn into a queue of unfinished animations.
    pub speed: f32,
    /// Master switch — off makes every transition instant.
    pub animate: bool,
    pub breakpoints: BTreeSet<usize>,
}

impl Default for Timeline {
    fn default() -> Self {
        Self {
            len: 0,
            idx: 0,
            from: 0,
            t: 1.0,
            anim_secs: MANUAL_ANIM,
            playing: false,
            dwell_left: 0.0,
            speed: 1.0,
            animate: true,
            breakpoints: BTreeSet::new(),
        }
    }
}

impl Timeline {
    pub fn new(len: usize) -> Self {
        Self {
            len,
            ..Default::default()
        }
    }

    /// Point the timeline at a different trace, preserving user preferences
    /// (speed, animation toggle) but dropping position and breakpoints.
    pub fn reset(&mut self, len: usize) {
        let (speed, animate) = (self.speed, self.animate);
        *self = Self {
            len,
            speed,
            animate,
            ..Default::default()
        };
    }

    /// Re-target after an input edit, keeping the cursor where it was if the
    /// new trace is long enough. Authoring feels much better this way: tweak an
    /// input, keep watching the same part of the algorithm.
    pub fn retarget(&mut self, len: usize) {
        let keep = self.idx.min(len.saturating_sub(1));
        self.len = len;
        self.idx = keep;
        self.from = keep;
        self.t = 1.0;
        self.playing = false;
        self.breakpoints.retain(|b| *b < len);
    }

    pub fn idx(&self) -> usize {
        self.idx
    }
    pub fn from_idx(&self) -> usize {
        self.from
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn playing(&self) -> bool {
        self.playing
    }
    pub fn at_end(&self) -> bool {
        self.len == 0 || self.idx + 1 >= self.len
    }
    pub fn at_start(&self) -> bool {
        self.idx == 0
    }

    /// Eased 0..=1 progress of the current transition. `1.0` means settled.
    pub fn transition(&self) -> f32 {
        ease_out_cubic(self.t)
    }

    /// Raw (un-eased) progress, for effects that want linear time.
    pub fn transition_linear(&self) -> f32 {
        self.t
    }

    pub fn settled(&self) -> bool {
        self.t >= 1.0
    }

    /// True while the visualization still needs repainting.
    pub fn needs_repaint(&self) -> bool {
        self.playing || !self.settled()
    }

    // ── movement ────────────────────────────────────────────────────────────

    fn goto(&mut self, target: usize, animated: bool) {
        if self.len == 0 {
            return;
        }
        let target = target.min(self.len - 1);
        if target == self.idx {
            return;
        }
        self.from = self.idx;
        self.idx = target;
        // Jumping more than one step (scrub, continue-to-breakpoint) has no
        // meaningful in-between picture, so it snaps.
        let adjacent = target.abs_diff(self.from) == 1;
        self.t = if self.animate && animated && adjacent {
            0.0
        } else {
            1.0
        };
        self.anim_secs = if self.playing {
            (BASE_DWELL / self.speed * 0.6).min(MAX_PLAY_ANIM)
        } else {
            MANUAL_ANIM
        };
    }

    /// Advance one recorded step, descending into calls.
    pub fn step_in(&mut self) {
        self.playing = false;
        self.goto(self.idx + 1, true);
    }

    pub fn step_back(&mut self) {
        self.playing = false;
        self.goto(self.idx.saturating_sub(1), true);
    }

    /// Next step at the same depth or shallower — i.e. run nested calls to
    /// completion without showing them.
    pub fn step_over(&mut self, trace: &Trace) {
        self.playing = false;
        let d = trace.get(self.idx).map(|s| s.depth).unwrap_or(0);
        let target = (self.idx + 1..self.len)
            .find(|j| trace.steps[*j].depth <= d)
            .unwrap_or(self.len.saturating_sub(1));
        self.goto(target, target == self.idx + 1);
    }

    /// Run until the current frame returns.
    pub fn step_out(&mut self, trace: &Trace) {
        self.playing = false;
        let d = trace.get(self.idx).map(|s| s.depth).unwrap_or(0);
        let target = (self.idx + 1..self.len)
            .find(|j| trace.steps[*j].depth < d)
            .unwrap_or(self.len.saturating_sub(1));
        self.goto(target, target == self.idx + 1);
    }

    pub fn step(&mut self, mode: StepMode, trace: &Trace) {
        match mode {
            StepMode::In => self.step_in(),
            StepMode::Over => self.step_over(trace),
            StepMode::Out => self.step_out(trace),
            StepMode::Back => self.step_back(),
        }
    }

    /// VS Code style continue: run to the next breakpoint, else to the end.
    pub fn continue_run(&mut self) {
        self.playing = false;
        let target = self
            .breakpoints
            .range(self.idx + 1..)
            .next()
            .copied()
            .unwrap_or(self.len.saturating_sub(1));
        self.goto(target, false);
    }

    pub fn restart(&mut self) {
        self.playing = false;
        self.goto(0, false);
        self.t = 1.0;
    }

    pub fn to_end(&mut self) {
        self.playing = false;
        self.goto(self.len.saturating_sub(1), false);
    }

    /// Scrubber drag — always instant, the user is driving the picture.
    pub fn jump_to(&mut self, i: usize) {
        self.playing = false;
        self.goto(i, false);
    }

    pub fn toggle_play(&mut self) {
        if self.at_end() {
            // Play from a finished trace restarts rather than doing nothing.
            self.restart();
            self.playing = true;
        } else {
            self.playing = !self.playing;
        }
        self.dwell_left = 0.0;
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn toggle_breakpoint(&mut self, i: usize) {
        if !self.breakpoints.remove(&i) {
            self.breakpoints.insert(i);
        }
    }

    /// Breakpoints are set on source *tags*, so toggling one marks every step
    /// that lands on that line.
    pub fn toggle_breakpoint_tag(&mut self, tag: &str, trace: &Trace) {
        let hits: Vec<usize> = trace
            .steps
            .iter()
            .enumerate()
            .filter(|(_, s)| s.tag == tag)
            .map(|(i, _)| i)
            .collect();
        let any = hits.iter().any(|i| self.breakpoints.contains(i));
        for i in hits {
            if any {
                self.breakpoints.remove(&i);
            } else {
                self.breakpoints.insert(i);
            }
        }
    }

    pub fn has_breakpoint_tag(&self, tag: &str, trace: &Trace) -> bool {
        self.breakpoints
            .iter()
            .any(|i| trace.steps.get(*i).is_some_and(|s| s.tag == tag))
    }

    // ── clock ───────────────────────────────────────────────────────────────

    /// Drive the animation and autoplay. `dt` is seconds since the last frame.
    /// Returns true when the picture changed and a repaint is needed.
    pub fn tick(&mut self, dt: f32) -> bool {
        let mut dirty = false;
        let dt = dt.clamp(0.0, 0.25); // survive a stalled frame without a jump

        if self.t < 1.0 {
            self.t = (self.t + dt / self.anim_secs.max(0.001)).min(1.0);
            dirty = true;
        }

        if self.playing {
            if self.at_end() {
                self.playing = false;
                return true;
            }
            self.dwell_left -= dt;
            if self.dwell_left <= 0.0 && self.t >= 1.0 {
                let next = self.idx + 1;
                self.goto(next, true);
                self.dwell_left = BASE_DWELL / self.speed.max(0.05);
                if self.breakpoints.contains(&next) {
                    self.playing = false;
                }
                dirty = true;
            }
        }
        dirty
    }
}

fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Step, StepEvent};

    fn trace_with_depths(depths: &[usize]) -> Trace {
        Trace {
            steps: depths
                .iter()
                .enumerate()
                .map(|(i, d)| Step {
                    tag: format!("t{i}"),
                    depth: *d,
                    event: StepEvent::Stmt,
                    frames: vec![],
                    views: vec![],
                    note: String::new(),
                    log_len: 0,
                })
                .collect(),
            logs: vec![],
            result: None,
        }
    }

    #[test]
    fn step_over_skips_the_nested_call() {
        //            0  1  2  3  4
        let tr = trace_with_depths(&[1, 2, 3, 3, 1]);
        let mut tl = Timeline::new(tr.len());
        tl.step_over(&tr); // from depth 1 -> next depth <= 1
        assert_eq!(tl.idx(), 4);
    }

    #[test]
    fn step_out_leaves_the_current_frame() {
        let tr = trace_with_depths(&[1, 2, 3, 2, 1]);
        let mut tl = Timeline::new(tr.len());
        tl.jump_to(2); // depth 3
        tl.step_out(&tr);
        assert_eq!(tl.idx(), 3, "depth 3 -> first step with depth < 3");
    }

    #[test]
    fn step_over_at_the_deepest_point_runs_to_end() {
        let tr = trace_with_depths(&[1, 2, 3]);
        let mut tl = Timeline::new(tr.len());
        tl.jump_to(2);
        tl.step_over(&tr);
        assert_eq!(tl.idx(), 2);
        assert!(tl.at_end());
    }

    #[test]
    fn continue_run_stops_at_the_next_breakpoint() {
        let tr = trace_with_depths(&[1; 10]);
        let mut tl = Timeline::new(tr.len());
        tl.breakpoints.insert(3);
        tl.breakpoints.insert(7);
        tl.continue_run();
        assert_eq!(tl.idx(), 3);
        tl.continue_run();
        assert_eq!(tl.idx(), 7);
        tl.continue_run();
        assert_eq!(tl.idx(), 9, "no breakpoint left -> end");
    }

    #[test]
    fn playback_pauses_when_it_hits_a_breakpoint() {
        let tr = trace_with_depths(&[1; 5]);
        let mut tl = Timeline::new(tr.len());
        tl.breakpoints.insert(1);
        tl.speed = 100.0; // dwell ~ 9ms
        tl.toggle_play();
        for _ in 0..20 {
            tl.tick(0.016);
        }
        assert_eq!(tl.idx(), 1);
        assert!(!tl.playing());
    }

    #[test]
    fn adjacent_steps_animate_but_jumps_snap() {
        let tr = trace_with_depths(&[1; 20]);
        let mut tl = Timeline::new(tr.len());
        tl.step_in();
        assert!(!tl.settled(), "single step should tween");
        tl.jump_to(15);
        assert!(tl.settled(), "scrub should be instant");
    }

    #[test]
    fn transition_completes_in_bounded_time() {
        let tr = trace_with_depths(&[1; 4]);
        let mut tl = Timeline::new(tr.len());
        tl.step_in();
        let mut elapsed = 0.0;
        while !tl.settled() && elapsed < 2.0 {
            tl.tick(0.016);
            elapsed += 0.016;
        }
        assert!(tl.settled());
        assert!(elapsed <= MANUAL_ANIM + 0.05, "took {elapsed}s");
    }

    #[test]
    fn retarget_keeps_position_when_possible() {
        let mut tl = Timeline::new(50);
        tl.jump_to(30);
        tl.retarget(40);
        assert_eq!(tl.idx(), 30);
        tl.retarget(10);
        assert_eq!(tl.idx(), 9, "clamped into the shorter trace");
    }

    #[test]
    fn tag_breakpoints_toggle_every_matching_step() {
        let tr = trace_with_depths(&[1, 1, 1, 1]);
        let mut tl = Timeline::new(tr.len());
        tl.toggle_breakpoint_tag("t2", &tr);
        assert!(tl.breakpoints.contains(&2));
        assert!(tl.has_breakpoint_tag("t2", &tr));
        tl.toggle_breakpoint_tag("t2", &tr);
        assert!(tl.breakpoints.is_empty());
    }

    #[test]
    fn play_from_the_end_restarts() {
        let tr = trace_with_depths(&[1; 3]);
        let mut tl = Timeline::new(tr.len());
        tl.to_end();
        tl.toggle_play();
        assert_eq!(tl.idx(), 0);
        assert!(tl.playing());
    }
}
