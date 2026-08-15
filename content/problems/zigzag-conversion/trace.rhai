// Zigzag Conversion — the picture is a red herring; only the row matters.
//
// The zigzag is drawn below to show what is going on, but the algorithm never
// builds it. A character's column is irrelevant to the answer, because the
// output reads whole rows left to right in the order the characters were
// written. So one row index bouncing between 0 and numRows-1 is the entire
// state.

// Simulate the walk once so the picture can be the right size from step one.
fn layout_size(n, num_rows) {
    let row = 0;
    let dir = 1;
    let col = 0;
    let max_col = 0;
    for i in 0..n {
        if col > max_col { max_col = col; }
        if row == 0 { dir = 1; } else if row == num_rows - 1 { dir = -1; }
        row += dir;
        if dir == -1 { col += 1; }
    }
    max_col + 1
}

fn blank(rows, cols) {
    let g = [];
    for r in 0..rows {
        let line = [];
        for c in 0..cols { line.push("·"); }
        g.push(line);
    }
    g
}

fn trace(input) {
    let s = input.s;
    let cs = chars(s);
    let n = cs.len();
    let num_rows = input.numRows;

    enter("convert", #{ numRows: N(num_rows) });

    if num_rows == 1 {
        step("single", "With one row there is no zigzag at all — the string is already its own answer.",
            [ array("input", cs) ]);
        log(`answer ${s}`, "result");
        result(s);
        step("ret", `Return "${s}" unchanged.`, [ text("answer", [s]) ], "return");
        leave(s);
        return;
    }
    step("single", `${num_rows} rows, so there really is a zigzag.`, [ array("input", cs) ]);

    let cols = layout_size(n, num_rows);
    let picture = blank(num_rows, cols);
    let rows = [];
    for r in 0..num_rows { rows.push(""); }
    step("init", "One buffer per row. Characters are appended to their row in the order they are met, which is already left-to-right within that row.",
        [ grid("zigzag", picture), text("row buffers", row_lines(rows)) ]);

    let row = 0;
    let dir = 1;
    let col = 0;
    set(#{ row: N(row), dir: N(dir) });
    step("cursor", "Start at the top row, heading down.",
        [ grid("zigzag", picture).cur([row, col]), text("row buffers", row_lines(rows)) ]);

    for i in 0..n {
        step("loop", `Character ${i}: "${cs[i]}".`,
            [ array("input", cs).ptr("i", i).done(before(i)),
              grid("zigzag", picture).cur([row, col]) ]);

        rows[row] += cs[i];
        grid_set(picture, row, col, cs[i]);
        step("place", `It belongs to row ${row}. Its column is only there for the picture — the answer never asks about it.`,
            [ grid("zigzag", picture).cur([row, col]).hl([[row, col]]),
              text("row buffers", row_lines(rows)) ]);

        if row == 0 {
            dir = 1;
            step("atTop", "At the top row.", [ grid("zigzag", picture).cur([row, col]) ]);
            step("down", "So the only way to go is down.", [ grid("zigzag", picture).cur([row, col]) ]);
        } else if row == num_rows - 1 {
            dir = -1;
            step("atBottom", "At the bottom row.", [ grid("zigzag", picture).cur([row, col]) ]);
            step("up", "So turn around and head up the diagonal.",
                [ grid("zigzag", picture).cur([row, col]) ]);
        } else {
            step("atTop", `Row ${row} is neither the top nor the bottom, so the direction is unchanged.`,
                [ grid("zigzag", picture).cur([row, col]) ]);
            step("atBottom", `Still heading ${if dir == 1 { "down" } else { "up" }}.`,
                [ grid("zigzag", picture).cur([row, col]) ]);
        }

        row += dir;
        if dir == -1 { col += 1; }
        set(#{ row: N(row), dir: N(dir) });
        if i + 1 < n {
            step("move", `Next character goes in row ${row}.`,
                [ grid("zigzag", picture).cur([row, col]) ]);
        } else {
            step("move", "That was the last character.",
                [ grid("zigzag", picture) ]);
        }
    }

    step("join", "Now read the rows off in order.",
        [ grid("zigzag", picture), text("row buffers", row_lines(rows)) ]);

    let out = "";
    for r in 0..num_rows {
        out += rows[r];
        step("concat", `Row ${r} contributes "${rows[r]}": "${out}".`,
            [ text("row buffers", row_lines(rows)).hl([r]), text("building", [out]) ]);
    }

    log(`answer ${out}`, "result");
    result(out);
    step("ret", `Read row by row: ${out}.`,
        [ grid("zigzag", picture), text("answer", [out]) ], "return");
    leave(out);
}

fn row_lines(rows) {
    let out = [];
    for r in 0..rows.len() { out.push(`row ${r}: ${rows[r]}`); }
    out
}
