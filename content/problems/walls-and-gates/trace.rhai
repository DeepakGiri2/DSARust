// Walls and Gates — one BFS from every gate at once.
//
// Seeding the queue with all the gates before any expansion makes them behave
// like a single source at distance 0. BFS then expands strictly in order of
// distance, so the first arrival at a room is necessarily the shortest one —
// which is why "is it still infinite?" is a sufficient visited test and why no
// distance ever has to be improved later.

// -1 = wall, 2147483647 = unreached room.
fn render(g) {
    let out = [];
    for row in g { out.push(join(row, ",")); }
    join(out, " ")
}

fn display(g) {
    let out = [];
    for row in g {
        let line = [];
        for v in row {
            line.push(if v == -1 { "W" } else if v == 2147483647 { "." } else { `${v}` });
        }
        out.push(line);
    }
    out
}

fn queue_labels(q, qi) {
    let out = [];
    for k in qi..q.len() { out.push(`(${q[k][0]},${q[k][1]})`); }
    out
}

fn trace(input) {
    let raw = grid_rows(input.rooms);
    let rows = raw.len();
    let cols = raw[0].len();

    let g = [];
    for r in 0..rows {
        let line = [];
        for c in 0..cols {
            if raw[r][c] == "W" { line.push(-1); }
            else if raw[r][c] == "G" { line.push(0); }
            else { line.push(2147483647); }
        }
        g.push(line);
    }

    enter("wallsAndGates", #{ rows: N(rows), cols: N(cols) });

    let q = [];
    let qi = 0;
    step("init", "One queue, shared by every gate.",
        [ grid("rooms", display(g)), queue("frontier", queue_labels(q, qi)) ]);

    for r in 0..rows {
        step("scanRows", `Scanning row ${r} for gates.`,
            [ grid("rooms", display(g)).cur([r, 0]) ]);
        for c in 0..cols {
            step("scanCols", `(${r},${c}).`, [ grid("rooms", display(g)).cur([r, c]) ]);
            if g[r][c] == 0 {
                step("gate", `A gate at (${r},${c}) — distance 0 by definition.`,
                    [ grid("rooms", display(g)).cur([r, c]).hl([[r, c]]) ]);
                q.push([r, c]);
                step("seed", "Seed the queue with it. Every gate goes in before anything is expanded, which makes them act as one source.",
                    [ queue("frontier", queue_labels(q, qi)) ]);
            }
        }
    }

    let visited = [];
    while qi < q.len() {
        step("loop", `${q.len() - qi} cell(s) on the frontier.`,
            [ grid("rooms", display(g)).done(visited),
              queue("frontier", queue_labels(q, qi)) ]);

        let cur = q[qi];
        qi += 1;
        visited.push(cur);
        step("pop", `Expanding (${cur[0]},${cur[1]}), which is ${g[cur[0]][cur[1]]} step(s) from a gate.`,
            [ grid("rooms", display(g)).cur(cur).done(visited),
              queue("frontier", queue_labels(q, qi)) ]);

        let dirs = [[1, 0], [-1, 0], [0, 1], [0, -1]];
        for d in dirs {
            let nr = cur[0] + d[0];
            let nc = cur[1] + d[1];
            step("neighbours", `Looking at (${nr},${nc}).`,
                [ grid("rooms", display(g)).cur(cur).done(visited) ]);

            if nr >= 0 && nr < rows && nc >= 0 && nc < cols && g[nr][nc] == 2147483647 {
                step("empty", `(${nr},${nc}) is an untouched room. Because BFS expands in distance order, this arrival is the shortest one there will ever be.`,
                    [ grid("rooms", display(g)).cur(cur).hl([[nr, nc]]).done(visited) ]);
                grid_set(g, nr, nc, g[cur[0]][cur[1]] + 1);
                log(`(${nr},${nc}) is ${g[nr][nc]} from a gate`);
                step("setDist", `Its distance is ${g[nr][nc]}.`,
                    [ grid("rooms", display(g)).cur([nr, nc]).done(visited) ]);
                q.push([nr, nc]);
                step("push", "Queue it for expansion.",
                    [ queue("frontier", queue_labels(q, qi)) ]);
            } else {
                step("empty",
                    if nr < 0 || nr >= rows || nc < 0 || nc >= cols {
                        "Off the grid.".to_string()
                    } else if g[nr][nc] == -1 {
                        "A wall."
                    } else {
                        `Already ${g[nr][nc]} from a gate — and that is at least as good, so nothing to do.`
                    },
                    [ grid("rooms", display(g)).cur(cur).done(visited) ]);
            }
        }
    }

    let answer = render(g);
    log(`answer ${answer}`, "result");
    result(answer);
    leave(answer);
}
