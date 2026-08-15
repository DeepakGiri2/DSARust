// Unique Paths — every cell is the sum of the two it can be reached from.
//
// You can only arrive at a cell from above or from the left, so the number of
// ways to reach it is the sum of those two. The top row and left column are
// all 1: there is exactly one way to walk in a straight line.

fn as_strings(grid) {
    let out = [];
    for row in grid {
        let line = [];
        for v in row { line.push(`${v}`); }
        out.push(line);
    }
    out
}

fn trace(input) {
    let m = input.m;
    let n = input.n;

    let dp = [];
    for r in 0..m {
        let row = [];
        for c in 0..n { row.push(0); }
        dp.push(row);
    }

    enter("uniquePaths", #{ m: N(m), n: N(n) });
    step("alloc", `A ${m} × ${n} table: dp[r][c] will hold how many ways lead to that cell.`,
        [ grid("ways", as_strings(dp)) ]);

    for r in 0..m {
        step("outer", `Row ${r}.`, [ grid("ways", as_strings(dp)).cur([r, 0]) ]);
        for c in 0..n {
            step("inner", `Cell (${r},${c}).`,
                [ grid("ways", as_strings(dp)).cur([r, c]) ]);

            if r == 0 || c == 0 {
                dp[r][c] = 1;
                step("edge", `It is on the top row or left column, so the only route is a straight line.`,
                    [ grid("ways", as_strings(dp)).cur([r, c]) ]);
                step("one", `dp[${r}][${c}] = 1.`,
                    [ grid("ways", as_strings(dp)).hl([[r, c]]) ]);
            } else {
                let up = dp[r - 1][c];
                let left = dp[r][c - 1];
                dp[r][c] = up + left;
                log(`dp[${r}][${c}] = ${up} + ${left} = ${dp[r][c]}`);
                step("sum",
                    `It can only be entered from above (${up} ways) or from the left (${left} ways): ${up} + ${left} = ${dp[r][c]}.`,
                    [ grid("ways", as_strings(dp)).hl([[r, c]])
                        .done([[r - 1, c], [r, c - 1]]) ]);
            }
        }
    }

    let answer = dp[m - 1][n - 1];
    log(`answer ${answer}`, "result");
    result(`${answer}`);
    step("ret", `The bottom-right corner holds the answer: ${answer} distinct paths.`,
        [ grid("ways", as_strings(dp)).hl([[m - 1, n - 1]]) ], "return");
    leave(`${answer}`);
}
