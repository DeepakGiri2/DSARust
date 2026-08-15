// Pascal's Triangle — each row is built from the one above it.
//
// Every interior cell is the sum of the two cells diagonally above, and the
// ends are always 1. The grid view makes the two parents of each cell visible
// as they are added.

/// Every cell of row `i`, as [row, col] pairs.
fn row_cells(i) {
    let cells = [];
    for c in 0..(i + 1) { cells.push([i, c]); }
    cells
}

fn as_grid(rows, width) {
    let out = [];
    for r in rows {
        let line = [];
        for v in r { line.push(v); }
        while line.len() < width { line.push(""); }
        out.push(line);
    }
    out
}

fn trace(input) {
    let num_rows = input.numRows;
    let res = [];

    enter("generate", #{ numRows: N(num_rows) });
    step("init", "The triangle starts empty.", [ text("triangle", []) ]);

    for i in 0..num_rows {
        set(#{ i: N(i) });
        step("row", `Row ${i} will have ${i + 1} value(s).`,
            [ grid("triangle", as_grid(res, num_rows)) ]);

        let row = [];
        for c in 0..(i + 1) { row.push(0); }
        step("newrow", `Make room for ${i + 1} value(s).`,
            [ grid("triangle", as_grid(res + [row], num_rows)).cur([i, 0]) ]);

        row[0] = 1;
        row[i] = 1;
        step("ends", "Both ends of every row are 1 — there is only one way to reach a corner.",
            [ grid("triangle", as_grid(res + [row], num_rows)).hl([[i, 0], [i, i]]) ]);

        for j in 1..i {
            let a = res[i - 1][j - 1];
            let b = res[i - 1][j];
            step("mid", `Cell (${i},${j}) sits under ${a} and ${b}.`,
                [ grid("triangle", as_grid(res + [row], num_rows))
                    .cur([i, j]).hl([[i - 1, j - 1], [i - 1, j]]) ]);
            row[j] = a + b;
            log(`row[${j}] = ${a} + ${b} = ${row[j]}`);
            step("sum", `${a} + ${b} = ${row[j]}.`,
                [ grid("triangle", as_grid(res + [row], num_rows))
                    .hl([[i, j]]).done([[i - 1, j - 1], [i - 1, j]]) ]);
        }

        res.push(row);
        step("push", `Row ${i} is complete: ${brackets(row)}.`,
            [ grid("triangle", as_grid(res, num_rows)).hl(row_cells(i)) ]);
    }

    log(`${res.len()} rows`, "result");
    result(`${res.len()} rows`);
    step("ret", `${num_rows} rows, each built from the one above it.`,
        [ grid("triangle", as_grid(res, num_rows)) ], "return");
    leave(`${res.len()} rows`);
}
