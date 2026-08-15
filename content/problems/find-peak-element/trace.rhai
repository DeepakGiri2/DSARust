// Find Peak Element — binary search without any sortedness.
//
// Halving does not require sorted data. It requires a test whose answer proves
// a peak still exists in the half you keep. Comparing an element with its right
// neighbour is such a test: a rising slope cannot rise forever, because past
// the end the value drops to negative infinity, so somewhere to the right it
// must turn over. The mirror argument covers a falling slope.

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();

    enter("findPeakElement", #{ n: N(n) });

    let lo = 0;
    let hi = n - 1;
    set(#{ lo: N(lo), hi: N(hi) });
    step("bounds", "Off both ends the value counts as negative infinity, so the whole array is guaranteed to contain a peak.",
        [ bars("nums", nums).pointers(#{ lo: lo, hi: hi }) ]);

    while lo < hi {
        step("loop", `A peak is guaranteed somewhere in [${lo}..${hi}].`,
            [ bars("nums", nums).pointers(#{ lo: lo, hi: hi }).window(lo, hi)
                .done(dead(n, lo, hi)),
              text("invariant", ["the live range always contains a peak"]) ]);

        let mid = lo + (hi - lo) / 2;
        set(#{ mid: N(mid) });
        step("mid", `mid = ${mid}, holding ${nums[mid]}; its right neighbour is ${nums[mid + 1]}.`,
            [ bars("nums", nums).pointers(#{ lo: lo, mid: mid, hi: hi }).hl([mid, mid + 1])
                .window(lo, hi).done(dead(n, lo, hi)) ]);

        if nums[mid] < nums[mid + 1] {
            step("rising", `${nums[mid]} < ${nums[mid + 1]}: the slope is rising. It cannot rise past the end of the array, so it must turn over somewhere to the right — a peak is guaranteed there.`,
                [ bars("nums", nums).pointers(#{ mid: mid }).hl([mid, mid + 1])
                    .bad(span(lo, mid)).window(lo, hi) ]);
            lo = mid + 1;
            set(#{ lo: N(lo) });
            step("goRight", `Discard everything up to ${mid}: lo = ${lo}.`,
                [ bars("nums", nums).pointers(#{ lo: lo, hi: hi }).window(lo, hi)
                    .done(dead(n, lo, hi)) ]);
        } else {
            step("falling", `${nums[mid]} > ${nums[mid + 1]}: the slope is falling, so walking left from ${mid} must reach a peak — possibly ${mid} itself.`,
                [ bars("nums", nums).pointers(#{ mid: mid }).hl([mid, mid + 1])
                    .bad(span(mid + 1, hi)).window(lo, hi) ]);
            hi = mid;
            set(#{ hi: N(hi) });
            step("goLeft", `Keep ${mid} — it is still a candidate — so hi = ${hi}, not ${mid - 1}.`,
                [ bars("nums", nums).pointers(#{ lo: lo, hi: hi }).window(lo, hi)
                    .done(dead(n, lo, hi)) ]);
        }
    }

    log(`peak at index ${lo}, value ${nums[lo]}`, "result");
    result(`${lo}`);
    step("ret", `The range collapsed to index ${lo}, value ${nums[lo]}. Every discarded half provably contained no *needed* peak, so this one is genuine.`,
        [ bars("nums", nums).ptr("peak", lo).hl([lo]) ], "return");
    leave(`${lo}`);
}
