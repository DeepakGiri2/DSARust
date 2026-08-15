// Flood Fill — the repaint is the visited mark.
//
// No separate visited grid is needed: once a pixel has been given the new
// colour it no longer matches the old one, so the entry test rejects it. That
// economy is exactly why the "already that colour" case has to be caught
// before the recursion starts — otherwise nothing would ever change, nothing
// would ever be rejected, and the search would not terminate.

fn render(g) {
    let out = [];
    for row in g { out.push(join(row, "")); }
    join(out, " ")
}

fn fill(g, r, c, from, to, painted) {
    if r < 0 || r >= g.len() || c < 0 || c >= g[0].len() || g[r][c] != from {
        enter(`fill(${r},${c})`);
        step("bounds",
            if r < 0 || r >= g.len() || c < 0 || c >= g[0].len() {
                `(${r},${c}) is off the image.`
            } else {
                `(${r},${c}) holds ${g[r][c]}, not ${from} — either a different region, or already repainted.`
            },
            [ grid("image", g).done(painted) ]);
        step("stop", "Nothing to do here.", [ grid("image", g).done(painted) ], "return");
        leave("");
        return [g, painted];
    }

    enter(`fill(${r},${c})`, #{ r: N(r), c: N(c) });
    step("bounds", `(${r},${c}) still holds the original colour ${from}, so it belongs to the region.`,
        [ grid("image", g).cur([r, c]).hl([[r, c]]).done(painted) ]);

    grid_set(g, r, c, to);
    painted.push([r, c]);
    log(`paint (${r},${c})`);
    step("paint", `Repaint it ${to}. From now on it fails the test above, which is what stops it being visited twice.`,
        [ grid("image", g).cur([r, c]).done(painted) ]);

    let out = fill(g, r + 1, c, from, to, painted);
    g = out[0];
    painted = out[1];
    step("down", "Down explored.", [ grid("image", g).cur([r, c]).done(painted) ]);

    out = fill(g, r - 1, c, from, to, painted);
    g = out[0];
    painted = out[1];
    step("up", "Up explored.", [ grid("image", g).cur([r, c]).done(painted) ]);

    out = fill(g, r, c + 1, from, to, painted);
    g = out[0];
    painted = out[1];
    step("right", "Right explored.", [ grid("image", g).cur([r, c]).done(painted) ]);

    out = fill(g, r, c - 1, from, to, painted);
    g = out[0];
    painted = out[1];
    step("left", "Left explored — this pixel is finished.",
        [ grid("image", g).done(painted) ], "return");

    leave("");
    [g, painted]
}

fn validate(input) {
    let g = grid_rows(input.image);
    if input.sr < 0 || input.sr >= g.len() || input.sc < 0 || input.sc >= g[0].len() {
        return `(${input.sr},${input.sc}) is outside the image`;
    }
}

fn trace(input) {
    let g = grid_rows(input.image);
    let sr = input.sr;
    let sc = input.sc;
    let to = `${input.color}`;

    enter("floodFill", #{ sr: N(sr), sc: N(sc) });

    let from = g[sr][sc];
    step("start", `The starting pixel is ${from}, so that is the colour the region is made of.`,
        [ grid("image", g).cur([sr, sc]).hl([[sr, sc]]) ]);

    if from == to {
        step("same", `It is already ${to}. Repainting would change nothing — and because the repaint doubles as the visited mark, the recursion would never stop.`,
            [ grid("image", g).cur([sr, sc]) ]);
        let answer = render(g);
        log(`answer ${answer}`, "result");
        result(answer);
        step("ret", "Return the image unchanged.", [ grid("image", g) ], "return");
        leave(answer);
        return;
    }
    step("same", `${from} is not ${to}, so there is real work to do.`,
        [ grid("image", g).cur([sr, sc]) ]);

    step("call", "Flood outwards from the start.", [ grid("image", g).cur([sr, sc]) ], "call");
    let out = fill(g, sr, sc, from, to, []);
    g = out[0];

    let answer = render(g);
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `${plural(out[1].len(), "pixel")} repainted.`,
        [ grid("image", g).done(out[1]) ], "return");
    leave(answer);
}
