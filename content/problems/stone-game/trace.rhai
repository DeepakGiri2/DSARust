// Stone Game — one number per window: the margin, not the score.
//
// Keeping both players' totals means the recurrence has to know whose turn it
// is. Keeping the *difference* removes that: dp[i] answers "how far ahead can
// whoever moves next finish, on this window?" — a question that reads the same
// for either player. Taking a pile therefore means adding its value and
// subtracting the opponent's margin on what is left.
//
// The rolling array works because a window of length L reads only windows of
// length L-1, and the scan runs left to right: dp[i+1] has not been rewritten
// yet, and dp[i] still holds the previous round's value.

fn windows(piles, length) {
    let out = [];
    for i in 0..piles.len() {
        if i + length <= piles.len() { out.push(`[${i}..${i + length - 1}]`); }
    }
    out
}

fn trace(input) {
    let piles = input.piles;
    let n = piles.len();

    enter("stoneGame", #{ n: N(n) });

    let dp = [];
    for i in 0..n { dp.push(0); }
    step("init", "dp[i] will hold the best margin the player to move can achieve, on the window starting at i.",
        [ bars("piles", piles), array("dp (margin)", dp) ]);

    for i in 0..n { dp[i] = piles[i]; }
    step("seed", "A window of one pile: the mover simply takes it, so the margin is that pile's value.",
        [ bars("piles", piles), array("dp (margin)", dp).hl(seq(0, n - 1)) ]);

    let length = 2;
    while length <= n {
        step("len", `Now every window of ${length} consecutive piles. Each one reads two windows of ${length - 1}, which are already solved.`,
            [ bars("piles", piles), array("dp (margin)", dp),
              text("windows of this size", windows(piles, length)) ]);

        let i = 0;
        while i + length <= n {
            let j = i + length - 1;
            step("start", `Window [${i}..${j}]: piles ${join(slice_of(piles, i, j), ", ")}.`,
                [ bars("piles", piles).hl(span(i, j)).window(i, j),
                  array("dp (margin)", dp).pointers(#{ i: i }) ]);

            let take_left = piles[i] - dp[i + 1];
            step("takeLeft", `Take the left pile, ${piles[i]}. The opponent then faces [${i + 1}..${j}] and can finish ${dp[i + 1]} ahead — so the margin is ${piles[i]} − ${dp[i + 1]} = ${take_left}.`,
                [ bars("piles", piles).hl([i]).window(i, j),
                  array("dp (margin)", dp).ptr("opponent", i + 1) ]);

            let take_right = piles[j] - dp[i];
            step("takeRight", `Or take the right pile, ${piles[j]}, leaving [${i}..${j - 1}] where the opponent is ${dp[i]} ahead: ${piles[j]} − ${dp[i]} = ${take_right}.`,
                [ bars("piles", piles).hl([j]).window(i, j),
                  array("dp (margin)", dp).ptr("opponent", i) ]);

            if take_left > take_right {
                step("choose", `${take_left} beats ${take_right}, so an optimal player takes the left pile.`,
                    [ bars("piles", piles).hl([i]).window(i, j) ]);
                dp[i] = take_left;
                log(`[${i}..${j}] margin ${take_left} (take left)`);
                step("storeLeft", `dp[${i}] = ${dp[i]}.`,
                    [ array("dp (margin)", dp).ptr("i", i).hl([i]) ]);
            } else {
                step("choose", `${take_right} is at least as good as ${take_left}, so take the right pile.`,
                    [ bars("piles", piles).hl([j]).window(i, j) ]);
                dp[i] = take_right;
                log(`[${i}..${j}] margin ${take_right} (take right)`);
                step("storeRight", `dp[${i}] = ${dp[i]}.`,
                    [ array("dp (margin)", dp).ptr("i", i).hl([i]) ]);
            }

            i += 1;
        }
        length += 1;
    }

    let ok = dp[0] > 0;
    log(`answer ${ok}`, "result");
    result(`${ok}`);
    step("ret", `Over the whole row the first player finishes ${dp[0]} ahead, so they ${if ok { "win" } else { "do not win" }}.`,
        [ bars("piles", piles), array("dp (margin)", dp).ptr("whole row", 0).hl([0]) ], "return");
    leave(`${ok}`);
}

fn slice_of(arr, lo, hi) {
    let out = [];
    let k = lo;
    while k <= hi { out.push(arr[k]); k += 1; }
    out
}
