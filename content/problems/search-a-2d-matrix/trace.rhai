// Search a 2D Matrix — one sorted list that happens to be stored in rows.
//
// Because each row starts above where the previous one ended, reading the
// matrix left to right, top to bottom gives a sorted sequence. So the search
// runs over a flat index and only converts to (row, col) when it needs to look
// at a value.

// "1,3,5,7 10,11,16,20" -> rows of numbers. The grid input type splits per
// character, which cannot express multi-digit values.
fn parse_rows(spec) {
    let out = [];
    for row in words(spec) {
        let cells = [];
        for v in row.split(",") { cells.push(num(v)); }
        out.push(cells);
    }
    out
}

fn cells_of(grid) {
    let out = [];
    for row in grid { for v in row { out.push(v); } }
    out
}

fn flat_dead(n, lo, hi) {
    dead(n, lo, hi)
}

fn trace(input) {
    let grid = parse_rows(input.matrix);
    let target = input.target;
    let rows = grid.len();
    let cols = grid[0].len();
    let flat = cells_of(grid);
    let n = flat.len();

    let lo = 0;
    let hi = n - 1;

    enter("searchMatrix", #{ target: N(target) });
    set(#{ lo: N(lo), hi: N(hi) });
    step("init", `${rows} × ${cols} = ${n} values, and reading them row by row gives a sorted list.`,
        [ grid("matrix", grid), array("as one sorted list", flat).window(lo, hi) ]);

    while lo <= hi {
        step("loop", `The value, if present, is in flat range [${lo}..${hi}].`,
            [ array("as one sorted list", flat).pointers(#{ lo: lo, hi: hi })
                .window(lo, hi).done(flat_dead(n, lo, hi)) ]);

        let mid = lo + (hi - lo) / 2;
        set(#{ mid: N(mid) });
        step("mid", `mid = ${mid}.`,
            [ array("as one sorted list", flat).pointers(#{ lo: lo, mid: mid, hi: hi })
                .window(lo, hi).done(flat_dead(n, lo, hi)) ]);

        let r = mid / cols;
        let c = mid % cols;
        set(#{ r: N(r), c: N(c) });
        step("map", `Flat index ${mid} is row ${mid} / ${cols} = ${r}, column ${mid} % ${cols} = ${c}.`,
            [ grid("matrix", grid).cur([r, c]),
              array("as one sorted list", flat).pointers(#{ mid: mid }).hl([mid]) ]);

        let v = flat[mid];
        step("read", `That cell holds ${v}.`,
            [ grid("matrix", grid).cur([r, c]).hl([[r, c]]),
              array("as one sorted list", flat).hl([mid]) ]);

        if v == target {
            log(`found at (${r},${c})`, "result");
            step("cmp", `${v} is the target.`,
                [ grid("matrix", grid).hl([[r, c]]), array("as one sorted list", flat).hl([mid]) ]);
            result("true");
            step("found", "Return true.", [ grid("matrix", grid).hl([[r, c]]) ], "return");
            leave("true");
            return;
        }

        if v < target {
            step("less", `${v} < ${target}, so everything up to ${mid} is too small.`,
                [ array("as one sorted list", flat).pointers(#{ lo: lo, mid: mid, hi: hi })
                    .bad(span(lo, mid)).window(lo, hi) ]);
            lo = mid + 1;
            set(#{ lo: N(lo) });
            step("right", `lo = ${lo}.`,
                [ array("as one sorted list", flat).pointers(#{ lo: lo, hi: hi })
                    .window(lo, hi).done(flat_dead(n, lo, hi)) ]);
        } else {
            step("less", `${v} > ${target}, so everything from ${mid} on is too big.`,
                [ array("as one sorted list", flat).pointers(#{ lo: lo, mid: mid, hi: hi })
                    .bad(span(mid, hi)).window(lo, hi) ]);
            hi = mid - 1;
            set(#{ hi: N(hi) });
            step("left", `hi = ${hi}.`,
                [ array("as one sorted list", flat).pointers(#{ lo: lo, hi: hi })
                    .window(lo, hi).done(flat_dead(n, lo, hi)) ]);
        }
    }

    log("not present", "result");
    result("false");
    step("none", `The range is empty, so ${target} is not in the matrix — reached in about log2(${n}) probes.`,
        [ grid("matrix", grid) ], "return");
    leave("false");
}
