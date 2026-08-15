// Rotate Image — transpose, then reverse each row.
//
// A clockwise quarter turn moves column j to row j, counted from the right.
// Transposing does the first half of that (column j becomes row j) and
// reversing each row does the second (counted from the right). Both are in
// place, so the rotation needs no second matrix.

fn trace(input) {
    let m = grid_rows(input.matrix);
    let n = m.len();

    enter("rotate", #{ n: N(n) });
    step("tOuter", "Step one: mirror the matrix across its main diagonal.",
        [ grid("matrix", m) ]);

    for i in 0..n {
        for j in (i + 1)..n {
            step("tInner", `Cell (${i},${j}) pairs with (${j},${i}).`,
                [ grid("matrix", m).cur([i, j]).hl([[j, i]]) ]);
            let a = grid_get(m, i, j);
            let b = grid_get(m, j, i);
            grid_set(m, i, j, b);
            grid_set(m, j, i, a);
            step("swap", `Swap ${a} and ${b}.`,
                [ grid("matrix", m).hl([[i, j], [j, i]]) ]);
        }
    }
    step("rOuter", "Transposed. Every column is now a row — but in the wrong order.",
        [ grid("matrix", m) ]);

    for i in 0..n {
        step("rInner", `Reverse row ${i}.`, [ grid("matrix", m).cur([i, 0]) ]);
        let l = 0;
        let r = n - 1;
        while l < r {
            let a = grid_get(m, i, l);
            let b = grid_get(m, i, r);
            grid_set(m, i, l, b);
            grid_set(m, i, r, a);
            step("rSwap", `Swap (${i},${l}) and (${i},${r}).`,
                [ grid("matrix", m).hl([[i, l], [i, r]]) ]);
            l += 1;
            r -= 1;
        }
    }

    let rows = [];
    for row in m { rows.push(join(row, "")); }
    let answer = join(rows, " ");
    log(`answer ${answer}`, "result");
    result(answer);
    step("rOuter", `Rotated in place: ${answer}.`, [ grid("matrix", m) ], "return");
    leave(answer);
}
