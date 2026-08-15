// Valid Sudoku — one pass, three kinds of key.
//
// Rather than scanning rows, then columns, then boxes separately, every filled
// cell contributes three keys to one set. The box key is the trick:
// (r/3, c/3) identifies which 3×3 block a cell lives in.

/// The board arrives as one flat 81-character string; fold it into 9 rows.
fn reshape(spec) {
    let cells = chars(spec);
    let rows = [];
    for r in 0..9 {
        let row = [];
        for c in 0..9 {
            let i = r * 9 + c;
            row.push(if i < cells.len() { cells[i] } else { "." });
        }
        rows.push(row);
    }
    rows
}

fn trace(input) {
    let board = reshape(input.board);
    let seen = set_new();
    let filled = [];

    enter("isValidSudoku", #{});
    set(#{ seen: SET([]) });
    step("init",
        "One set for everything. A digit's row, column and box each become a distinct key.",
        [ grid("board", board) ]);

    for r in 0..9 {
        step("rowloop", `Row ${r}.`, [ grid("board", board).cur([r, 0]).done(filled) ]);

        for c in 0..9 {
            step("colloop", `Cell (${r},${c}).`,
                [ grid("board", board).cur([r, c]).done(filled) ]);

            let v = grid_get(board, r, c);
            step("read", `It holds '${v}'.`,
                [ grid("board", board).cur([r, c]).done(filled) ]);

            if v == "." {
                step("empty", "Empty cells constrain nothing.",
                    [ grid("board", board).cur([r, c]).done(filled) ]);
                continue;
            }

            let box = `${r / 3},${c / 3}`;
            let keys = [`r${r}=${v}`, `c${c}=${v}`, `b${box}=${v}`];
            step("keys",
                `Three keys: "${keys[0]}", "${keys[1]}" and "${keys[2]}". The box index (${box}) comes from dividing both coordinates by 3.`,
                [ grid("board", board).cur([r, c]).hl([[r, c]]).done(filled),
                  text("keys for this cell", keys) ]);

            let clash = ();
            for key in keys {
                step("each", `Check "${key}".`,
                    [ grid("board", board).cur([r, c]).done(filled),
                      text("keys for this cell", keys) ]);
                if set_has(seen, key) {
                    clash = key;
                    break;
                }
            }

            if type_of(clash) != "()" {
                log(`duplicate: ${clash}`, "result");
                step("check", `"${clash}" has already been recorded — the same digit appears twice in that row, column or box.`,
                    [ grid("board", board).bad([[r, c]]).done(filled) ]);
                result("false");
                step("bad", "Return false.", [ grid("board", board).bad([[r, c]]) ], "return");
                leave("false");
                return;
            }

            step("check", "None of the three keys has been seen before.",
                [ grid("board", board).cur([r, c]).done(filled) ]);

            for key in keys { set_add(seen, key); }
            filled.push([r, c]);
            set(#{ seen: N(set_len(seen)) });
            step("mark", `Record all three. ${set_len(seen)} constraint(s) recorded so far.`,
                [ grid("board", board).hl([[r, c]]).done(filled) ]);
        }
    }

    log("no duplicates anywhere", "result");
    result("true");
    step("ok", "Every filled cell passed all three checks in a single sweep of the board.",
        [ grid("board", board).done(filled) ], "return");
    leave("true");
}
