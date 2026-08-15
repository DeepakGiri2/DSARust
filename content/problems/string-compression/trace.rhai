// String Compression — write chases read and never catches it.
//
// The safety argument is worth pausing on: a run of length L is replaced by
// 1 + digits(L) cells, and that is never more than L (a 2-digit count needs a
// run of at least 10). So the write index stays behind the read index and the
// overwrite can never clobber input the algorithm has not consumed yet. Watch
// the two pointers: write is always at or to the left of read.

const BUF = "chars   (in place)";

fn trace(input) {
    let buf = chars(input.chars);
    let n = buf.len();

    enter("compress", #{ chars: S(input.chars), n: N(n) });

    let write = 0;
    let read = 0;
    set(#{ write: N(0), read: N(0) });
    step("init", "Two indices into the same array: read consumes, write produces.",
        [ array(global::BUF, buf).ptr("w", 0).ptr("r", 0) ]);

    while read < n {
        step("loop", `read = ${read}, write = ${write}. write is ${write} cell(s) behind — that gap is the compression so far.`,
            [ array(global::BUF, buf).ptr("w", write).ptr("r", read).done(before(write)) ]);

        let ch = buf[read];
        set(#{ ch: S(ch) });
        step("char", `A run of ${ch} starts at ${read}.`,
            [ array(global::BUF, buf).ptr("w", write).ptr("r", read).hl([read]) ]);

        let count = 0;
        set(#{ count: N(0) });
        step("count", "Count it before writing anything — writing early would destroy the characters still to be read.",
            [ array(global::BUF, buf).ptr("r", read).hl([read]) ]);

        let start = read;
        while read < n && buf[read] == ch {
            read += 1;
            count += 1;
            set(#{ read: N(read), count: N(count) });
            step("grow", `${ch} × ${count} so far.`,
                [ array(global::BUF, buf).ptr("w", write).ptr("r", read).hl(span(start, read - 1)) ]);
        }
        step("run", `The run of ${ch} is ${count} long and ends at index ${read - 1}.`,
            [ array(global::BUF, buf).ptr("w", write).ptr("r", read).hl(span(start, read - 1)) ]);

        buf[write] = ch;
        write += 1;
        set(#{ write: N(write) });
        step("put", `Write ${ch} at index ${write - 1}.`,
            [ array(global::BUF, buf).ptr("w", write).ptr("r", read).done(before(write - 1)).hl([write - 1]) ]);

        if count > 1 {
            step("many", `${count} > 1, so the count is written out too.`,
                [ array(global::BUF, buf).ptr("w", write).ptr("r", read).done(before(write)) ]);
            let digits = chars(`${count}`);
            step("digits", `${count} is ${plural(digits.len(), "digit")} — each goes in its own cell. A count of 12 is '1','2', never the single value 12.`,
                [ text("count as characters", [join(digits, " ")]) ]);
            for d in digits {
                buf[write] = d;
                write += 1;
                set(#{ write: N(write) });
                step("digit", `Write '${d}'.`,
                    [ array(global::BUF, buf).ptr("w", write).ptr("r", read).done(before(write - 1)).hl([write - 1]) ]);
            }
        } else {
            step("many", `A run of 1 gets no number — "a1" would be longer than "a".`,
                [ array(global::BUF, buf).ptr("w", write).ptr("r", read).done(before(write)) ]);
        }
    }

    let out = "";
    for k in 0..write { out += buf[k]; }
    log(`length ${write}: ${out}`, "result");
    result(out);
    step("ret", `The first ${write} cells hold the answer: "${out}". Everything past index ${write - 1} is leftover and ignored.`,
        [ array(global::BUF, buf).done(before(write)).bad(after(write - 1, n)),
          text("compressed", [out, `returned length = ${write}`]) ],
        "return");
    leave(`${write}`);
}
