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

// Replace Words — depth in the trie *is* length, so the first hit is shortest.
//
// Every step down the trie extends the prefix by exactly one character. So the
// prefixes are examined in increasing length automatically, and the first node
// marked as a dictionary word is the shortest root — there is no set of
// candidates to compare and no minimum to track.
//
// The other economy is the early stop: as soon as the next character has no
// link, no longer prefix can be in the dictionary either, so the walk ends.

fn build(nodes, word) {
    let node = 0;
    for c in chars(word) {
        let next = child_of(nodes, node, c);
        if type_of(next) == "()" {
            let made = add_child(nodes, node, c);
            nodes = made[0];
            next = made[1];
        }
        node = next;
    }
    nodes[node].word = true;
    nodes
}

fn prefix_of(cs, k) {
    let out = "";
    for i in 0..k { out += cs[i]; }
    out
}

fn shortest_root(nodes, word) {
    enter(`shortestRoot("${word}")`, #{ word: S(word) });

    let cs = chars(word);
    let node = 0;
    let path = [0];
    step("start", "Start at the root of the trie.",
        [ graph("trie (· = root, * = dictionary word)", trie_nodes(nodes), trie_edges(nodes)).cur(node),
          array("word", cs) ]);

    for i in 0..cs.len() {
        step("walk", `Character ${i} of "${word}" is "${cs[i]}"; the prefix so far is "${prefix_of(cs, i)}".`,
            [ graph("trie (· = root, * = dictionary word)", trie_nodes(nodes), trie_edges(nodes))
                .cur(node).done(path),
              array("word", cs).ptr("i", i).done(before(i)) ]);

        let next = child_of(nodes, node, cs[i]);
        if type_of(next) == "()" {
            step("absent", `No "${cs[i]}" branch here, so no dictionary root shares this prefix — and no longer prefix could either. Stop.`,
                [ graph("trie (· = root, * = dictionary word)", trie_nodes(nodes), trie_edges(nodes))
                    .cur(node).bad([node]).done(path) ]);
            break;
        }

        node = next;
        path.push(node);
        step("descend", `Follow it: the prefix is now "${prefix_of(cs, i + 1)}".`,
            [ graph("trie (· = root, * = dictionary word)", trie_nodes(nodes), trie_edges(nodes))
                .cur(node).done(path) ]);

        if nodes[node].word {
            step("isRoot", `This node is marked, so "${prefix_of(cs, i + 1)}" is in the dictionary. Because depth equals length, it is necessarily the *shortest* such root.`,
                [ graph("trie (· = root, * = dictionary word)", trie_nodes(nodes), trie_edges(nodes))
                    .cur(node).done(path) ]);
            let root = prefix_of(cs, i + 1);
            log(`${word} -> ${root}`);
            step("shortest", `Replace "${word}" with "${root}".`,
                [ array("word", cs).hl(seq(0, i)) ], "return");
            leave(root);
            return root;
        }
        step("isRoot", `"${prefix_of(cs, i + 1)}" is a prefix of some root but not a root itself.`,
            [ graph("trie (· = root, * = dictionary word)", trie_nodes(nodes), trie_edges(nodes))
                .cur(node).done(path) ]);
    }

    step("whole", `No root is a prefix of "${word}", so it is left alone.`,
        [ array("word", cs) ], "return");
    leave(word);
    word
}

fn trace(input) {
    let dict = words(input.dictionary);
    let sentence = words(input.sentence);

    enter("replaceWords", #{});

    let nodes = new_trie();
    for w in dict { nodes = build(nodes, w); }
    step("build", `${plural(dict.len(), "root")} stored: ${join(dict, ", ")}. Shared prefixes share nodes.`,
        [ graph("trie (· = root, * = dictionary word)", trie_nodes(nodes), trie_edges(nodes)) ]);

    let out = [];
    step("init", "The rewritten sentence.",
        [ array("sentence", sentence), text("output", out) ]);

    for i in 0..sentence.len() {
        step("loop", `Word ${i}: "${sentence[i]}".`,
            [ array("sentence", sentence).ptr("i", i).done(before(i)) ], "call");
        let root = shortest_root(nodes, sentence[i]);
        out.push(root);
        step("replace", `"${sentence[i]}" becomes "${root}".`,
            [ text("output", [join(out, " ")]) ]);
    }

    let answer = join(out, " ");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `Result: ${answer}.`, [ text("output", [answer]) ], "return");
    leave(answer);
}
