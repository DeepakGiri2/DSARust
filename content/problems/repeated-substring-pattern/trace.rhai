// Repeated Substring Pattern — one number out of the KMP table decides it.
//
// If a prefix of length L is also a suffix, then sliding the string left by
// n − L lands it exactly on itself. A string that repeats has that shift as
// its block size, so the answer is just "does n − L divide n?". The L > 0
// guard is what stops a string with no self-overlap (like "abc", period 3)
// from claiming to be one block repeated once.

const S = "s";
const LPS = "lps — longest prefix that is also a suffix";

fn trace(input) {
    let s = chars(input.s);
    let n = s.len();

    enter("repeatedSubstringPattern", #{ s: S(input.s), n: N(n) });

    let lps = [];
    for i in 0..n { lps.push(0); }
    set(#{ lps: A(lps) });
    step("lps", "Build the same failure table KMP uses. Here it is not used for searching — only its last entry matters.",
        [ array(global::S, s), array(global::LPS, lps) ]);

    let i = 1;
    let length = 0;
    while i < n {
        set(#{ i: N(i), length: N(length) });
        step("build", `i = ${i}: compare s[${i}] = ${s[i]} with s[${length}] = ${s[length]}.`,
            [ array(global::S, s).ptr("i", i).ptr("len", length).done(before(i)),
              array(global::LPS, lps).done(before(i)) ]);

        if s[i] == s[length] {
            length += 1;
            step("match", `Agreement — the overlap grows to ${length}.`,
                [ array(global::S, s).ptr("i", i).hl([i, length - 1]) ]);
            lps[i] = length;
            set(#{ lps: A(lps), length: N(length) });
            step("set", `lps[${i}] = ${length}.`,
                [ array(global::S, s).ptr("i", i).done(before(i)), array(global::LPS, lps).hl([i]) ]);
            i += 1;
        } else if length > 0 {
            step("fall", `${s[i]} ≠ ${s[length]} — fall back to the next-longest overlap instead of starting over.`,
                [ array(global::S, s).ptr("i", i).ptr("len", length).bad([i]) ]);
            length = lps[length - 1];
            set(#{ length: N(length) });
            step("back", `length = ${length}.`,
                [ array(global::S, s).ptr("i", i).ptr("len", length), array(global::LPS, lps) ]);
        } else {
            lps[i] = 0;
            set(#{ lps: A(lps) });
            step("zero", `Nothing overlaps here: lps[${i}] = 0.`, [ array(global::LPS, lps).hl([i]) ]);
            i += 1;
        }
    }

    let longest = lps[n - 1];
    set(#{ longest: N(longest) });
    unset("i"); unset("length");
    step("longest",
        `The last entry is what counts: a prefix of length ${longest} is also a suffix. Those two copies overlap in the middle of the string.`,
        [ array(global::S, s).hl(span(0, longest - 1)).done(span(n - longest, n - 1)),
          array(global::LPS, lps).hl([n - 1]) ]);

    let period = n - longest;
    set(#{ period: N(period) });
    step("period",
        `So shifting s left by ${n} − ${longest} = ${period} maps it onto itself: ${period} is its period.`,
        [ array(global::S, s).window(0, period - 1).hl(span(0, period - 1)),
          text("arithmetic", [`n = ${n}`, `overlap = ${longest}`, `period = ${period}`]) ]);

    let ok = longest > 0 && n % period == 0;
    log(`${n} % ${period} = ${n % period}`, "result");
    result(if ok { "true" } else { "false" });
    if ok {
        step("ret", `${period} divides ${n} exactly ${n / period} times, so s is "${input.s.sub_string(0, period)}" repeated ${n / period} times. true`,
            [ array(global::S, s).done(seq(0, n - 1)).hl(span(0, period - 1)),
              text("arithmetic", [`${n} % ${period} = 0 ✓`, `blocks = ${n / period}`]) ],
            "return");
    } else {
        step("ret", `Period ${period} does not tile ${n} evenly (or there is no overlap at all), so no repetition builds s. false`,
            [ array(global::S, s).bad(seq(0, n - 1)),
              text("arithmetic", [`${n} % ${period} = ${n % period}`, `overlap = ${longest}`]) ],
            "return");
    }
    leave(if ok { "true" } else { "false" });
}
