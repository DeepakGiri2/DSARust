fn parse_rows(spec) {
    let out = [];
    for row in words(spec) {
        let cells = [];
        for v in row.split(",") { cells.push(num(v)); }
        out.push(cells);
    }
    out
}

fn render_rows(g) {
    let out = [];
    for row in g { out.push(join(row, ",")); }
    join(out, " ")
}

// Snakes and Ladders — BFS, with the board layout as arithmetic.
//
// Each roll is one move, so all edges cost the same and breadth-first search
// gives the fewest moves. The boustrophedon numbering never needs searching:
// square s sits in row (s-1)/n counted from the bottom, and the column is
// reversed on every other row. A snake or ladder does not add a move — it only
// changes where the edge from the roll actually lands.

fn cell_of(n, square) {
    let idx = square - 1;
    let row = idx / n;
    let col = idx % n;
    if row % 2 == 1 { col = n - 1 - col; }
    [n - 1 - row, col]
}

fn display(board, n) {
    let g = [];
    for r in 0..n {
        let line = [];
        for c in 0..n {
            line.push(if board[r][c] == -1 { "·" } else { `${board[r][c]}` });
        }
        g.push(line);
    }
    g
}

fn queue_labels(q, qi) {
    let out = [];
    for k in qi..q.len() { out.push(q[k]); }
    out
}

fn trace(input) {
    let board = parse_rows(input.board);
    let n = board.len();

    enter("snakesAndLadders", #{ n: N(n) });

    let target = n * n;
    set(#{ target: N(target) });
    step("target", `The board has ${target} squares, numbered from the bottom left and snaking back and forth.`,
        [ grid("board (· = plain square, otherwise the destination)", display(board, n)) ]);

    let dist = [];
    for i in 0..(target + 1) { dist.push(-1); }
    step("init", "One distance per square; -1 means unreached.",
        [ array("moves to reach each square (1..n²)", tail(dist)) ]);

    dist[1] = 0;
    step("seed", "Square 1 costs nothing — it is where the game starts.",
        [ array("moves to reach each square (1..n²)", tail(dist)).hl([0]) ]);

    let q = [1];
    let qi = 0;
    step("queue", "The frontier.",
        [ queue("frontier", queue_labels(q, qi)) ]);

    let answer = -1;
    while qi < q.len() {
        step("loop", `${q.len() - qi} square(s) on the frontier.`,
            [ queue("frontier", queue_labels(q, qi)),
              array("moves to reach each square (1..n²)", tail(dist)) ]);

        let cur = q[qi];
        qi += 1;
        let rc = cell_of(n, cur);
        step("pop", `Expanding square ${cur}, reached in ${plural(dist[cur], "move")}.`,
            [ grid("board (· = plain square, otherwise the destination)", display(board, n)).cur(rc),
              queue("frontier", queue_labels(q, qi)) ]);

        if cur == target {
            answer = dist[cur];
            log(`reached ${target} in ${answer} move(s)`, "result");
            step("arrived", `Square ${cur} is the last one. Because BFS expands in order of move count, this is the first and therefore shortest arrival.`,
                [ grid("board (· = plain square, otherwise the destination)", display(board, n)).cur(rc) ]);
            step("ret", `${plural(answer, "move")}.`,
                [ array("moves to reach each square (1..n²)", tail(dist)) ], "return");
            break;
        }

        for roll in 1..7 {
            step("rolls", `Rolling ${roll}.`,
                [ grid("board (· = plain square, otherwise the destination)", display(board, n)).cur(rc) ]);

            let next = cur + roll;
            step("step", `That would land on square ${next}.`,
                [ text("move", [`from ${cur}`, `roll ${roll}`, `lands on ${next}`]) ]);

            if next > target {
                step("past", `${next} is past the end of the board, so this roll and every larger one is illegal.`,
                    [ text("move", [`${next} > ${target}`]) ]);
                break;
            }

            let nrc = cell_of(n, next);
            step("locate", `Square ${next} is at row ${nrc[0]}, column ${nrc[1]} — pure arithmetic from the numbering.`,
                [ grid("board (· = plain square, otherwise the destination)", display(board, n))
                    .cur(rc).hl([nrc]) ]);

            if board[nrc[0]][nrc[1]] != -1 {
                step("jump", `That square holds a snake or ladder to ${board[nrc[0]][nrc[1]]}.`,
                    [ grid("board (· = plain square, otherwise the destination)", display(board, n))
                        .cur(nrc) ]);
                next = board[nrc[0]][nrc[1]];
                log(`${cur} + ${roll} jumps to ${next}`);
                step("teleport", `The move ends on ${next} instead — still one move, just a different destination.`,
                    [ grid("board (· = plain square, otherwise the destination)", display(board, n))
                        .cur(cell_of(n, next)) ]);
            } else {
                step("jump", "A plain square.",
                    [ grid("board (· = plain square, otherwise the destination)", display(board, n))
                        .cur(nrc) ]);
            }

            if dist[next] == -1 {
                step("fresh", `Square ${next} has not been reached before.`,
                    [ array("moves to reach each square (1..n²)", tail(dist)).ptr("next", next - 1) ]);
                dist[next] = dist[cur] + 1;
                step("setDist", `It takes ${plural(dist[next], "move")}.`,
                    [ array("moves to reach each square (1..n²)", tail(dist)).hl([next - 1]) ]);
                q.push(next);
                step("push", "Queue it.", [ queue("frontier", queue_labels(q, qi)) ]);
            } else {
                step("fresh", `Square ${next} was already reached in ${plural(dist[next], "move")}, which cannot be beaten from here.`,
                    [ array("moves to reach each square (1..n²)", tail(dist)).ptr("next", next - 1) ]);
            }
        }
    }

    if answer < 0 {
        log("unreachable", "result");
        step("none", "The frontier emptied without reaching the last square.",
            [ array("moves to reach each square (1..n²)", tail(dist)) ], "return");
    }

    result(`${answer}`);
    leave(`${answer}`);
}

fn tail(dist) {
    let out = [];
    for i in 1..dist.len() { out.push(dist[i]); }
    out
}
