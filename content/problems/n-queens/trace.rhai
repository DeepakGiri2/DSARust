// N-Queens — three sets replace the whole board.
//
// One queen per row makes row conflicts impossible by construction. The three
// remaining directions become arithmetic: two squares share a column when
// their column indices match, a ↘ diagonal when row − column matches, and a ↗
// diagonal when row + column matches. Keeping a set for each turns "is this
// square attacked?" into three lookups, with no scanning of the board at all —
// and undoing a placement is three removals.

fn board_view(board, n, placed) {
    let g = [];
    for r in 0..n {
        let line = [];
        for c in 0..n {
            line.push(if r < placed && board[r] == c { "Q" } else { "·" });
        }
        g.push(line);
    }
    g
}

fn queens(board, placed) {
    let out = [];
    for r in 0..placed { out.push([r, board[r]]); }
    out
}

fn set_view(s) {
    if s.len() == 0 { return "(empty)"; }
    join(set_items(s), ", ")
}

fn back(n, row, board, cols, diag, anti, res) {
    enter(`back(row = ${row})`, #{ row: N(row) });

    if row == n {
        step("complete", `All ${n} queens are placed with no two attacking.`,
            [ grid("board", board_view(board, n, row)).hl(queens(board, row)) ]);
        let cells = [];
        for r in 0..n { cells.push(board[r]); }
        res.push(join(cells, ","));
        log(`solution ${join(cells, ",")}`, "result");
        step("record", `Record it as the column of each row: ${join(cells, ",")}.`,
            [ text("solutions", res) ]);
        step("retDone", "Unwind and look for another.", [ text("solutions", res) ], "return");
        leave("");
        return [board, cols, diag, anti, res];
    }

    for c in 0..n {
        step("loop", `Row ${row}, trying column ${c}.`,
            [ grid("board", board_view(board, n, row)).cur([row, c]).hl(queens(board, row)),
              text("occupied", [`columns: ${set_view(cols)}`,
                                `row − col: ${set_view(diag)}`,
                                `row + col: ${set_view(anti)}`]) ]);

        if set_has(cols, c) || set_has(diag, row - c) || set_has(anti, row + c) {
            step("attacked",
                if set_has(cols, c) { `Column ${c} already holds a queen.` }
                else if set_has(diag, row - c) { `A queen already sits on the ↘ diagonal row − col = ${row - c}.` }
                else { `A queen already sits on the ↗ diagonal row + col = ${row + c}.` },
                [ grid("board", board_view(board, n, row)).cur([row, c]).bad([[row, c]])
                    .hl(queens(board, row)) ]);
            continue;
        }

        set_add(cols, c);
        set_add(diag, row - c);
        set_add(anti, row + c);
        board[row] = c;
        step("place", `Place a queen at (${row},${c}) and claim column ${c}, diagonal ${row - c} and anti-diagonal ${row + c}.`,
            [ grid("board", board_view(board, n, row + 1)).hl(queens(board, row + 1)),
              text("occupied", [`columns: ${set_view(cols)}`,
                                `row − col: ${set_view(diag)}`,
                                `row + col: ${set_view(anti)}`]) ]);

        step("recurse", `Move on to row ${row + 1}.`,
            [ grid("board", board_view(board, n, row + 1)).hl(queens(board, row + 1)) ], "call");
        let out = back(n, row + 1, board, cols, diag, anti, res);
        board = out[0];
        cols = out[1];
        diag = out[2];
        anti = out[3];
        res = out[4];

        set_del(cols, c);
        set_del(diag, row - c);
        set_del(anti, row + c);
        step("undo", `Take the queen at (${row},${c}) away again — three removals, exactly mirroring the three claims.`,
            [ grid("board", board_view(board, n, row)).hl(queens(board, row)),
              text("occupied", [`columns: ${set_view(cols)}`,
                                `row − col: ${set_view(diag)}`,
                                `row + col: ${set_view(anti)}`]) ]);
    }

    leave("");
    [board, cols, diag, anti, res]
}

fn trace(input) {
    let n = input.n;

    enter("solveNQueens", #{ n: N(n) });

    let board = [];
    for r in 0..n { board.push(-1); }
    step("init", `An ${n} × ${n} board, one queen per row.`,
        [ grid("board", board_view(board, n, 0)) ]);

    let cols = set_new();
    step("cols", "Columns already claimed.", [ text("occupied", ["columns: (empty)"]) ]);
    let diag = set_new();
    step("diag", "Two squares are on the same ↘ diagonal exactly when row − column matches.",
        [ text("occupied", ["row − col: (empty)"]) ]);
    let anti = set_new();
    step("anti", "And on the same ↗ diagonal exactly when row + column matches.",
        [ text("occupied", ["row + col: (empty)"]) ]);
    step("board", "The column chosen for each row.", [ grid("board", board_view(board, n, 0)) ]);

    step("call", "Start at row 0.", [ grid("board", board_view(board, n, 0)) ], "call");
    let out = back(n, 0, board, cols, diag, anti, []);
    let res = out[4];

    let answer = join(res, " ");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `${plural(res.len(), "arrangement")} for ${n} queens.`,
        [ text("solutions", res) ], "return");
    leave(answer);
}
