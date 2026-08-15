// Rotting Oranges — BFS spreading from every rotten cell at once.
//
// This is a multi-source BFS: all the initially rotten oranges are level 0, so
// one wave per minute reaches exactly the cells that rot in that minute. A
// plain DFS would count distance from one source and get the timing wrong.

fn trace(input) {
    let grid = grid_rows(input.grid);
    let rows = grid.len();
    let cols = grid[0].len();

    let frontier = [];
    let fresh = 0;
    let rotted = [];

    enter("orangesRotting", #{});
    step("init", "0 = empty, 1 = fresh, 2 = rotten.", [ grid("grid", grid) ]);

    for r in 0..rows {
        for c in 0..cols {
            if grid_get(grid, r, c) == "2" { frontier.push([r, c]); }
            if grid_get(grid, r, c) == "1" { fresh += 1; }
        }
    }
    set(#{ fresh: N(fresh), rotten: N(frontier.len()) });
    step("scan",
        `${frontier.len()} orange(s) start rotten and ${fresh} are fresh. Every rotten one is a starting point of the same wave.`,
        [ grid("grid", grid).hl(frontier) ]);

    let minutes = 0;
    set(#{ minutes: N(minutes) });
    step("min0", "No time has passed.", [ grid("grid", grid).hl(frontier) ]);

    while frontier.len() > 0 && fresh > 0 {
        step("wave", `Minute ${minutes + 1}: ${frontier.len()} rotten orange(s) infect their neighbours.`,
            [ grid("grid", grid).hl(frontier).done(rotted) ]);

        minutes += 1;
        set(#{ minutes: N(minutes) });
        step("tick", `The clock advances to ${minutes}.`,
            [ grid("grid", grid).hl(frontier).done(rotted) ]);

        let next = [];
        for cell in frontier {
            let r = cell[0];
            let c = cell[1];
            step("spread", `Spread from (${r},${c}).`,
                [ grid("grid", grid).cur([r, c]).hl(frontier).done(rotted) ]);

            let dirs = [[1, 0], [-1, 0], [0, 1], [0, -1]];
            for d in dirs {
                let nr = r + d[0];
                let nc = c + d[1];
                if nr < 0 || nr >= rows || nc < 0 || nc >= cols { continue; }
                if grid_get(grid, nr, nc) != "1" { continue; }

                step("check", `(${nr},${nc}) holds a fresh orange next to a rotten one.`,
                    [ grid("grid", grid).cur([nr, nc]).hl([[r, c]]).done(rotted) ]);
                grid_set(grid, nr, nc, "2");
                fresh -= 1;
                next.push([nr, nc]);
                set(#{ fresh: N(fresh) });
                log(`(${nr},${nc}) rots at minute ${minutes}`);
                step("rot", `It rots. ${fresh} fresh orange(s) left.`,
                    [ grid("grid", grid).hl(next).done(rotted) ]);
            }
            rotted.push([r, c]);
        }
        frontier = next;
    }

    if fresh > 0 {
        step("chk", `${fresh} orange(s) are still fresh and nothing rotten can reach them.`,
            [ grid("grid", grid).done(rotted) ]);
        log("some oranges are unreachable", "result");
        result("-1");
        step("bad", "Return -1.", [ grid("grid", grid).done(rotted) ], "return");
        leave("-1");
        return;
    }

    step("chk", "No fresh oranges remain.", [ grid("grid", grid).done(rotted) ]);
    log(`answer ${minutes}`, "result");
    result(`${minutes}`);
    step("ret", `Everything rotted after ${minutes} minute(s) — one wave per minute.`,
        [ grid("grid", grid).done(rotted) ], "return");
    leave(`${minutes}`);
}
