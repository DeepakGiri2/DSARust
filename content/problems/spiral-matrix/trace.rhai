fn parse_rows(spec) {
    let out = [];
    for row in words(spec) {
        let cells = [];
        for v in row.split(",") { cells.push(num(v)); }
        out.push(cells);
    }
    out
}

fn render_rows(g) {
    let out = [];
    for row in g { out.push(join(row, ",")); }
    join(out, " ")
}

// Spiral Matrix — four walls closing in, not four directions.
//
// Tracking a heading and turning at obstacles needs a visited map. Tracking
// four boundaries needs nothing: each side, once walked, is retired by moving
// its wall inwards, so the next walk cannot reach it. The two `if` checks in
// the middle exist for the degenerate end-game — with one row left, walking
// back along the "bottom" would re-read the row just walked.

fn trace(input) {
    let g = parse_rows(input.matrix);
    let rows = g.len();
    let cols = g[0].len();
    let out = [];
    let done = [];

    enter("spiralOrder", #{ rows: N(rows), cols: N(cols) });
    step("init", "The values, in the order they come off the spiral.",
        [ grid("matrix", g), array("output", out) ]);

    let top = 0;
    let bottom = rows - 1;
    let left = 0;
    let right = cols - 1;
    set(#{ top: N(top), bottom: N(bottom), left: N(left), right: N(right) });
    step("bounds", "Four walls. Everything strictly inside them is still unvisited.",
        [ grid("matrix", g),
          text("walls", [`top ${top}, bottom ${bottom}`, `left ${left}, right ${right}`]) ]);

    while top <= bottom && left <= right {
        step("loop", `Still a live rectangle: rows ${top}..${bottom}, columns ${left}..${right}.`,
            [ grid("matrix", g).done(done),
              text("walls", [`top ${top}, bottom ${bottom}`, `left ${left}, right ${right}`]) ]);

        let c = left;
        while c <= right {
            out.push(g[top][c]);
            done.push([top, c]);
            c += 1;
        }
        step("goRight", `Walk the top row left to right: ${join(slice_row(g, top, left, right), ", ")}.`,
            [ grid("matrix", g).cur([top, right]).done(done), array("output", out) ]);

        top += 1;
        set(#{ top: N(top) });
        step("shrinkTop", `Retire that row — the top wall drops to ${top}, so nothing can walk it again.`,
            [ grid("matrix", g).done(done),
              text("walls", [`top ${top}, bottom ${bottom}`, `left ${left}, right ${right}`]) ]);

        let r = top;
        while r <= bottom {
            out.push(g[r][right]);
            done.push([r, right]);
            r += 1;
        }
        step("goDown", `Walk the right column downwards.`,
            [ grid("matrix", g).cur([bottom, right]).done(done), array("output", out) ]);

        right -= 1;
        set(#{ right: N(right) });
        step("shrinkRight", `Right wall moves in to ${right}.`,
            [ grid("matrix", g).done(done),
              text("walls", [`top ${top}, bottom ${bottom}`, `left ${left}, right ${right}`]) ]);

        if top <= bottom {
            step("checkRow", `Rows ${top}..${bottom} still exist, so there is a genuine bottom row to walk back along.`,
                [ grid("matrix", g).done(done) ]);
            let c2 = right;
            while c2 >= left {
                out.push(g[bottom][c2]);
                done.push([bottom, c2]);
                c2 -= 1;
            }
            step("goLeft", "Walk the bottom row right to left.",
                [ grid("matrix", g).cur([bottom, left]).done(done), array("output", out) ]);
            bottom -= 1;
            set(#{ bottom: N(bottom) });
            step("shrinkBottom", `Bottom wall moves up to ${bottom}.`,
                [ grid("matrix", g).done(done),
                  text("walls", [`top ${top}, bottom ${bottom}`, `left ${left}, right ${right}`]) ]);
        } else {
            step("checkRow", "The walls have crossed vertically — the row just walked was the last one, so walking back along it would repeat those values.",
                [ grid("matrix", g).done(done) ]);
        }

        if left <= right {
            step("checkCol", `Columns ${left}..${right} still exist, so there is a genuine left column to climb.`,
                [ grid("matrix", g).done(done) ]);
            let r2 = bottom;
            while r2 >= top {
                out.push(g[r2][left]);
                done.push([r2, left]);
                r2 -= 1;
            }
            step("goUp", "Climb the left column.",
                [ grid("matrix", g).cur([top, left]).done(done), array("output", out) ]);
            left += 1;
            set(#{ left: N(left) });
            step("shrinkLeft", `Left wall moves in to ${left}.`,
                [ grid("matrix", g).done(done),
                  text("walls", [`top ${top}, bottom ${bottom}`, `left ${left}, right ${right}`]) ]);
        } else {
            step("checkCol", "The walls have crossed horizontally, so there is no column left to climb.",
                [ grid("matrix", g).done(done) ]);
        }
    }

    let answer = join(out, " ");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `The walls met, so every cell has been read exactly once: ${answer}.`,
        [ grid("matrix", g).done(done), array("output", out) ], "return");
    leave(answer);
}

fn slice_row(g, r, lo, hi) {
    let out = [];
    let c = lo;
    while c <= hi { out.push(g[r][c]); c += 1; }
    out
}
