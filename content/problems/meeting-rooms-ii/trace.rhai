fn parse_pairs(spec) {
    let out = [];
    for part in spec.split(",") {
        let w = words(part);
        if w.len() >= 2 { out.push([num(w[0]), num(w[1])]); }
    }
    out
}

fn labels(ivs) {
    let out = [];
    for iv in ivs { out.push(`[${iv[0]},${iv[1]}]`); }
    out
}

fn sort_by_start(ivs) {
    for i in 1..ivs.len() {
        let j = i;
        while j > 0 && ivs[j - 1][0] > ivs[j][0] {
            swap_at(ivs, j - 1, j);
            j -= 1;
        }
    }
    ivs
}

// Meeting Rooms II — take the intervals apart.
//
// Which meeting occupies which room is never asked, only how many are running
// at once. So the pairing between a start and its own end is irrelevant, and
// the two lists can be sorted independently and swept together. Each start
// before the next end opens a room, each remaining end closes one, and the
// peak count is the answer.

fn sort_nums(arr) {
    for i in 1..arr.len() {
        let j = i;
        while j > 0 && arr[j - 1] > arr[j] {
            swap_at(arr, j - 1, j);
            j -= 1;
        }
    }
    arr
}

fn trace(input) {
    let ivs = parse_pairs(input.intervals);
    let n = ivs.len();

    enter("minMeetingRooms", #{ n: N(n) });

    let starts = [];
    let ends = [];
    for iv in ivs { starts.push(iv[0]); ends.push(iv[1]); }
    starts = sort_nums(starts);
    ends = sort_nums(ends);
    step("split", "Starts and ends, each sorted on its own. Which end belongs to which start no longer matters — only how many rooms are in use.",
        [ array("meetings", labels(ivs)),
          array("start times (sorted)", starts), array("end times (sorted)", ends) ]);

    let rooms = 0;
    let best = 0;
    set(#{ rooms: N(rooms), best: N(best) });
    step("init", "Rooms currently in use, and the most ever used at once.",
        [ text("state", [`rooms in use: ${rooms}`, `peak: ${best}`]) ]);

    let i = 0;
    let j = 0;
    set(#{ i: N(i), j: N(j) });
    step("cursors", "Sweep both lists together, always taking whichever event happens first.",
        [ array("start times (sorted)", starts).ptr("i", i),
          array("end times (sorted)", ends).ptr("j", j) ]);

    while i < starts.len() {
        step("loop", `${starts.len() - i} meeting(s) still to start.`,
            [ array("start times (sorted)", starts).ptr("i", i).done(before(i)),
              array("end times (sorted)", ends).ptr("j", j).done(before(j)),
              text("state", [`rooms in use: ${rooms}`, `peak: ${best}`]) ]);

        if starts[i] < ends[j] {
            step("overlap", `The next start, ${starts[i]}, comes before the next end, ${ends[j]} — so a meeting begins while another is still running.`,
                [ array("start times (sorted)", starts).ptr("i", i).hl([i]),
                  array("end times (sorted)", ends).ptr("j", j).hl([j]) ]);
            rooms += 1;
            i += 1;
            set(#{ rooms: N(rooms), i: N(i) });
            log(`open a room; ${rooms} in use`);
            step("open", `Open a room: ${rooms} in use.`,
                [ text("state", [`rooms in use: ${rooms}`]) ]);

            if rooms > best {
                step("peak", `${rooms} is more than the previous peak of ${best}.`,
                    [ text("state", [`rooms in use: ${rooms}`, `peak: ${best}`]) ]);
                best = rooms;
                set(#{ best: N(best) });
                step("record", `New peak: ${best}.`, [ text("state", [`peak: ${best}`]) ]);
            } else {
                step("peak", `${rooms} does not beat the peak of ${best}.`,
                    [ text("state", [`rooms in use: ${rooms}`, `peak: ${best}`]) ]);
                step("record", "Peak unchanged.", [ text("state", [`peak: ${best}`]) ]);
            }
        } else {
            step("free", `The next end, ${ends[j]}, comes first — a meeting finishes before the next one starts.`,
                [ array("start times (sorted)", starts).ptr("i", i).hl([i]),
                  array("end times (sorted)", ends).ptr("j", j).hl([j]) ]);
            rooms -= 1;
            j += 1;
            set(#{ rooms: N(rooms), j: N(j) });
            log(`free a room; ${rooms} in use`);
            step("close", `Free a room: ${rooms} in use.`,
                [ text("state", [`rooms in use: ${rooms}`]) ]);
        }
    }

    log(`answer ${best}`, "result");
    result(`${best}`);
    step("ret", `At its busiest, ${plural(best, "meeting")} ran at once, so ${plural(best, "room")} suffice.`,
        [ array("meetings", labels(ivs)), text("state", [`peak: ${best}`]) ], "return");
    leave(`${best}`);
}
