// Encode and Decode Strings — length-prefixing beats any delimiter.
//
// Any separator character could itself appear inside a word. Writing the
// length before each word removes the ambiguity entirely: the decoder is told
// exactly how many characters to take, so the content is never inspected.

fn trace(input) {
    let strs = words(input.words);

    enter("encode", #{ count: N(strs.len()) });
    let encoded = "";
    set(#{ encoded: S(encoded) });
    step("einit", "Build one string that can be split back apart unambiguously.",
        [ text("input", strs) ]);

    for i in 0..strs.len() {
        let w = strs[i];
        step("eloop", `Word ${i}: "${w}" is ${w.len()} character(s) long.`,
            [ text("input", strs).hl([i]), array("encoded", chars(encoded)) ]);
        encoded += `${w.len()}#${w}`;
        set(#{ encoded: S(encoded) });
        log(`append ${w.len()}#${w}`);
        step("eappend", `Append "${w.len()}#${w}" — the length, a marker, then the word itself.`,
            [ text("input", strs).hl([i]),
              array("encoded", chars(encoded)).hl(seq(encoded.len() - w.len() - 2, encoded.len() - 1)) ]);
    }
    log(`encoded: ${encoded}`, "result");
    step("eret", `The encoding is "${encoded}".`,
        [ array("encoded", chars(encoded)) ], "return");
    leave(encoded);

    enter("decode", #{ s: S(encoded) });
    let cs = chars(encoded);
    let res = [];
    step("dinit", "Now take it apart again — without ever looking at what the words contain.",
        [ array("encoded", cs) ]);

    let i = 0;
    set(#{ i: N(i) });
    step("di0", "Start at the beginning.", [ array("encoded", cs).ptr("i", i) ]);

    while i < cs.len() {
        step("dloop", `There is more to read from position ${i}.`,
            [ array("encoded", cs).ptr("i", i).done(before(i)), text("decoded", res) ]);

        let j = i;
        step("dj", "Find the marker that ends the length field.",
            [ array("encoded", cs).pointers(#{ i: i, j: j }).done(before(i)) ]);
        while cs[j] != "#" {
            step("dscan", `'${cs[j]}' is part of the length.`,
                [ array("encoded", cs).pointers(#{ i: i, j: j }).hl(span(i, j)) ]);
            j += 1;
        }

        let length = num(join(cs.extract(i, j - i), ""));
        set(#{ length: N(length) });
        step("dlen", `The length field reads ${length}.`,
            [ array("encoded", cs).pointers(#{ i: i, j: j }).hl(span(i, j - 1)) ]);

        let word = join(cs.extract(j + 1, length), "");
        step("dword", `Take exactly ${length} character(s) after the marker: "${word}". A '#' inside the word would not matter.`,
            [ array("encoded", cs).hl(span(j + 1, j + length)) ]);

        res.push(word);
        step("dadd", `Recovered "${word}".`,
            [ array("encoded", cs).hl(span(j + 1, j + length)), text("decoded", res) ]);

        i = j + 1 + length;
        set(#{ i: N(i) });
        step("dnext", `Jump straight to position ${i}.`,
            [ array("encoded", cs).ptr("i", i).done(before(i)), text("decoded", res) ]);
    }

    let answer = join(res, " ");
    log(`decoded: ${answer}`, "result");
    result(answer);
    step("dret", `Round trip complete: ${answer}.`, [ text("decoded", res) ], "return");
    leave(answer);
}
