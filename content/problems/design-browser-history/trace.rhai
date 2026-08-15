// Design Browser History — a line with a cursor, and one destructive rule.
//
// Back and forward are trivial cursor arithmetic. The rule that actually
// shapes the data structure is what a *visit* does: everything ahead of the
// cursor stops being reachable, so it is thrown away before the new page is
// appended. Without that truncation, a forward after a visit would resurrect a
// page the user can no longer reach.

fn trace(input) {
    let pages = [input.homepage];
    let cur = 0;
    let out = [];

    enter("BrowserHistory", #{ homepage: S(input.homepage) });
    step("init", `History starts with just the homepage, "${input.homepage}".`,
        [ array("history", pages) ]);
    set(#{ cur: N(cur) });
    step("cur", "The cursor marks the page currently shown.",
        [ array("history", pages).ptr("cur", cur).hl([cur]) ]);

    for raw in input.ops.split(",") {
        let parts = words(raw);
        if parts.len() == 0 { continue; }

        if parts[0] == "visit" {
            let url = parts[1];
            enter(`visit("${url}")`, #{ url: S(url) });

            let lost = pages.len() - 1 - cur;
            pages.truncate(cur + 1);
            step("truncate",
                if lost > 0 {
                    `Visiting throws away the ${plural(lost, "page")} ahead of the cursor — they are no longer reachable by any sequence of clicks.`
                } else {
                    "Nothing is ahead of the cursor, so there is nothing to discard.".to_string()
                },
                [ array("history", pages).ptr("cur", cur).hl([cur]) ]);

            pages.push(url);
            step("append", `Append "${url}".`,
                [ array("history", pages).ptr("cur", cur).hl([pages.len() - 1]) ]);

            cur += 1;
            set(#{ cur: N(cur) });
            log(`visit ${url}`);
            step("moveCur", `The cursor follows it to index ${cur}.`,
                [ array("history", pages).ptr("cur", cur).hl([cur]) ], "return");
            leave("");
            continue;
        }

        let steps = num(parts[1]);

        if parts[0] == "back" {
            enter(`back(${steps})`, #{ steps: N(steps) });
            let want = steps;
            if steps > cur {
                steps = cur;
                step("clampBack", `Only ${cur} step(s) of history exist behind the cursor, so ${want} is clamped to ${steps} rather than being an error.`,
                    [ array("history", pages).ptr("cur", cur).hl([cur]) ]);
            } else {
                step("clampBack", `${steps} step(s) back is within the history.`,
                    [ array("history", pages).ptr("cur", cur).hl([cur]) ]);
            }
            cur -= steps;
            set(#{ cur: N(cur) });
            step("stepBack", `Cursor moves to index ${cur}.`,
                [ array("history", pages).ptr("cur", cur).hl([cur]) ]);
            out.push(pages[cur]);
            log(`back(${want}) -> ${pages[cur]}`, "result");
            step("retBack", `Showing "${pages[cur]}". Nothing was deleted — the pages ahead are still reachable by going forward.`,
                [ array("history", pages).ptr("cur", cur).hl([cur]), text("output", out) ], "return");
            leave(pages[cur]);
            continue;
        }

        enter(`forward(${steps})`, #{ steps: N(steps) });
        let want = steps;
        if cur + steps >= pages.len() {
            steps = pages.len() - 1 - cur;
            step("clampFwd", `Only ${steps} step(s) of history exist ahead, so ${want} is clamped to ${steps}.`,
                [ array("history", pages).ptr("cur", cur).hl([cur]) ]);
        } else {
            step("clampFwd", `${steps} step(s) forward stays inside the history.`,
                [ array("history", pages).ptr("cur", cur).hl([cur]) ]);
        }
        cur += steps;
        set(#{ cur: N(cur) });
        step("stepFwd", `Cursor moves to index ${cur}.`,
            [ array("history", pages).ptr("cur", cur).hl([cur]) ]);
        out.push(pages[cur]);
        log(`forward(${want}) -> ${pages[cur]}`, "result");
        step("retFwd", `Showing "${pages[cur]}".`,
            [ array("history", pages).ptr("cur", cur).hl([cur]), text("output", out) ], "return");
        leave(pages[cur]);
    }

    let answer = join(out, " ");
    result(answer);
    leave(answer);
}
