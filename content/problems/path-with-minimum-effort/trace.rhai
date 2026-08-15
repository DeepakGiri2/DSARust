fn parse_rows(spec) {
    let out = [];
    for row in words(spec) {
        let cells = [];
        for v in row.split(",") { cells.push(num(v)); }
        out.push(cells);
    }
    out
}

// Path With Minimum Effort — the same max-Dijkstra, weighted differently.
//
// A route is only as bad as its worst single climb, so combining a route with
// a new edge takes the larger of the two rather than the sum. Dijkstra's
// argument survives because that combination is still monotone: adding a step
// can never make a route easier. Here an edge weighs the height difference
// between the two cells, so a flat route costs 0 however long it is.

fn abs_of(v) { if v < 0 { -v } else { v } }

fn effort_view(e) {
    let g = [];
    for row in e {
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

fn frontier_cells(e, visited) {
    let out = [];
    for r in 0..e.len() {
        for c in 0..e[0].len() {
            if !visited[r][c] && e[r][c] < 1000000 { out.push([r, c]); }
        }
    }
    out
}

fn trace(input) {
    let h = parse_rows(input.heights);
    let rows = h.len();
    let cols = h[0].len();

    enter("minimumEffortPath", #{ rows: N(rows), cols: N(cols) });

    let effort = [];
    let visited = [];
    for r in 0..rows {
        let e = [];
        let v = [];
        for c in 0..cols { e.push(1000000); v.push(false); }
        effort.push(e);
        visited.push(v);
    }
    step("init", "effort[r][c] is the smallest possible worst-step of any route reaching that cell.",
        [ grid("heights", h), grid("minimum effort to reach", effort_view(effort)) ]);
    step("visited", "Settled cells are final.",
        [ grid("minimum effort to reach", effort_view(effort)) ]);

    grid_set(effort, 0, 0, 0);
    step("seed", "Standing still costs no effort at all.",
        [ grid("minimum effort to reach", effort_view(effort)).hl([[0, 0]]) ]);

    let answer = -1;
    for iter in 0..(rows * cols) {
        step("loop", `Round ${iter + 1}.`,
            [ grid("minimum effort to reach", effort_view(effort)).done(visited_cells(visited))
                .hl(frontier_cells(effort, visited)) ]);

        let br = -1;
        let bc = -1;
        step("pick", "Which reachable, unsettled cell has the smallest effort?",
            [ grid("minimum effort to reach", effort_view(effort)).done(visited_cells(visited)) ]);

        for r in 0..rows {
            for c in 0..cols {
                if !visited[r][c] && effort[r][c] < 1000000 &&
                    (br < 0 || effort[r][c] < effort[br][bc]) {
                    br = r;
                    bc = c;
                }
            }
        }
        step("scan", `${plural(frontier_cells(effort, visited).len(), "cell")} on the frontier.`,
            [ grid("minimum effort to reach", effort_view(effort)).done(visited_cells(visited))
                .hl(frontier_cells(effort, visited)) ]);

        if br < 0 {
            step("stuck", "Nothing reachable remains.",
                [ grid("minimum effort to reach", effort_view(effort)) ]);
            break;
        }

        step("lowest", `(${br},${bc}) at effort ${effort[br][bc]} is the smallest, so no other route to it can be gentler.`,
            [ grid("heights", h).cur([br, bc]),
              grid("minimum effort to reach", effort_view(effort)).cur([br, bc])
                  .done(visited_cells(visited)) ]);

        if br == rows - 1 && bc == cols - 1 {
            answer = effort[br][bc];
            log(`minimum effort ${answer}`, "result");
            step("arrived", "That is the destination.",
                [ grid("minimum effort to reach", effort_view(effort)).cur([br, bc]) ]);
            step("ret", `The gentlest route has effort ${answer}.`,
                [ grid("heights", h).done(visited_cells(visited)).hl([[br, bc]]) ], "return");
            break;
        }

        visited[br][bc] = true;
        step("settle", `Settle (${br},${bc}).`,
            [ grid("minimum effort to reach", effort_view(effort)).done(visited_cells(visited)) ]);

        let dirs = [[1, 0], [-1, 0], [0, 1], [0, -1]];
        for d in dirs {
            let nr = br + d[0];
            let nc = bc + d[1];
            if nr < 0 || nr >= rows || nc < 0 || nc >= cols || visited[nr][nc] { continue; }

            step("relax", `Step to (${nr},${nc}).`,
                [ grid("heights", h).cur([br, bc]).hl([[nr, nc]]) ]);

            let diff = abs_of(h[nr][nc] - h[br][bc]);
            step("diff", `The climb from ${h[br][bc]} to ${h[nr][nc]} is ${diff}.`,
                [ text("cost", [`this step: ${diff}`]) ]);

            let t = effort[br][bc];
            if diff > t { t = diff; }
            step("worst", `A route is only as hard as its hardest step, so the route's effort becomes max(${effort[br][bc]}, ${diff}) = ${t}.`,
                [ text("cost", [`route so far: ${effort[br][bc]}`, `this step: ${diff}`,
                                `combined: ${t}`]) ]);

            if t < effort[nr][nc] {
                step("better", `${t} beats the ${if effort[nr][nc] >= 1000000 { "nothing" } else { `${effort[nr][nc]}` }} known for that cell.`,
                    [ grid("minimum effort to reach", effort_view(effort)).cur([nr, nc]) ]);
                grid_set(effort, nr, nc, t);
                log(`(${nr},${nc}) at effort ${t}`);
                step("update", `(${nr},${nc}) now costs ${t}.`,
                    [ grid("minimum effort to reach", effort_view(effort)).cur([nr, nc])
                        .hl(frontier_cells(effort, visited)) ]);
            } else {
                step("better", `${t} is no better than the ${effort[nr][nc]} already known.`,
                    [ grid("minimum effort to reach", effort_view(effort)).cur([nr, nc]) ]);
            }
        }
    }

    if answer < 0 {
        log("unreachable", "result");
        step("none", "The destination was never reached.",
            [ grid("minimum effort to reach", effort_view(effort)) ], "return");
    }

    result(`${answer}`);
    leave(`${answer}`);
}
