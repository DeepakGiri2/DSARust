// Group Anagrams — a canonical key per word.
//
// Two words are anagrams exactly when their sorted letters match, so the
// sorted string is a fingerprint. Grouping then costs one hash lookup per
// word instead of comparing every pair.

fn sorted_key(word) {
    let cs = chars(word);
    for i in 1..cs.len() {
        let j = i;
        while j > 0 && cs[j - 1] > cs[j] {
            swap_at(cs, j - 1, j);
            j -= 1;
        }
    }
    join(cs, "")
}

fn trace(input) {
    let words = words(input.words);
    let groups = map_new();
    let order = [];

    enter("groupAnagrams", #{ count: N(words.len()) });
    set(#{ groups: MAP(groups) });
    step("init", "A map from fingerprint to the words that share it.",
        [ text("words", words), kv("groups — key → words", groups) ]);
    step("order", "Remember the order keys first appeared so the output is stable.",
        [ text("words", words), kv("groups — key → words", groups) ]);

    for i in 0..words.len() {
        let w = words[i];
        set(#{ i: N(i), word: S(w) });
        step("loop", `Word ${i}: "${w}".`,
            [ text("words", words).hl([i]), kv("groups — key → words", groups) ]);

        let key = sorted_key(w);
        set(#{ key: S(key) });
        log(`"${w}" -> "${key}"`);
        step("key", `Sorting its letters gives "${key}". Every anagram of "${w}" produces exactly this.`,
            [ text("words", words).hl([i]), kv("groups — key → words", groups) ]);

        if !map_has(groups, key) {
            step("new", `"${key}" has not been seen before — start a new group.`,
                [ text("words", words).hl([i]), kv("groups — key → words", groups) ]);
            order.push(key);
            map_set(groups, key, w);
            set(#{ groups: MAP(groups, [key]) });
            step("addorder", `Group "${key}" now holds "${w}".`,
                [ text("words", words).hl([i]), kv("groups — key → words", groups).hl([key]) ]);
        } else {
            let existing = map_get(groups, key);
            map_set(groups, key, `${existing},${w}`);
            set(#{ groups: MAP(groups, [key]) });
            step("new", `"${key}" already exists — "${w}" is an anagram of what is in there.`,
                [ text("words", words).hl([i]), kv("groups — key → words", groups).hl([key]) ]);
            step("add", `Group "${key}" now holds ${map_get(groups, key)}.`,
                [ text("words", words).hl([i]), kv("groups — key → words", groups).hl([key]) ]);
        }
    }

    let out = [];
    step("res", "Collect the groups.", [ kv("groups — key → words", groups) ]);
    for key in order {
        out.push(`[${map_get(groups, key)}]`);
        step("collect", `Group "${key}": ${map_get(groups, key)}.`,
            [ kv("groups — key → words", groups).hl([key]), text("result", out) ]);
        step("push", `${out.len()} group(s) so far.`, [ text("result", out) ]);
    }

    let answer = `[${join(out, ", ")}]`;
    log(`answer ${answer}`, "result");
    result(answer);
    step("return", `${plural(out.len(), "group")}, found with one hash lookup per word: ${answer}.`,
        [ text("result", out) ], "return");
    leave(answer);
}
