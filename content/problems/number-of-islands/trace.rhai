// Number of Islands — flood fill sinks each island as it is counted.
//
// The trick worth seeing: the scan does not need to know anything clever. It
// counts a cell only when it is still land, and then the DFS erases the entire
// connected island so the scan can never count it twice.

fn sink(grid, r, c, visited) {
    let rows = grid.len();
    let cols = grid[0].len();

    if r < 0 || r >= rows || c < 0 || c >= cols {
        enter(`dfs(${r},${c})`);
        step("bounds", `(${r},${c}) is off the edge of the grid — stop.`,
            [ grid("grid", grid).done(visited) ]);
        step("retn", "Return to the caller.", [ grid("grid", grid).done(visited) ], "return");
        leave("");
        return [grid, visited];
    }

    if grid_get(grid, r, c) != "1" {
        enter(`dfs(${r},${c})`);
        step("bounds", `(${r},${c}) is water (or already sunk) — nothing to do here.`,
            [ grid("grid", grid).cur([r, c]).done(visited) ]);
        step("retn", "Return to the caller.", [ grid("grid", grid).done(visited) ], "return");
        leave("");
        return [grid, visited];
    }

    enter(`dfs(${r},${c})`, #{ r: N(r), c: N(c) });
    step("bounds", `(${r},${c}) is land and inside the grid, so it belongs to the island being sunk.`,
        [ grid("grid", grid).cur([r, c]).hl([[r, c]]).done(visited) ]);

    grid_set(grid, r, c, "0");
    visited.push([r, c]);
    log(`sink (${r},${c})`);
    step("sink",
        "Flip it to water. This is what stops the search from revisiting the cell and what stops the outer scan from counting this island again.",
        [ grid("grid", grid).cur([r, c]).done(visited) ]);

    let dirs = [[1, 0, "down"], [-1, 0, "up"], [0, 1, "right"], [0, -1, "left"]];
    for d in dirs {
        step(d[2], `Follow the island ${d[2]} from (${r},${c}).`,
            [ grid("grid", grid).cur([r, c]).done(visited) ], "call");
        let out = sink(grid, r + d[0], c + d[1], visited);
        grid = out[0];
        visited = out[1];
    }

    leave("");
    [grid, visited]
}

fn trace(input) {
    let grid = grid_rows(input.grid);
    let rows = grid.len();
    let cols = grid[0].len();
    let count = 0;
    let visited = [];

    enter("numIslands", #{ rows: N(rows), cols: N(cols) });
    set(#{ count: N(count) });
    step("count0", "No islands counted yet. Scan every cell; each piece of land that survives to be seen starts a new island.",
        [ grid("grid", grid) ]);

    for r in 0..rows {
        step("rowloop", `Row ${r}.`, [ grid("grid", grid).cur([r, 0]).done(visited) ]);
        for c in 0..cols {
            step("colloop", `Cell (${r},${c}) holds '${grid_get(grid, r, c)}'.`,
                [ grid("grid", grid).cur([r, c]).done(visited) ]);

            if grid_get(grid, r, c) == "1" {
                step("isfound", `(${r},${c}) is land that has not been sunk yet — this is a brand new island.`,
                    [ grid("grid", grid).cur([r, c]).hl([[r, c]]).done(visited) ]);
                count += 1;
                set(#{ count: N(count) });
                log(`island #${count} found at (${r},${c})`, "result");
                step("inc", `That makes ${plural(count, "island")}.`,
                    [ grid("grid", grid).cur([r, c]).hl([[r, c]]).done(visited) ]);

                step("calldfs", `Flood fill from (${r},${c}) to erase the whole island before continuing the scan.`,
                    [ grid("grid", grid).cur([r, c]).done(visited) ], "call");
                let out = sink(grid, r, c, visited);
                grid = out[0];
                visited = out[1];
            }
        }
    }

    log(`total ${count}`, "result");
    result(`${count}`);
    step("ret", `The scan finished with every island erased exactly once: ${plural(count, "island")}.`,
        [ grid("grid", grid).done(visited) ], "return");
    leave(`${count}`);
}
