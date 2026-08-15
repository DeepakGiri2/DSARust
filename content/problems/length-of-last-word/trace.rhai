// Length of Last Word — scan backwards, in two phases.
//
// Starting from the end means the last word is found without knowing how many
// words there are. Phase one steps over the trailing spaces; phase two counts
// until the next space.

fn trace(input) {
    let cs = chars(input.s);
    let n = cs.len();
    let i = n - 1;
    let length = 0;
    let counted = [];

    enter("lengthOfLastWord", #{ s: S(input.s) });
    set(#{ i: N(i) });
    step("start", "Start at the very end of the string.", [ array("s", cs).ptr("i", i) ]);
    set(#{ length: N(length) });
    step("len0", "Nothing counted yet.", [ array("s", cs).ptr("i", i) ]);

    while i >= 0 && cs[i] == " " {
        step("skip", `Position ${i} is a trailing space — it is not part of any word.`,
            [ array("s", cs).ptr("i", i).bad([i]).done(after(i, n)) ]);
        i -= 1;
        set(#{ i: N(i) });
        step("dec", `i = ${i}.`, [ array("s", cs).ptr("i", i).done(after(i, n)) ]);
    }

    while i >= 0 && cs[i] != " " {
        counted.push(i);
        step("count", `'${cs[i]}' belongs to the last word.`,
            [ array("s", cs).ptr("i", i).hl(counted).done(after(counted[0], n)) ]);
        length += 1;
        set(#{ length: N(length), i: N(i - 1) });
        step("inc", `length = ${length}.`,
            [ array("s", cs).ptr("i", i - 1).hl(counted) ]);
        i -= 1;
    }

    log(`last word length ${length}`, "result");
    result(`${length}`);
    step("ret",
        if length == 0 {
            "The string held no word at all.".to_string()
        } else {
            `The last word is ${length} character(s) long — reached without ever splitting the string.`
        },
        [ array("s", cs).hl(counted) ], "return");
    leave(`${length}`);
}
