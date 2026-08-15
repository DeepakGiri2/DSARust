// Corporate Flight Bookings — the difference array.
//
// Adding seats to every flight in a range is the obvious move and the slow
// one. Instead mark only the edges: +seats where the range starts, −seats just
// past where it ends. Nothing in between is touched. The running sum at the
// end "replays" those edges, and each flight ends up holding exactly the
// bookings that were open when the sweep passed it.

fn parse_bookings(spec) {
    let out = [];
    for part in spec.split(",") {
        let w = words(part);
        if w.len() >= 3 { out.push([num(w[0]), num(w[1]), num(w[2])]); }
    }
    out
}

fn flight_labels(n) {
    let out = [];
    for i in 0..n { out.push(`flight ${i + 1}`); }
    out
}

fn trace(input) {
    let bookings = parse_bookings(input.bookings);
    let n = input.n;

    enter("corpFlightBookings", #{ bookings: N(bookings.len()), n: N(n) });

    let diff = [];
    for i in 0..(n + 1) { diff.push(0); }
    set(#{ diff: A(diff) });
    step("init",
        `A difference array with ${n + 1} slots — one per flight plus a spare, so a booking that ends on the last flight still has somewhere to write its closing mark.`,
        [ array("diff — change at this flight", diff) ]);

    for b in 0..bookings.len() {
        let first = bookings[b][0];
        let last = bookings[b][1];
        let seats = bookings[b][2];
        set(#{ first: N(first), last: N(last), seats: N(seats) });
        step("loop", `Booking ${b}: ${seats} seats on flights ${first} … ${last}. Two writes, not ${last - first + 1}.`,
            [ array("diff — change at this flight", diff).window(first - 1, last - 1) ]);

        diff[first - 1] += seats;
        set(#{ diff: A(diff) });
        step("open", `+${seats} at index ${first - 1}: from flight ${first} on, ${seats} more seats are in play.`,
            [ array("diff — change at this flight", diff).hl([first - 1]) ]);

        diff[last] -= seats;
        set(#{ diff: A(diff) });
        step("close", `−${seats} at index ${last}: the booking stops applying after flight ${last}.`,
            [ array("diff — change at this flight", diff).hl([last]) ]);
    }

    let ans = [];
    for i in 0..n { ans.push(0); }
    unset("first"); unset("last"); unset("seats");
    step("alloc", `All ${bookings.len()} bookings are recorded as ${bookings.len() * 2} marks. Now sweep left to right.`,
        [ array("diff — change at this flight", diff), array("seats per flight", ans) ]);

    let run = 0;
    set(#{ run: N(0) });
    step("zero", "run = 0 — the number of seats currently booked as the sweep moves.",
        [ array("diff — change at this flight", diff), array("seats per flight", ans) ]);

    for i in 0..n {
        step("sweep", `Flight ${i + 1} (index ${i}).`,
            [ array("diff — change at this flight", diff).ptr("i", i).done(before(i)),
              array("seats per flight", ans).done(before(i)) ]);

        run += diff[i];
        set(#{ run: N(run) });
        step("run", `Apply the change here: run = ${run - diff[i]} + (${diff[i]}) = ${run}.`,
            [ array("diff — change at this flight", diff).ptr("i", i).hl([i]).done(before(i)),
              array("seats per flight", ans).done(before(i)) ]);

        ans[i] = run;
        set(#{ ans: A(ans) });
        step("fill", `Flight ${i + 1} carries ${run} seats.`,
            [ array("diff — change at this flight", diff).ptr("i", i).done(span(0, i)),
              array("seats per flight", ans).hl([i]).done(before(i)) ]);
    }

    let out = join(ans, " ");
    log(`seats: ${out}`, "result");
    result(out);
    step("ret", `Every flight totalled in one pass: ${brackets(ans)}.`,
        [ array("seats per flight", ans).done(seq(0, n - 1)) ], "return");
    leave(brackets(ans));
}
