// Trapping Rain Water — two pointers carrying the running maxima.
//
// Water above a bar is min(tallest to its left, tallest to its right) minus
// its own height. The trick is that you never need both maxima: whichever
// side is currently shorter is the side that *decides* the water, so it is
// safe to settle that column and move that pointer.

fn trace(input) {
    let h = input.height;
    let n = h.len();

    let l = 0;
    let r = n - 1;
    let lmax = 0;
    let rmax = 0;
    let water = 0;
    let filled = [];

    enter("trap", #{ height: A(h) });
    set(#{ l: N(l), r: N(r) });
    step("init", "A pointer at each end.", [ bars("height", h).pointers(#{ l: l, r: r }) ]);

    set(#{ lmax: N(lmax), rmax: N(rmax) });
    step("max", "lmax and rmax are the tallest bars seen so far from each side.",
        [ bars("height", h).pointers(#{ l: l, r: r }) ]);

    set(#{ water: N(water) });
    step("water", "No water counted yet.", [ bars("height", h).pointers(#{ l: l, r: r }) ]);

    while l < r {
        step("loop", `Unresolved span [${l}..${r}].`,
            [ bars("height", h).pointers(#{ l: l, r: r }).window(l, r).hl(filled) ]);

        if h[l] < h[r] {
            step("which",
                `The left bar (${h[l]}) is shorter than the right one (${h[r]}). Whatever happens further right, there is already a bar at least ${h[r]} tall over there — so the left side alone decides column ${l}.`,
                [ bars("height", h).pointers(#{ l: l, r: r }).hl([l]).window(l, r) ]);

            if h[l] >= lmax {
                lmax = h[l];
                set(#{ lmax: N(lmax) });
                step("chkL", `${h[l]} is a new left-hand maximum, so nothing can sit on top of it.`,
                    [ bars("height", h).pointers(#{ l: l, r: r }).hl([l]) ]);
                step("setL", `lmax = ${lmax}.`,
                    [ bars("height", h).pointers(#{ l: l, r: r }).window(l, r).hl(filled) ]);
            } else {
                let add = lmax - h[l];
                water += add;
                filled.push(l);
                set(#{ water: N(water) });
                log(`column ${l} holds ${add}`);
                step("chkL", `Bar ${l} is ${h[l]} tall but is walled in by a bar of ${lmax} on its left.`,
                    [ bars("height", h).pointers(#{ l: l, r: r }).hl([l]) ]);
                step("addL", `That column holds ${lmax} − ${h[l]} = ${add}. Total is now ${water}.`,
                    [ bars("height", h).pointers(#{ l: l, r: r }).hl(filled) ]);
            }
            l += 1;
            set(#{ l: N(l) });
            step("moveL", `l = ${l}.`,
                [ bars("height", h).pointers(#{ l: l, r: r }).window(l, r).hl(filled) ]);
        } else {
            step("which", `The right bar (${h[r]}) is the shorter side, so it decides column ${r}.`,
                [ bars("height", h).pointers(#{ l: l, r: r }).hl([r]).window(l, r) ]);

            if h[r] >= rmax {
                rmax = h[r];
                set(#{ rmax: N(rmax) });
                step("chkR", `${h[r]} is a new right-hand maximum.`,
                    [ bars("height", h).pointers(#{ l: l, r: r }).hl([r]) ]);
                step("setR", `rmax = ${rmax}.`,
                    [ bars("height", h).pointers(#{ l: l, r: r }).window(l, r).hl(filled) ]);
            } else {
                let add = rmax - h[r];
                water += add;
                filled.push(r);
                set(#{ water: N(water) });
                log(`column ${r} holds ${add}`);
                step("chkR", `Bar ${r} is ${h[r]} tall, walled in by ${rmax} on its right.`,
                    [ bars("height", h).pointers(#{ l: l, r: r }).hl([r]) ]);
                step("addR", `That column holds ${rmax} − ${h[r]} = ${add}. Total is now ${water}.`,
                    [ bars("height", h).pointers(#{ l: l, r: r }).hl(filled) ]);
            }
            r -= 1;
            set(#{ r: N(r) });
            step("moveR", `r = ${r}.`,
                [ bars("height", h).pointers(#{ l: l, r: r }).window(l, r).hl(filled) ]);
        }
    }

    log(`answer ${water}`, "result");
    result(`${water}`);
    step("ret", `The pointers met — every column has been settled. Total trapped: ${water}.`,
        [ bars("height", h).hl(filled) ], "return");
    leave(`${water}`);
}
