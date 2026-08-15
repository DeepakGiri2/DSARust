// Detect Squares — enumerate diagonals, because each one fixes a whole square.
//
// Enumerating squares directly means guessing a side length and a direction.
// Enumerating *diagonals* does not: for a given query corner, a stored point
// that is the same distance away horizontally and vertically is the opposite
// corner of exactly one axis-aligned square, and the other two corners then
// have no freedom at all. Multiplying the counts of those two corners handles
// duplicate points for free — which is why counts are kept, not a set.

fn abs_of(v) { if v < 0 { -v } else { v } }

fn key_of(x, y) { `(${x},${y})` }

fn grid_of(counts, minx, maxx, miny, maxy) {
    let g = [];
    let y = maxy;
    while y >= miny {
        let row = [];
        let x = minx;
        while x <= maxx {
            let k = key_of(x, y);
            row.push(if map_has(counts, k) { `${map_get(counts, k)}` } else { "·" });
            x += 1;
        }
        g.push(row);
        y -= 1;
    }
    g
}

fn axis_labels(lo, hi) {
    let out = [];
    let v = lo;
    while v <= hi { out.push(`${v}`); v += 1; }
    out
}

fn desc_labels(lo, hi) {
    let out = [];
    let v = hi;
    while v >= lo { out.push(`${v}`); v -= 1; }
    out
}

fn trace(input) {
    // The picture must be a fixed size from the first step, so scan the whole
    // op list up front to find the bounding box.
    let minx = 1000000; let maxx = -1000000;
    let miny = 1000000; let maxy = -1000000;
    for raw in input.ops.split(",") {
        let parts = words(raw);
        if parts.len() < 3 { continue; }
        let x = num(parts[1]);
        let y = num(parts[2]);
        if x < minx { minx = x; }
        if x > maxx { maxx = x; }
        if y < miny { miny = y; }
        if y > maxy { maxy = y; }
    }
    let drawable = (maxx - minx) <= 20 && (maxy - miny) <= 20;
    if !drawable { minx = 0; maxx = 0; miny = 0; maxy = 0; }

    let counts = map_new();
    let pts = [];
    let out = [];

    enter("DetectSquares", #{});
    step("init", "Point multiplicities, not a set — the same point may be added several times, and each copy makes a different square.",
        [ kv("point counts", counts) ]);
    step("list", "The insertion order is kept too, so a query can walk the candidate diagonals.",
        [ array("points added", pts) ]);

    for raw in input.ops.split(",") {
        let parts = words(raw);
        if parts.len() < 3 { continue; }
        let x = num(parts[1]);
        let y = num(parts[2]);

        if parts[0] == "add" {
            enter(`add(${x}, ${y})`, #{ x: N(x), y: N(y) });
            map_inc(counts, key_of(x, y), 1);
            log(`add ${key_of(x, y)}`);
            step("add", `(${x},${y}) has now been added ${plural(map_get(counts, key_of(x, y)), "time")}.`,
                [ kv("point counts", counts).hl([key_of(x, y)]),
                  grid("points", grid_of(counts, minx, maxx, miny, maxy))
                      .labels(desc_labels(miny, maxy), axis_labels(minx, maxx)) ]);
            pts.push(key_of(x, y));
            step("remember", "Remember it in the scan list as well.",
                [ array("points added", pts).hl([pts.len() - 1]) ], "return");
            leave("");
            continue;
        }

        enter(`count(${x}, ${y})`, #{ x: N(x), y: N(y) });
        step("query", `Query corner (${x},${y}).`,
            [ grid("points", grid_of(counts, minx, maxx, miny, maxy))
                  .labels(desc_labels(miny, maxy), axis_labels(minx, maxx))
                  .cur([maxy - y, x - minx]),
              array("points added", pts) ]);

        let total = 0;
        set(#{ total: N(total) });
        step("total", "No squares counted yet.",
            [ array("points added", pts), text("total", [`${total}`]) ]);

        for k in 0..pts.len() {
            let parts2 = split_point(pts[k]);
            let px = parts2[0];
            let py = parts2[1];

            step("scan", `Candidate opposite corner ${k}: (${px},${py}).`,
                [ array("points added", pts).ptr("k", k).hl([k]),
                  grid("points", grid_of(counts, minx, maxx, miny, maxy))
                      .labels(desc_labels(miny, maxy), axis_labels(minx, maxx))
                      .cur([maxy - y, x - minx]) ]);

            if abs_of(px - x) != abs_of(py - y) || px == x || py == y {
                step("diagonal",
                    if px == x || py == y {
                        `It shares a ${if px == x { "column" } else { "row" }} with the query point, so the "square" would be degenerate.`
                    } else {
                        `Horizontal distance ${abs_of(px - x)} but vertical distance ${abs_of(py - y)} — that is a rectangle's diagonal, not a square's.`
                    },
                    [ array("points added", pts).ptr("k", k).bad([k]) ]);
                continue;
            }

            let side = abs_of(px - x);
            step("diagonal", `Both distances are ${side}, so (${px},${py}) and (${x},${y}) are opposite corners of exactly one square — and the other two corners are forced to be (${px},${y}) and (${x},${py}).`,
                [ array("points added", pts).ptr("k", k).hl([k]),
                  grid("points", grid_of(counts, minx, maxx, miny, maxy))
                      .labels(desc_labels(miny, maxy), axis_labels(minx, maxx))
                      .hl([[maxy - y, x - minx], [maxy - py, px - minx],
                           [maxy - y, px - minx], [maxy - py, x - minx]]) ]);

            let c1 = if map_has(counts, key_of(px, y)) { map_get(counts, key_of(px, y)) } else { 0 };
            let c2 = if map_has(counts, key_of(x, py)) { map_get(counts, key_of(x, py)) } else { 0 };
            total += c1 * c2;
            set(#{ total: N(total) });
            if c1 * c2 > 0 { log(`${key_of(px, py)} gives ${c1 * c2} square(s)`); }
            step("corners", `(${px},${y}) exists ${c1} time(s) and (${x},${py}) ${c2} time(s), so this diagonal accounts for ${c1} × ${c2} = ${c1 * c2} square(s). Total ${total}.`,
                [ kv("point counts", counts).hl([key_of(px, y), key_of(x, py)]),
                  text("total", [`${total}`]) ]);
        }

        out.push(`${total}`);
        log(`count(${x}, ${y}) -> ${total}`, "result");
        step("ret", `${plural(total, "square")} can be completed at (${x},${y}).`,
            [ text("output", out) ], "return");
        leave(`${total}`);
    }

    let answer = join(out, " ");
    result(answer);
    leave(answer);
}

fn split_point(k) {
    let inner = k.sub_string(1, k.len() - 2);
    let bits = inner.split(",");
    [num(bits[0]), num(bits[1])]
}
