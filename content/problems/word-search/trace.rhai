// Word Search — DFS with an undo.
//
// Marking a cell before recursing stops the path from reusing it; unmarking it
// afterwards is essential, because a cell that fails on one path may well be
// needed by another. That single restore line is the difference between a
// correct search and a wrong one.

fn dfs(board, word, r, c, k, path) {
    let rows = board.len();
    let cols = board[0].len();

    if r < 0 || r >= rows || c < 0 || c >= cols || grid_get(board, r, c) != word[k] {
        enter(`dfs(${r},${c},k=${k})`);
        let why = if r < 0 || r >= rows || c < 0 || c >= cols {
            "off the board"
        } else if grid_get(board, r, c) == "#" {
            "already used by this path"
        } else {
            "the wrong letter"
        };
        step("bounds", `(${r},${c}) is ${why}.`,
            [ grid("board", board).cur(if r >= 0 && r < rows && c >= 0 && c < cols { [r, c] } else { () })
                .path(path),
              array("word", word).ptr("k", k) ]);
        step("retbad", "Dead end — back out.", [ grid("board", board).path(path) ], "return");
        leave("false");
        return [board, false];
    }

    enter(`dfs(${r},${c},k=${k})`, #{ r: N(r), c: N(c), k: N(k) });
    path.push([r, c]);
    step("bounds", `(${r},${c}) holds '${word[k]}', which is what position ${k} of the word needs.`,
        [ grid("board", board).cur([r, c]).path(path),
          array("word", word).ptr("k", k).hl(before(k + 1)) ]);

    if k == word.len() - 1 {
        log("word found", "result");
        step("found", "That was the last letter — the whole word has been traced out.",
            [ grid("board", board).hl(path), array("word", word).hl(seq(0, word.len() - 1)) ]);
        step("retok", "Return true all the way up.",
            [ grid("board", board).hl(path) ], "return");
        leave("true");
        return [board, true];
    }

    let saved = grid_get(board, r, c);
    grid_set(board, r, c, "#");
    step("mark", `Mark (${r},${c}) as used so this path cannot walk back over it.`,
        [ grid("board", board).cur([r, c]).path(path), array("word", word).ptr("k", k) ]);

    let dirs = [[1, 0, "down"], [-1, 0, "up"], [0, 1, "right"], [0, -1, "left"]];
    let found = false;
    for d in dirs {
        if found { break; }
        step(d[2], `Try ${d[2]} from (${r},${c}) for '${word[k + 1]}'.`,
            [ grid("board", board).cur([r, c]).path(path),
              array("word", word).ptr("k", k + 1) ], "call");
        let out = dfs(board, word, r + d[0], c + d[1], k + 1, path);
        board = out[0];
        found = out[1];
    }

    if !found {
        grid_set(board, r, c, saved);
        path.truncate(path.len() - 1);
        step("restore",
            `No direction worked. Restore '${saved}' — another path may still need this cell.`,
            [ grid("board", board).cur([r, c]).path(path) ]);
        step("retback", "Return false.", [ grid("board", board).path(path) ], "return");
    } else {
        step("retback", "A match was found below; unwind.",
            [ grid("board", board).hl(path) ], "return");
    }
    leave(`${found}`);
    [board, found]
}

fn trace(input) {
    let board = grid_rows(input.board);
    let word = chars(input.word);
    let rows = board.len();
    let cols = board[0].len();

    enter("exist", #{ word: S(input.word) });

    for r in 0..rows {
        step("rowloop", `Row ${r}.`, [ grid("board", board).cur([r, 0]), array("word", word) ]);
        for c in 0..cols {
            step("colloop", `Could the word start at (${r},${c})?`,
                [ grid("board", board).cur([r, c]), array("word", word).ptr("k", 0) ]);
            step("calldfs", `Search from (${r},${c}).`,
                [ grid("board", board).cur([r, c]) ], "call");

            let out = dfs(board, word, r, c, 0, []);
            board = out[0];
            if out[1] {
                result("true");
                step("rettrue", "The word is on the board.",
                    [ grid("board", board) ], "return");
                leave("true");
                return;
            }
        }
    }

    log("word not found", "result");
    result("false");
    step("retfalse", "No starting cell led to a complete match.",
        [ grid("board", board) ], "return");
    leave("false");
}
