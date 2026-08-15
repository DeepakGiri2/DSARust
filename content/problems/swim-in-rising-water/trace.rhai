fn parse_rows(spec) {
    let out = [];
    for row in words(spec) {
        let cells = [];
        for v in row.split(",") { cells.push(num(v)); }
        out.push(cells);
    }
    out
}

// Swim in Rising Water — Dijkstra where the path cost is a maximum.
//
// The cost of a route is how long you must wait for its highest cell, not the
// total of its cells. Dijkstra's proof only needs the path cost to be
// non-decreasing as a path grows, and a maximum satisfies that just as a sum
// does — so the algorithm carries over unchanged except for one line: combine
// with max instead of plus.

fn best_view(best) {
    let g = [];
    for row in best {
        let line = [];
        for v in row { line.push(if v >= 1000000 { "∞" } else { `${v}` }); }
        g.push(line);
    }
    g
}

fn visited_cells(visited) {
    let out = [];
    for r in 0..visited.len() {
        for c in 0..visited[0].len() { if visited[r][c] { out.push([r, c]); } }
    }
    out
}

fn frontier_cells(best, visited) {
    let out = [];
    for r in 0..best.len() {
        for c in 0..best[0].len() {
            if !visited[r][c] && best[r][c] < 1000000 { out.push([r, c]); }
        }
    }
    out
}

fn trace(input) {
    let g = parse_rows(input.grid);
    let n = g.len();

    enter("swimInWater", #{ n: N(n) });

    let best = [];
    let visited = [];
    for r in 0..n {
        let b = [];
        let v = [];
        for c in 0..n { b.push(1000000); v.push(false); }
        best.push(b);
        visited.push(v);
    }
    step("init", "best[r][c] is the earliest time at which (r,c) can be reached — the lowest achievable maximum along any route to it.",
        [ grid("elevations", g), grid("earliest arrival time", best_view(best)) ]);
    step("visited", "A settled cell's time is final.",
        [ grid("earliest arrival time", best_view(best)) ]);

    grid_set(best, 0, 0, g[0][0]);
    step("seed", `Standing on the start already requires waiting for its own elevation, ${g[0][0]}.`,
        [ grid("earliest arrival time", best_view(best)).hl([[0, 0]]) ]);

    let answer = -1;
    for iter in 0..(n * n) {
        step("loop", `Round ${iter + 1}.`,
            [ grid("earliest arrival time", best_view(best)).done(visited_cells(visited))
                .hl(frontier_cells(best, visited)) ]);

        let br = -1;
        let bc = -1;
        step("pick", "Which reachable, unsettled cell has the smallest arrival time?",
            [ grid("earliest arrival time", best_view(best)).done(visited_cells(visited)) ]);

        for r in 0..n {
            for c in 0..n {
                if !visited[r][c] && best[r][c] < 1000000 && (br < 0 || best[r][c] < best[br][bc]) {
                    br = r;
                    bc = c;
                }
            }
        }
        step("scan", `${plural(frontier_cells(best, visited).len(), "cell")} on the frontier.`,
            [ grid("earliest arrival time", best_view(best)).done(visited_cells(visited))
                .hl(frontier_cells(best, visited)) ]);

        if br < 0 {
            step("stuck", "Nothing reachable is left — the corner cannot be reached at all.",
                [ grid("earliest arrival time", best_view(best)).done(visited_cells(visited)) ]);
            break;
        }

        step("lowest", `(${br},${bc}) at time ${best[br][bc]} is the smallest. Extending any other route could only raise its maximum, so this value is final.`,
            [ grid("elevations", g).cur([br, bc]),
              grid("earliest arrival time", best_view(best)).cur([br, bc])
                  .done(visited_cells(visited)) ]);

        if br == n - 1 && bc == n - 1 {
            answer = best[br][bc];
            log(`reach the corner at ${answer}`, "result");
            step("arrived", "That is the bottom-right corner.",
                [ grid("earliest arrival time", best_view(best)).cur([br, bc]) ]);
            step("ret", `The earliest possible arrival is ${answer}.`,
                [ grid("elevations", g).done(visited_cells(visited)).hl([[br, bc]]) ], "return");
            break;
        }

        visited[br][bc] = true;
        step("settle", `Settle (${br},${bc}).`,
            [ grid("earliest arrival time", best_view(best)).done(visited_cells(visited)) ]);

        let dirs = [[1, 0], [-1, 0], [0, 1], [0, -1]];
        for d in dirs {
            let nr = br + d[0];
            let nc = bc + d[1];
            if nr < 0 || nr >= n || nc < 0 || nc >= n || visited[nr][nc] { continue; }

            step("relax", `Neighbour (${nr},${nc}), elevation ${g[nr][nc]}.`,
                [ grid("elevations", g).cur([br, bc]).hl([[nr, nc]]) ]);

            let t = best[br][bc];
            if g[nr][nc] > t { t = g[nr][nc]; }
            step("level", `Getting there means waiting for whichever is higher: the route so far (${best[br][bc]}) or this cell (${g[nr][nc]}) — so ${t}. Note the max, where an ordinary shortest path would add.`,
                [ text("cost", [`route so far: ${best[br][bc]}`, `this cell: ${g[nr][nc]}`,
                                `combined: ${t}`]) ]);

            if t < best[nr][nc] {
                step("better", `${t} beats the ${if best[nr][nc] >= 1000000 { "nothing" } else { `${best[nr][nc]}` }} known for that cell.`,
                    [ grid("earliest arrival time", best_view(best)).cur([nr, nc]) ]);
                grid_set(best, nr, nc, t);
                log(`(${nr},${nc}) reachable at ${t}`);
                step("update", `(${nr},${nc}) is reachable at ${t}.`,
                    [ grid("earliest arrival time", best_view(best)).cur([nr, nc])
                        .hl(frontier_cells(best, visited)) ]);
            } else {
                step("better", `${t} is no better than the ${best[nr][nc]} already known.`,
                    [ grid("earliest arrival time", best_view(best)).cur([nr, nc]) ]);
            }
        }
    }

    if answer < 0 {
        log("unreachable", "result");
        step("none", "The corner was never reached.",
            [ grid("earliest arrival time", best_view(best)) ], "return");
    }

    result(`${answer}`);
    leave(`${answer}`);
}
