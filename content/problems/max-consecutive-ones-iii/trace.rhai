// Max Consecutive Ones III — "flip k zeros" is a window with k zeros in it.
//
// The flips are never actually performed. Once the question is read as "how
// long a window can I hold with at most k zeros inside?", the array does not
// need to change at all — only the two edges move, and the answer is the
// widest the window ever gets.

const A = "nums";

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();
    let k = input.k;

    enter("longestOnes", #{ nums: A(nums), k: N(k) });

    let left = 0;
    let zeros = 0;
    let best = 0;
    set(#{ left: N(0), zeros: N(0), best: N(0) });
    step("init", `The window may hold at most ${k} zero(s). Nothing is ever flipped — the flips are just the zeros allowed inside.`,
        [ array(global::A, nums), text("window", ["left = 0", "zeros = 0", "best = 0"]) ]);

    for right in 0..n {
        set(#{ right: N(right) });
        step("grow", `Extend the right edge to ${right} (value ${nums[right]}).`,
            [ array(global::A, nums).ptr("l", left).ptr("r", right).hl(span(left, right)),
              text("window", [`window = [${left}, ${right}]`, `zeros = ${zeros}`, `best = ${best}`]) ]);

        if nums[right] == 0 {
            zeros += 1;
            set(#{ zeros: N(zeros) });
            step("iszero", `A zero enters the window — that is one of the ${k} flips spent. zeros = ${zeros}.`,
                [ array(global::A, nums).ptr("l", left).ptr("r", right).hl(span(left, right)).bad([right]) ]);
            step("count", `zeros = ${zeros} of ${k} allowed.`,
                [ text("window", [`window = [${left}, ${right}]`, `zeros = ${zeros}`, `best = ${best}`]) ]);
        } else {
            step("iszero", "A one — free, no flip needed.",
                [ array(global::A, nums).ptr("l", left).ptr("r", right).hl(span(left, right)) ]);
        }

        while zeros > k {
            step("shrink", `${zeros} zeros is more than ${k} — the window is illegal and the left edge must give ground.`,
                [ array(global::A, nums).ptr("l", left).ptr("r", right).bad(span(left, right)) ]);
            if nums[left] == 0 {
                step("leftzero", `The element leaving, nums[${left}], is a zero.`,
                    [ array(global::A, nums).ptr("l", left).ptr("r", right).bad([left]) ]);
                zeros -= 1;
                set(#{ zeros: N(zeros) });
                step("give", `A flip is handed back: zeros = ${zeros}.`,
                    [ text("window", [`zeros = ${zeros}`, `k = ${k}`]) ]);
            } else {
                step("leftzero", `nums[${left}] is a one, so leaving it behind costs nothing but length.`,
                    [ array(global::A, nums).ptr("l", left).ptr("r", right) ]);
            }
            left += 1;
            set(#{ left: N(left) });
            step("move", `left = ${left}. Note the left edge never moves backwards — that is why this is linear, not quadratic.`,
                [ array(global::A, nums).ptr("l", left).ptr("r", right).hl(span(left, right)) ]);
        }

        let width = right - left + 1;
        if width > best {
            step("best", `The window [${left}, ${right}] is ${width} long, beating ${best}.`,
                [ array(global::A, nums).ptr("l", left).ptr("r", right).hl(span(left, right)) ]);
            best = width;
            set(#{ best: N(best) });
            step("set", `best = ${best}.`,
                [ text("window", [`window = [${left}, ${right}]`, `zeros = ${zeros}`, `best = ${best}`]) ]);
        } else {
            step("best", `Width ${width} does not beat ${best}.`,
                [ array(global::A, nums).ptr("l", left).ptr("r", right).hl(span(left, right)) ]);
        }
    }

    log(`longest run = ${best}`, "result");
    result(`${best}`);
    step("ret", `The widest legal window was ${best} long.`,
        [ array(global::A, nums), text("window", [`best = ${best}`]) ], "return");
    leave(`${best}`);
}
