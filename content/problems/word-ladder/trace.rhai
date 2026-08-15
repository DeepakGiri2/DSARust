// Word Ladder — an unweighted shortest path, so breadth-first.
//
// The graph is never built explicitly: two words are neighbours when they
// differ in one letter, and that is cheap enough to test on demand. What
// matters is the order of exploration. BFS finishes an entire level before
// starting the next, so the first time the target appears it is at the
// smallest possible depth. Marking a word used the moment it is queued stops
// it being reached again by a longer route.

fn differs_by_one(a, b) {
    if a.len() != b.len() { return false; }
    let ca = chars(a);
    let cb = chars(b);
    let diff = 0;
    for i in 0..ca.len() { if ca[i] != cb[i] { diff += 1; } }
    diff == 1
}

fn unused_words(words, used) {
    let out = [];
    for i in 0..words.len() { if !used[i] { out.push(i); } }
    out
}

fn trace(input) {
    let begin = input.beginWord;
    let end = input.endWord;
    let words = words(input.wordList);
    let n = words.len();

    enter("ladderLength", #{});

    let used = [];
    for i in 0..n { used.push(false); }
    step("init", "A flag per word: once a word has been queued, no later route to it can be shorter.",
        [ array("word list", words), text("search", [`begin: ${begin}`, `end: ${end}`]) ]);

    let target = -1;
    set(#{ target: N(target) });
    step("target", `Is "${end}" even in the list?`, [ array("word list", words) ]);

    for i in 0..n {
        if words[i] == end { target = i; }
    }
    step("findTarget",
        if target >= 0 { `Yes, at index ${target}.` } else { `No.`.to_string() },
        [ array("word list", words).hl(if target >= 0 { [target] } else { [] }) ]);

    if target < 0 {
        log("target not in the list", "result");
        step("absent", `"${end}" is not in the list, so no chain can ever end there.`,
            [ array("word list", words) ], "return");
        result("0");
        leave("0");
        return;
    }
    step("absent", `"${end}" is present, so a chain may exist.`,
        [ array("word list", words).hl([target]) ]);

    let q = [begin];
    step("seed", `Start the frontier with "${begin}".`,
        [ queue("frontier", q), array("word list", words) ]);

    let steps = 1;
    set(#{ steps: N(steps) });
    step("steps", "The chain length counts words, not changes, so it starts at 1.",
        [ queue("frontier", q), text("search", [`chain length so far: ${steps}`]) ]);

    let answer = 0;
    while q.len() > 0 {
        step("loop", `Level ${steps}: ${plural(q.len(), "word")} to expand.`,
            [ queue("frontier", q),
              array("word list", words).done(claimed(used)),
              text("search", [`chain length so far: ${steps}`]) ]);

        let next = [];
        step("level", "Collect the next level separately, so the levels stay distinct.",
            [ queue("frontier", q), queue("next level", next) ]);

        let broke = false;
        for w in q {
            step("scan", `Expanding "${w}".`,
                [ queue("frontier", q), array("word list", words).done(claimed(used)) ]);

            if w == end {
                answer = steps;
                log(`reached ${end} in ${steps}`, "result");
                step("found", `"${w}" is the target, and BFS reached it at level ${steps} — no shorter chain exists.`,
                    [ array("word list", words).hl([target]).done(claimed(used)) ]);
                step("ret", `The chain has ${plural(steps, "word")}.`,
                    [ text("search", [`answer: ${steps}`]) ], "return");
                broke = true;
                break;
            }

            for i in 0..n {
                step("candidates", `Could "${w}" become "${words[i]}"?`,
                    [ array("word list", words).ptr("i", i).done(claimed(used)) ]);

                if used[i] {
                    step("skip", `"${words[i]}" was already claimed at an earlier or equal level, so any route through it now would be longer.`,
                        [ array("word list", words).ptr("i", i).done(claimed(used)) ]);
                    continue;
                }

                if differs_by_one(w, words[i]) {
                    step("adjacent", `"${w}" and "${words[i]}" differ in exactly one letter, so they are neighbours.`,
                        [ array("word list", words).ptr("i", i).hl([i]).done(claimed(used)) ]);
                    used[i] = true;
                    step("consume", `Claim "${words[i]}" immediately — that is what keeps each word to a single expansion.`,
                        [ array("word list", words).done(claimed(used)) ]);
                    next.push(words[i]);
                    step("enqueue", `Queue it for level ${steps + 1}.`,
                        [ queue("next level", next) ]);
                } else {
                    step("adjacent", `They differ in more than one letter, so no single change connects them.`,
                        [ array("word list", words).ptr("i", i).done(claimed(used)) ]);
                }
            }
        }

        if broke { break; }

        q = next;
        step("advance", `Move to the next level: ${plural(q.len(), "word")}.`,
            [ queue("frontier", q) ]);
        steps += 1;
        set(#{ steps: N(steps) });
        step("bump", `Chain length is now ${steps}.`,
            [ text("search", [`chain length so far: ${steps}`]) ]);
    }

    if answer == 0 {
        log("no chain", "result");
        step("fail", `The frontier emptied without reaching "${end}", so no chain of single-letter changes exists.`,
            [ array("word list", words).done(claimed(used)) ], "return");
    }

    result(`${answer}`);
    leave(`${answer}`);
}

fn claimed(used) {
    let out = [];
    for i in 0..used.len() { if used[i] { out.push(i); } }
    out
}
