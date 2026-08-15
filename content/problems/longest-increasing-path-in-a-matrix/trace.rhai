fn parse_rows(spec) {
    let out = [];
    for row in words(spec) {
        let cells = [];
        for v in row.split(",") { cells.push(num(v)); }
        out.push(cells);
    }
    out
}

// Longest Increasing Path — strictly increasing means no cycles, so memoise.
//
// Every legal move goes to a strictly larger value, so a path can never return
// to where it has been. The move graph is therefore acyclic: no visited set is
// needed, and — more importantly — the answer for a cell is a fixed property
// of the matrix, not of how the search arrived there. That is precisely the
// condition under which caching a DFS result is sound.

fn solved(memo) {
    let out = [];
    for r in 0..memo.len() {
        for c in 0..memo[0].len() {
            if memo[r][c] != 0 { out.push([r, c]); }
        }
    }
    out
}

fn dfs(m, memo, r, c) {
    enter(`dfs(${r},${c}) — value ${m[r][c]}`, #{ value: N(m[r][c]) });

    if memo[r][c] != 0 {
        step("cached", `(${r},${c}) was solved before: ${memo[r][c]}. Because the path must increase, that answer cannot depend on how we got here.`,
            [ grid("matrix", m).cur([r, c]), grid("longest path from each cell", memo).cur([r, c]) ]);
        step("retCached", `Return ${memo[r][c]} without expanding anything.`,
            [ grid("longest path from each cell", memo).cur([r, c]).hl([[r, c]]) ], "return");
        leave(`${memo[r][c]}`);
        return [memo, memo[r][c]];
    }

    let best = 1;
    step("start", "A path can always consist of just this cell, so 1 is the floor.",
        [ grid("matrix", m).cur([r, c]).hl([[r, c]]),
          grid("longest path from each cell", memo).done(solved(memo)) ]);

    let dirs = [[1, 0], [-1, 0], [0, 1], [0, -1]];
    for d in dirs {
        let nr = r + d[0];
        let nc = c + d[1];
        step("neighbours", `Looking at (${nr},${nc}).`,
            [ grid("matrix", m).cur([r, c]) ]);

        if nr >= 0 && nr < m.len() && nc >= 0 && nc < m[0].len() && m[nr][nc] > m[r][c] {
            step("higher", `${m[nr][nc]} > ${m[r][c]}, so the path may continue there — and can never come back, since values only rise.`,
                [ grid("matrix", m).cur([r, c]).hl([[nr, nc]]) ]);

            step("recurse", `How far does the path go from (${nr},${nc})?`,
                [ grid("matrix", m).cur([nr, nc]) ], "call");
            let out = dfs(m, memo, nr, nc);
            memo = out[0];
            let l = 1 + out[1];

            if l > best {
                step("better", `${l} beats the ${best} found so far from here.`,
                    [ grid("longest path from each cell", memo).cur([r, c]).done(solved(memo)) ]);
                best = l;
                step("take", `Best from (${r},${c}) is now ${best}.`,
                    [ grid("matrix", m).cur([r, c]).hl([[r, c], [nr, nc]]) ]);
            } else {
                step("better", `${l} does not beat ${best}.`,
                    [ grid("longest path from each cell", memo).cur([r, c]).done(solved(memo)) ]);
            }
        } else {
            step("higher",
                if nr < 0 || nr >= m.len() || nc < 0 || nc >= m[0].len() {
                    "That is off the matrix.".to_string()
                } else {
                    `${m[nr][nc]} is not greater than ${m[r][c]}, so the path cannot go that way.`
                },
                [ grid("matrix", m).cur([r, c]) ]);
        }
    }

    grid_set(memo, r, c, best);
    log(`(${r},${c}) -> ${best}`);
    step("memo", `Record ${best} for (${r},${c}). Every later query reads it instead of re-expanding.`,
        [ grid("longest path from each cell", memo).cur([r, c]).done(solved(memo)) ]);
    step("retBest", `Return ${best}.`,
        [ grid("longest path from each cell", memo).cur([r, c]) ], "return");
    leave(`${best}`);
    [memo, best]
}

fn trace(input) {
    let m = parse_rows(input.matrix);
    let rows = m.len();
    let cols = m[0].len();

    enter("longestIncreasingPath", #{ rows: N(rows), cols: N(cols) });

    let memo = [];
    for r in 0..rows {
        let line = [];
        for c in 0..cols { line.push(0); }
        memo.push(line);
    }
    step("init", "One cached answer per cell; 0 means not solved yet.",
        [ grid("matrix", m), grid("longest path from each cell", memo) ]);

    let best = 0;
    set(#{ best: N(best) });
    step("best", "Any cell could start the longest path, so every one is tried — but each is only ever expanded once.",
        [ grid("matrix", m) ]);

    for r in 0..rows {
        step("rowLoop", `Row ${r}.`, [ grid("matrix", m).cur([r, 0]) ]);
        for c in 0..cols {
            step("colLoop", `Starting from (${r},${c}).`,
                [ grid("matrix", m).cur([r, c]),
                  grid("longest path from each cell", memo).done(solved(memo)) ]);

            step("call", `Ask how long the increasing path from (${r},${c}) is.`,
                [ grid("matrix", m).cur([r, c]) ], "call");
            let out = dfs(m, memo, r, c);
            memo = out[0];

            if out[1] > best {
                step("cmp", `${out[1]} beats the best so far, ${best}.`,
                    [ grid("longest path from each cell", memo).done(solved(memo)) ]);
                best = out[1];
                set(#{ best: N(best) });
                log(`best ${best}`);
                step("update", `Longest path is now ${best}.`,
                    [ grid("longest path from each cell", memo).done(solved(memo)) ]);
            } else {
                step("cmp", `${out[1]} does not beat ${best}.`,
                    [ grid("longest path from each cell", memo).done(solved(memo)) ]);
            }
        }
    }

    log(`answer ${best}`, "result");
    result(`${best}`);
    step("ret", `The longest strictly increasing path has ${plural(best, "cell")}.`,
        [ grid("matrix", m), grid("longest path from each cell", memo) ], "return");
    leave(`${best}`);
}
