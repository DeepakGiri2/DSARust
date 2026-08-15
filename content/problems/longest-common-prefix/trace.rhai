// Longest Common Prefix — compare column by column, not word by word.
//
// Taking one character position at a time and checking it across every word
// means the answer is found the instant the first disagreement appears; there
// is no need to finish reading any word.

fn trace(input) {
    let words = words(input.s);
    let first = chars(words[0]);
    let prefix = "";

    enter("longestCommonPrefix", #{ count: N(words.len()) });
    set(#{ prefix: S(prefix) });
    step("init", "Build the prefix one column at a time.",
        [ text("words", words), array("words[0]", first) ]);

    for j in 0..first.len() {
        set(#{ j: N(j) });
        step("col", `Column ${j}.`,
            [ text("words", words), array("words[0]", first).ptr("j", j).hl(before(j)) ]);

        let c = first[j];
        step("pick", `The first word has '${c}' at position ${j}. Every other word must agree.`,
            [ text("words", words), array("words[0]", first).ptr("j", j).hl([j]) ]);

        for k in 1..words.len() {
            let w = chars(words[k]);
            step("row", `Check word ${k} ("${words[k]}").`,
                [ text("words", words).hl([k]),
                  array("words[0]", first).ptr("j", j).hl([j]) ]);

            if j >= w.len() || w[j] != c {
                let why = if j >= w.len() {
                    `"${words[k]}" is only ${w.len()} long, so it has no column ${j}`
                } else {
                    `"${words[k]}" has '${w[j]}' at column ${j}, not '${c}'`
                };
                step("mismatch", `${why} — the common prefix cannot grow past ${j}.`,
                    [ text("words", words).hl([k]),
                      array("words[0]", first).bad([j]).hl(before(j)) ]);
                log(`mismatch at column ${j}`, "result");
                result(if prefix == "" { "\"\"" } else { prefix });
                step("ret", `Return "${prefix}".`,
                    [ array("words[0]", first).hl(before(j)) ], "return");
                leave(prefix);
                return;
            }
        }

        prefix += c;
        set(#{ prefix: S(prefix) });
        log(`prefix = ${prefix}`);
        step("grow", `Every word agrees on '${c}': prefix = "${prefix}".`,
            [ text("words", words), array("words[0]", first).hl(span(0, j)) ]);
    }

    log(`prefix = ${prefix}`, "result");
    result(if prefix == "" { "\"\"" } else { prefix });
    step("done", `The first word ran out, so it is itself the common prefix: "${prefix}".`,
        [ array("words[0]", first).hl(seq(0, first.len() - 1)) ], "return");
    leave(prefix);
}
