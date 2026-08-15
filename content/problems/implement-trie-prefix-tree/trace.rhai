// Implement Trie — one node per character, shared prefixes shared.
//
// The words "apple" and "app" travel the same nodes for their first three
// characters. That is why `search` and `startsWith` differ by exactly one
// thing: whether the node reached is marked as the end of a word.

fn trace(input) {
    // Nodes are rows of "path -> children, isEnd"; the path doubles as the id.
    let children = map_new();
    let ends = set_new();

    enter("Trie", #{});
    step("insLoop", "A trie starts as a single empty root.",
        [ kv("nodes — path → children", children) ]);

    for op in input.ops.split(",") {
        let parts = words(op);
        if parts.len() < 2 { continue; }
        let cmd = parts[0];
        let word = parts[1];
        let cs = chars(word);

        if cmd == "insert" {
            enter(`Insert("${word}")`, #{ word: S(word) });
            let path = "";
            for i in 0..cs.len() {
                let c = cs[i];
                let next = path + c;
                step("insLoop", `Character '${c}' of "${word}".`,
                    [ array("word", cs).ptr("i", i).hl([i]),
                      kv("nodes — path → children", children) ]);

                if !map_has(children, path) || !map_get(children, path).contains(c) {
                    let existing = if map_has(children, path) { map_get(children, path) } else { "" };
                    map_set(children, path, existing + c);
                    step("insMiss", `No '${c}' branch under "${if path == "" { "root" } else { path }}" — create it.`,
                        [ array("word", cs).ptr("i", i),
                          kv("nodes — path → children", children).hl([path]) ]);
                } else {
                    step("insMiss", `A '${c}' branch already exists under "${if path == "" { "root" } else { path }}" — reuse it. This is the sharing that makes a trie compact.`,
                        [ array("word", cs).ptr("i", i),
                          kv("nodes — path → children", children).hl([path]) ]);
                }
                path = next;
                step("insDown", `Descend to "${path}".`,
                    [ array("word", cs).ptr("i", i).hl(before(i + 1)),
                      kv("nodes — path → children", children) ]);
            }
            set_add(ends, path);
            log(`inserted ${word}`);
            step("insEnd", `Mark "${path}" as the end of a word. Nothing else about the node changes.`,
                [ kv("nodes — path → children", children),
                  kv("end-of-word", ends).hl([path]) ]);
            leave("");
        } else {
            let looking_for_word = cmd == "search";
            enter(`${cmd}("${word}")`, #{ word: S(word) });
            let path = "";
            let failed = false;

            for i in 0..cs.len() {
                let c = cs[i];
                step(if looking_for_word { "srchLoop" } else { "swLoop" },
                    `Follow '${c}'.`,
                    [ array("query", cs).ptr("i", i).hl([i]),
                      kv("nodes — path → children", children) ]);

                if !map_has(children, path) || !map_get(children, path).contains(c) {
                    step(if looking_for_word { "srchMiss" } else { "swMiss" },
                        `There is no '${c}' branch here, so the trie holds nothing with this prefix.`,
                        [ array("query", cs).bad([i]),
                          kv("nodes — path → children", children).bad([path]) ]);
                    log(`${cmd}("${word}") -> false`, "result");
                    step(if looking_for_word { "srchFail" } else { "swFail" }, "Return false.",
                        [ array("query", cs).bad([i]) ], "return");
                    leave("false");
                    failed = true;
                    break;
                }
                path += c;
                step(if looking_for_word { "srchDown" } else { "swDown" },
                    `Now at "${path}".`,
                    [ array("query", cs).hl(before(i + 1)),
                      kv("nodes — path → children", children).hl([path]) ]);
            }

            if !failed {
                if looking_for_word {
                    let found = set_has(ends, path);
                    log(`search("${word}") -> ${found}`, "result");
                    step("srchEnd",
                        if found {
                            `"${path}" is marked as the end of a word, so the word is present.`
                        } else {
                            `The path exists, but "${path}" was never marked as a word end — it is only a prefix.`
                        },
                        [ kv("end-of-word", ends).hl([path]) ], "return");
                    leave(`${found}`);
                } else {
                    log(`startsWith("${word}") -> true`, "result");
                    step("swEnd", `The whole prefix was walkable, and that is all startsWith asks.`,
                        [ kv("nodes — path → children", children).hl([path]) ], "return");
                    leave("true");
                }
            }
        }
    }

    result(`${map_len(children)} node(s)`);
}
