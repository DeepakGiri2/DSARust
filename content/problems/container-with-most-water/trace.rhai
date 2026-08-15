// Container With Most Water — why moving the shorter wall is always safe.
//
// Drawn as bars so the container is literally visible. The key beat is the
// `which` step: keeping the shorter wall can never help, because the width
// only shrinks from here and the height is already capped by that wall.

fn trace(input) {
    let h = input.heights;
    let n = h.len();
    let best = 0;
    let best_l = 0;
    let best_r = n - 1;

    let l = 0;
    let r = n - 1;

    enter("maxArea", #{ heights: A(h) });
    set(#{ l: N(l), r: N(r) });
    step("init", "Start with the widest possible container: the first and last walls.",
        [ bars("heights", h).pointers(#{ l: l, r: r }) ]);

    set(#{ best: N(best) });
    step("best", "No water held yet.", [ bars("heights", h).pointers(#{ l: l, r: r }) ]);

    while l < r {
        step("loop", `Walls ${l} and ${r} are ${r - l} apart.`,
            [ bars("heights", h).pointers(#{ l: l, r: r }).done(dead(n, l, r)) ]);

        let low = if h[l] < h[r] { h[l] } else { h[r] };
        step("hmin", `The water level is set by the shorter wall: min(${h[l]}, ${h[r]}) = ${low}.`,
            [ bars("heights", h).pointers(#{ l: l, r: r }).hl([if h[l] < h[r] { l } else { r }])
                .done(dead(n, l, r)) ]);
        step("pick", `Height ${low}, width ${r - l}.`,
            [ bars("heights", h).pointers(#{ l: l, r: r }).hl([l, r]).done(dead(n, l, r)) ]);

        let area = low * (r - l);
        set(#{ area: N(area) });
        log(`area = ${low} * ${r - l} = ${area}`);
        step("area", `This container holds ${low} × ${r - l} = ${area}.`,
            [ bars("heights", h).pointers(#{ l: l, r: r }).hl([l, r]).done(dead(n, l, r)) ]);

        if area > best {
            step("cmp", `${area} beats the best so far (${best}).`,
                [ bars("heights", h).pointers(#{ l: l, r: r }).hl([l, r]).done(dead(n, l, r)) ]);
            best = area;
            best_l = l;
            best_r = r;
            set(#{ best: N(best) });
            log(`new best ${best} between walls ${l} and ${r}`, "result");
            step("update", `Record ${best}.`,
                [ bars("heights", h).pointers(#{ l: l, r: r }).hl([l, r]).done(dead(n, l, r)) ]);
        } else {
            step("cmp", `${area} does not beat ${best}.`,
                [ bars("heights", h).pointers(#{ l: l, r: r }).hl([best_l, best_r])
                    .done(dead(n, l, r)) ]);
        }

        if h[l] < h[r] {
            step("which",
                `Wall ${l} (height ${h[l]}) is the shorter one. Keeping it can only ever give a narrower container of the same or lower height, so it is safe to drop.`,
                [ bars("heights", h).pointers(#{ l: l, r: r }).bad([l]).done(dead(n, l, r)) ]);
            l += 1;
            set(#{ l: N(l) });
            step("moveL", `Move l to ${l}.`,
                [ bars("heights", h).pointers(#{ l: l, r: r }).done(dead(n, l, r)) ]);
        } else {
            step("which", `Wall ${r} (height ${h[r]}) is the shorter one, so it is the one to drop.`,
                [ bars("heights", h).pointers(#{ l: l, r: r }).bad([r]).done(dead(n, l, r)) ]);
            r -= 1;
            set(#{ r: N(r) });
            step("moveR", `Move r to ${r}.`,
                [ bars("heights", h).pointers(#{ l: l, r: r }).done(dead(n, l, r)) ]);
        }
    }

    log(`answer ${best}`, "result");
    result(`${best}`);
    step("ret", `The most water any container can hold is ${best}, between walls ${best_l} and ${best_r}.`,
        [ bars("heights", h).hl([best_l, best_r]) ], "return");
    leave(`${best}`);
}
