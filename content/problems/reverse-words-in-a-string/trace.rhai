// Reverse Words in a String — read right to left and the reversal is free.
//
// Everything awkward about this problem is the whitespace, and reading
// backwards turns it into two tiny loops: skip spaces, then take everything
// that is not a space. No split, no filter, no list to reverse afterwards.

const S = "s   (· marks a space)";

fn dots(cs) {
    let out = [];
    for c in cs { out.push(if c == " " { "·" } else { c }); }
    out
}

fn trace(input) {
    let raw = chars(input.s);
    let s = dots(raw);
    let n = raw.len();

    enter("reverseWords", #{ s: S(input.s) });

    let parts = [];
    step("init", "The output is built left to right while the input is read right to left — that is the whole reversal.",
        [ array(global::S, s), text("output", ["(empty)"]) ]);

    let i = n - 1;
    set(#{ i: N(i) });
    step("start", `Start at the last character, index ${i}.`,
        [ array(global::S, s).ptr("i", i) ]);

    while i >= 0 {
        step("loop", `i = ${i}: look for the next word, moving left.`,
            [ array(global::S, s).ptr("i", i).done(after(i, n)),
              text("output", if parts.is_empty() { ["(empty)"] } else { [join(parts, " ")] }) ]);

        let skipped = 0;
        while i >= 0 && raw[i] == " " {
            i -= 1;
            skipped += 1;
        }
        set(#{ i: N(i) });
        if skipped > 0 {
            step("skip", `Skipped ${plural(skipped, "space")} — leading, trailing and repeated spaces all vanish here, with no special case for any of them.`,
                [ array(global::S, s).ptr("i", i).done(after(i, n)) ]);
        } else {
            step("skip", "No spaces to skip at this position.",
                [ array(global::S, s).ptr("i", i).done(after(i, n)) ]);
        }

        if i < 0 {
            step("stop", "Nothing left but spaces — the scan is done.",
                [ array(global::S, s).done(seq(0, n - 1)),
                  text("output", [join(parts, " ")]) ]);
            break;
        }

        let end = i;
        set(#{ end: N(end) });
        step("end", `A word ends at index ${end} (${s[end]}).`,
            [ array(global::S, s).ptr("i", i).ptr("end", end).hl([end]) ]);

        while i >= 0 && raw[i] != " " {
            i -= 1;
        }
        set(#{ i: N(i) });
        step("word", `Its first character is at index ${i + 1}; i now sits at ${i}, just before it.`,
            [ array(global::S, s).ptr("i", i).ptr("end", end).hl(span(i + 1, end)) ]);

        if !parts.is_empty() {
            step("sep", "One space between words — exactly one, because the output is built rather than copied.",
                [ text("output", [join(parts, " ") + " "]) ]);
        } else {
            step("sep", "First word out, so no separator yet.",
                [ text("output", ["(empty)"]) ]);
        }

        let w = "";
        for k in (i + 1)..(end + 1) { w += raw[k]; }
        parts.push(w);
        log(`word: ${w}`);
        step("take", `Append "${w}". It is the ${parts.len()}${if parts.len() == 1 { "st" } else { "th" }} word of the answer and the ${parts.len()}${if parts.len() == 1 { "st" } else { "th" }} from the end of the input.`,
            [ array(global::S, s).done(after(i, n)).hl(span(i + 1, end)),
              text("output", [join(parts, " ")]) ]);
    }

    let out = join(parts, " ");
    log(out, "result");
    result(out);
    step("ret", `"${out}" — one pass, no split, no reversal step.`,
        [ text("output", [out]) ], "return");
    leave(out);
}
