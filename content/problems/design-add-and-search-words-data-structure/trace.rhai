fn new_trie() {
    [#{ ch: "·", word: false, kids: [] }]
}

fn child_of(nodes, id, c) {
    for k in nodes[id].kids {
        if k[0] == c { return k[1]; }
    }
    ()
}

fn add_child(nodes, id, c) {
    let fresh = nodes.len();
    nodes.push(#{ ch: c, word: false, kids: [] });
    let kids = nodes[id].kids;
    kids.push([c, fresh]);
    nodes[id].kids = kids;
    [nodes, fresh]
}

fn trie_nodes(nodes) {
    let out = [];
    for i in 0..nodes.len() {
        out.push(#{ id: i, label: if nodes[i].word { `${nodes[i].ch}*` } else { nodes[i].ch } });
    }
    out
}

fn trie_edges(nodes) {
    let out = [];
    for i in 0..nodes.len() {
        for k in nodes[i].kids { out.push([i, k[1], true]); }
    }
    out
}

// Add and Search Words — the wildcard is the only place that branches.
//
// A trie makes an exact lookup a walk: one character, one link, no searching.
// A '.' breaks that, because any child might be the right one — so the search
// becomes a recursion that tries them all. What keeps this affordable is that
// the branching is confined to the dot positions; every ordinary character
// still narrows the search to a single node, so a query with one or two dots
// explores only a handful of paths rather than the whole tree.
//
// The star on a node below marks the end of a stored word — needed because
// "ba" being a prefix of "bad" does not make it a stored word.

fn match_at(nodes, id, cs, i, path) {
    enter(`match("${if i < cs.len() { cs[i] } else { "" }}", ${i})`, #{ i: N(i) });

    if i == cs.len() {
        step("end", "The pattern is exhausted.",
            [ graph("trie", trie_nodes(nodes), trie_edges(nodes)).cur(id).done(path) ]);
        step("atWord",
            if nodes[id].word { "And this node marks the end of a stored word, so it is a match." }
            else { "But this node is only a prefix, not a stored word — so it does not count.".to_string() },
            [ graph("trie", trie_nodes(nodes), trie_edges(nodes)).cur(id).done(path) ], "return");
        leave(`${nodes[id].word}`);
        return nodes[id].word;
    }

    let c = cs[i];
    step("char", `The pattern's next character is "${c}".`,
        [ graph("trie", trie_nodes(nodes), trie_edges(nodes)).cur(id).done(path) ]);

    if c == "." {
        step("wildcard", `"." matches anything, so any of this node's ${plural(nodes[id].kids.len(), "child")} could be right.`,
            [ graph("trie", trie_nodes(nodes), trie_edges(nodes)).cur(id)
                .frontier(kid_ids(nodes, id)).done(path) ]);

        for k in nodes[id].kids {
            step("tryAll", `Try the "${k[0]}" branch.`,
                [ graph("trie", trie_nodes(nodes), trie_edges(nodes)).cur(k[1]).done(path) ]);
            path.push(k[1]);
            if match_at(nodes, k[1], cs, i + 1, path) {
                step("branch", `The "${k[0]}" branch matched, so nothing else needs trying.`,
                    [ graph("trie", trie_nodes(nodes), trie_edges(nodes)).done(path) ], "return");
                leave("true");
                return true;
            }
            path.truncate(path.len() - 1);
        }

        step("noBranch", "No child led to a match.",
            [ graph("trie", trie_nodes(nodes), trie_edges(nodes)).cur(id).done(path) ], "return");
        leave("false");
        return false;
    }

    let next = child_of(nodes, id, c);
    step("exact", `An ordinary character, so exactly one link can be followed.`,
        [ graph("trie", trie_nodes(nodes), trie_edges(nodes)).cur(id).done(path) ]);

    if type_of(next) == "()" {
        step("dead", `No "${c}" branch exists here, so this path is dead.`,
            [ graph("trie", trie_nodes(nodes), trie_edges(nodes)).cur(id).bad([id]).done(path) ],
            "return");
        leave("false");
        return false;
    }

    path.push(next);
    step("recurse", `Follow "${c}" — no branching, so the search stays narrow.`,
        [ graph("trie", trie_nodes(nodes), trie_edges(nodes)).cur(next).done(path) ], "call");
    let ok = match_at(nodes, next, cs, i + 1, path);
    leave(`${ok}`);
    ok
}

fn kid_ids(nodes, id) {
    let out = [];
    for k in nodes[id].kids { out.push(k[1]); }
    out
}

fn trace(input) {
    let nodes = new_trie();
    let out = [];

    enter("WordDictionary", #{});
    step("init", "A trie: one node per distinct prefix, so common prefixes are stored once.",
        [ graph("trie", trie_nodes(nodes), trie_edges(nodes)) ]);
    step("flag", "A star marks a node where a stored word ends — being a prefix is not the same as being a word.",
        [ graph("trie", trie_nodes(nodes), trie_edges(nodes)) ]);

    for raw in input.ops.split(",") {
        let parts = words(raw);
        if parts.len() < 2 { continue; }

        if parts[0] == "add" {
            let word = parts[1];
            enter(`addWord("${word}")`, #{ word: S(word) });

            let node = 0;
            step("addStart", "Start at the root.",
                [ graph("trie", trie_nodes(nodes), trie_edges(nodes)).cur(node) ]);

            for c in chars(word) {
                step("addLoop", `Character "${c}".`,
                    [ graph("trie", trie_nodes(nodes), trie_edges(nodes)).cur(node) ]);

                let next = child_of(nodes, node, c);
                if type_of(next) == "()" {
                    step("missing", `No "${c}" branch here yet.`,
                        [ graph("trie", trie_nodes(nodes), trie_edges(nodes)).cur(node) ]);
                    let made = add_child(nodes, node, c);
                    nodes = made[0];
                    next = made[1];
                    step("create", `Create one. Words sharing this prefix will reuse it.`,
                        [ graph("trie", trie_nodes(nodes), trie_edges(nodes)).cur(next) ]);
                } else {
                    step("missing", `A "${c}" branch already exists — shared with another word.`,
                        [ graph("trie", trie_nodes(nodes), trie_edges(nodes)).cur(next) ]);
                    step("create", "Nothing to create.",
                        [ graph("trie", trie_nodes(nodes), trie_edges(nodes)).cur(next) ]);
                }

                node = next;
                step("descend", `Move down to "${c}".`,
                    [ graph("trie", trie_nodes(nodes), trie_edges(nodes)).cur(node) ]);
            }

            nodes[node].word = true;
            log(`added ${word}`);
            step("mark", `Mark this node: "${word}" ends here.`,
                [ graph("trie", trie_nodes(nodes), trie_edges(nodes)).cur(node) ], "return");
            leave("");
            continue;
        }

        let pattern = parts[1];
        enter(`search("${pattern}")`, #{ pattern: S(pattern) });
        step("searchStart", `Match "${pattern}" from the root.`,
            [ graph("trie", trie_nodes(nodes), trie_edges(nodes)).cur(0) ], "call");
        let ok = match_at(nodes, 0, chars(pattern), 0, [0]);
        out.push(`${ok}`);
        log(`search("${pattern}") -> ${ok}`, "result");
        leave(`${ok}`);
    }

    let answer = join(out, " ");
    result(answer);
    leave(answer);
}
