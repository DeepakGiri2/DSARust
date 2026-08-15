// Max Area of Island — flood fill that returns a count.
//
// Identical in shape to counting islands; the only change is that the
// recursion returns a number. Sinking each cell as it is visited is what makes
// the count exact: no cell can be reached twice, so nothing is double counted.

fn dfs(grid, r, c, visited) {
    let rows = grid.len();
    let cols = grid[0].len();

    if r < 0 || r >= rows || c < 0 || c >= cols || grid_get(grid, r, c) != "1" {
        enter(`dfs(${r},${c})`);
        step("bounds", `(${r},${c}) is off the grid or not land.`,
            [ grid("grid", grid).done(visited) ]);
        step("retZero", "Contributes 0.", [ grid("grid", grid).done(visited) ], "return");
        leave("0");
        return [grid, 0, visited];
    }

    enter(`dfs(${r},${c})`, #{ r: N(r), c: N(c) });
    step("bounds", `(${r},${c}) is land.`,
        [ grid("grid", grid).cur([r, c]).hl([[r, c]]).done(visited) ]);

    grid_set(grid, r, c, "0");
    visited.push([r, c]);
    step("sink", "Sink it so it can never be counted again.",
        [ grid("grid", grid).cur([r, c]).done(visited) ]);

    let total = 1;
    let dirs = [[1, 0], [-1, 0], [0, 1], [0, -1]];
    for d in dirs {
        let out = dfs(grid, r + d[0], c + d[1], visited);
        grid = out[0];
        total += out[1];
        visited = out[2];
    }

    log(`(${r},${c}) subtree area ${total}`);
    step("sum", `This cell plus its neighbours gives ${total}.`,
        [ grid("grid", grid).done(visited) ], "return");
    leave(`${total}`);
    [grid, total, visited]
}

fn trace(input) {
    let grid = grid_rows(input.grid);
    let rows = grid.len();
    let cols = grid[0].len();
    let best = 0;
    let visited = [];

    enter("maxAreaOfIsland", #{});
    set(#{ best: N(best) });
    step("init", "No island measured yet.", [ grid("grid", grid) ]);

    for r in 0..rows {
        step("rowloop", `Row ${r}.`, [ grid("grid", grid).cur([r, 0]).done(visited) ]);
        for c in 0..cols {
            step("colloop", `Cell (${r},${c}) holds '${grid_get(grid, r, c)}'.`,
                [ grid("grid", grid).cur([r, c]).done(visited) ]);

            if grid_get(grid, r, c) == "1" {
                step("found", `Unvisited land at (${r},${c}) — a new island.`,
                    [ grid("grid", grid).cur([r, c]).hl([[r, c]]).done(visited) ]);
                step("call", "Flood fill it and count as we go.",
                    [ grid("grid", grid).cur([r, c]).done(visited) ], "call");
                let out = dfs(grid, r, c, visited);
                grid = out[0];
                visited = out[2];

                if out[1] > best {
                    step("cmp", `Its area is ${out[1]}, beating ${best}.`,
                        [ grid("grid", grid).done(visited) ]);
                    best = out[1];
                    set(#{ best: N(best) });
                    log(`new largest island: ${best}`, "result");
                    step("update", `Largest is now ${best}.`,
                        [ grid("grid", grid).done(visited) ]);
                } else {
                    step("cmp", `Its area is ${out[1]}, which does not beat ${best}.`,
                        [ grid("grid", grid).done(visited) ]);
                }
            }
        }
    }

    log(`answer ${best}`, "result");
    result(`${best}`);
    step("ret", `The largest island holds ${best} cell(s).`,
        [ grid("grid", grid).done(visited) ], "return");
    leave(`${best}`);
}
