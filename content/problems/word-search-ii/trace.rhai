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

// Word Search II — one board walk, all the words at once.
//
// Running an ordinary word search per word repeats the same board traversals
// for every word sharing a prefix. Descending the trie in step with the board
// removes that repetition entirely: the position in the trie *is* the set of
// words still possible.
//
// The pruning is what makes it fast. As soon as the path spells something that
// no dictionary word begins with, the trie has no link to follow and the whole
// branch dies immediately — no matter how much board is left.
//
// Clearing the word flag on a hit is a neat dedup: the same word found by a
// second path simply will not register.

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

fn dfs(board, r, c, nodes, node, prefix, res, path) {
    if r < 0 || r >= board.len() || c < 0 || c >= board[0].len() {
        enter(`dfs(${r},${c})`);
        step("bounds", `(${r},${c}) is off the board.`, [ grid("board", board).path(path) ], "return");
        leave("");
        return [board, nodes, res];
    }

    enter(`dfs(${r},${c})`, #{ prefix: S(prefix) });
    step("bounds", `(${r},${c}) is on the board.`,
        [ grid("board", board).cur([r, c]).path(path) ]);

    let ch = board[r][c];
    step("cell", `It holds "${ch}".`,
        [ grid("board", board).cur([r, c]).path(path) ]);

    let next = if ch == "#" { () } else { child_of(nodes, node, ch) };
    if type_of(next) == "()" {
        step("prune",
            if ch == "#" { "That cell is already on the current path, and a letter cannot be reused.".to_string() }
            else { `No dictionary word continues "${prefix}" with "${ch}", so this branch is finished — however much board is left.` },
            [ grid("board", board).cur([r, c]).bad([[r, c]]).path(path),
              graph("trie", trie_nodes(nodes), trie_edges(nodes)).cur(node) ], "return");
        leave("");
        return [board, nodes, res];
    }

    node = next;
    prefix += ch;
    path.push([r, c]);
    step("advance", `Descend the trie: the path now spells "${prefix}", which is still a prefix of something.`,
        [ grid("board", board).cur([r, c]).path(path),
          graph("trie", trie_nodes(nodes), trie_edges(nodes)).cur(node) ]);

    if nodes[node].word {
        step("found", `"${prefix}" is a whole dictionary word.`,
            [ grid("board", board).path(path),
              graph("trie", trie_nodes(nodes), trie_edges(nodes)).cur(node) ]);
        res.push(prefix);
        log(`found ${prefix}`, "result");
        step("record", `Record it.`, [ text("words found", res) ]);
        nodes[node].word = false;
        step("dedup", "Clear the flag so a second path spelling the same word does not report it again.",
            [ graph("trie", trie_nodes(nodes), trie_edges(nodes)).cur(node) ]);
    }

    grid_set(board, r, c, "#");
    step("mark", "Block the cell so the path cannot reuse it.",
        [ grid("board", board).path(path) ]);

    let out = dfs(board, r + 1, c, nodes, node, prefix, res, path);
    board = out[0]; nodes = out[1]; res = out[2];
    step("down", "Down explored.", [ grid("board", board).path(path) ]);

    out = dfs(board, r - 1, c, nodes, node, prefix, res, path);
    board = out[0]; nodes = out[1]; res = out[2];
    step("up", "Up explored.", [ grid("board", board).path(path) ]);

    out = dfs(board, r, c + 1, nodes, node, prefix, res, path);
    board = out[0]; nodes = out[1]; res = out[2];
    step("right", "Right explored.", [ grid("board", board).path(path) ]);

    out = dfs(board, r, c - 1, nodes, node, prefix, res, path);
    board = out[0]; nodes = out[1]; res = out[2];
    step("left", "Left explored.", [ grid("board", board).path(path) ]);

    grid_set(board, r, c, ch);
    path.truncate(path.len() - 1);
    step("restore", `Put "${ch}" back — it is free for other paths.`,
        [ grid("board", board).path(path) ], "return");

    leave("");
    [board, nodes, res]
}

fn trace(input) {
    let board = grid_rows(input.board);
    let ws = words(input.words);
    let rows = board.len();
    let cols = board[0].len();

    enter("findWords", #{ rows: N(rows), cols: N(cols) });

    let nodes = new_trie();
    for w in ws { nodes = build(nodes, w); }
    step("build", `All ${plural(ws.len(), "word")} in one trie, so a single board walk searches for every one of them at once.`,
        [ grid("board", board),
          graph("trie", trie_nodes(nodes), trie_edges(nodes)) ]);

    let res = [];
    step("init", "Words found.", [ text("words found", res) ]);

    for r in 0..rows {
        step("rowLoop", `Row ${r}.`, [ grid("board", board).cur([r, 0]) ]);
        for c in 0..cols {
            step("colLoop", `Start a path at (${r},${c}).`,
                [ grid("board", board).cur([r, c]) ]);
            step("call", `Walk from (${r},${c}).`, [ grid("board", board).cur([r, c]) ], "call");
            let out = dfs(board, r, c, nodes, 0, "", res, []);
            board = out[0];
            nodes = out[1];
            res = out[2];
        }
    }

    let answer = join(res, " ");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `${plural(res.len(), "word")} found: ${answer}.`,
        [ grid("board", board), text("words found", res) ], "return");
    leave(answer);
}
