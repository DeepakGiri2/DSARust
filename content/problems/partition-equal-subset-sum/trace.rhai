// Partition Equal Subset Sum — subset-sum in disguise, scanned backwards.
//
// Two equal halves means one of them totals exactly half the array, so the
// only question is whether that number is reachable as a subset sum. Each
// value extends every already-reachable total by itself. Walking the totals
// downwards is what keeps each number to a single use: a lower total that this
// same pass just marked is never read again, because the scan has already gone
// past it.

fn marked(reach) {
    let out = [];
    for i in 0..reach.len() { if reach[i] { out.push(i); } }
    out
}

fn totals(t) {
    let out = [];
    for i in 0..(t + 1) { out.push(i); }
    out
}

fn trace(input) {
    let nums = input.nums;
    let n = nums.len();

    enter("canPartition", #{ n: N(n) });

    let total = 0;
    for v in nums { total += v; }
    set(#{ total: N(total) });
    step("sum", `The whole array adds up to ${total}.`, [ array("nums", nums) ]);

    if total % 2 == 1 {
        log(`${total} is odd`, "result");
        step("odd", `${total} is odd, so it cannot be split into two equal whole-number halves — no scanning needed.`,
            [ array("nums", nums), text("totals", [`sum = ${total}`, "odd"]) ], "return");
        result("false");
        leave("false");
        return;
    }
    step("odd", `${total} is even, so equal halves are at least arithmetically possible.`,
        [ array("nums", nums) ]);

    let target = total / 2;
    set(#{ target: N(target) });
    step("target", `Each half would have to total ${target}. The question is now simply: can any subset reach ${target}?`,
        [ array("nums", nums), text("totals", [`sum = ${total}`, `target = ${target}`]) ]);

    let reach = [];
    for i in 0..(target + 1) { reach.push(false); }
    step("init", `One flag per total from 0 to ${target}.`,
        [ array("totals 0..target", totals(target)) ]);

    reach[0] = true;
    step("seed", "A total of 0 is reachable by choosing nothing — every other reachable total is built from this one.",
        [ array("totals 0..target", totals(target)).hl(marked(reach)) ]);

    for k in 0..n {
        let v = nums[k];
        step("loop", `Now allow ${v} to be used.`,
            [ array("nums", nums).ptr("k", k).done(before(k)),
              array("totals 0..target", totals(target)).hl(marked(reach)) ]);

        let s = target;
        while s >= v {
            step("inner", `Could ${s} be reached by adding ${v} to something?`,
                [ array("totals 0..target", totals(target)).ptr("s", s).hl(marked(reach)) ]);

            if reach[s - v] {
                step("check", `${s - v} is reachable, so ${s - v} + ${v} = ${s} is too.`,
                    [ array("totals 0..target", totals(target))
                        .pointers(#{ from: s - v, s: s }).hl(marked(reach)) ]);
                reach[s] = true;
                log(`${s} is reachable`);
                step("mark", `Mark ${s}. Scanning downwards means ${s} will not be read again on this pass, so ${v} cannot be spent twice.`,
                    [ array("totals 0..target", totals(target)).ptr("s", s).hl(marked(reach)) ]);
            } else {
                step("check", `${s - v} is not reachable, so ${v} cannot get us to ${s} either.`,
                    [ array("totals 0..target", totals(target))
                        .pointers(#{ from: s - v, s: s }).bad([s - v]).hl(marked(reach)) ]);
            }
            s -= 1;
        }
    }

    let ok = reach[target];
    log(`answer ${ok}`, "result");
    result(`${ok}`);
    step("ret",
        if ok {
            `${target} is reachable, so one group sums to ${target} and the rest necessarily does too.`
        } else {
            `${target} was never reached, so no split can be equal.`
        },
        [ array("totals 0..target", totals(target)).hl(marked(reach)).ptr("target", target) ],
        "return");
    leave(`${ok}`);
}
