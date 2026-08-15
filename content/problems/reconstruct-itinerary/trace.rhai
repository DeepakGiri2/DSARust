// Reconstruct Itinerary — record an airport only once it is finished.
//
// Greedy fails because the smallest next airport can be a dead end with
// tickets still unused elsewhere. Hierholzer's trick is not to fight that but
// to exploit it: an airport is written down only after every ticket out of it
// has been used, so a dead end is the first entry recorded. Reversing at the
// end puts it last — which is exactly where an unavoidable dead end has to be.

fn parse_tickets(spec) {
    let out = [];
    for part in spec.split(",") {
        let w = words(part);
        if w.len() >= 2 { out.push([w[0], w[1]]); }
    }
    out
}

fn sorted_copy(arr) {
    let out = [];
    for v in arr { out.push(v); }
    for i in 1..out.len() {
        let j = i;
        while j > 0 && out[j - 1] > out[j] {
            swap_at(out, j - 1, j);
            j -= 1;
        }
    }
    out
}

fn remaining_view(adj) {
    let out = [];
    for e in adj {
        out.push(`${e[0]} -> ${if e[1].len() == 0 { "(none)" } else { join(e[1], ", ") }}`);
    }
    out
}

fn visit(adj, airport, route) {
    enter(`visit(${airport})`, #{ airport: S(airport) });

    while map_has(adj, airport) && map_get(adj, airport).len() > 0 {
        let outs = map_get(adj, airport);
        step("while", `${airport} still has ${plural(outs.len(), "unused ticket")}: ${join(outs, ", ")}.`,
            [ text("tickets still unused", remaining_view(adj)), array("route (reversed)", route) ]);

        let next = outs[0];
        outs.remove(0);
        map_set(adj, airport, outs);
        step("take", `Take the smallest, ${next} — and remove the ticket so it can never be used twice.`,
            [ text("tickets still unused", remaining_view(adj)) ]);

        step("recurse", `Fly to ${next}.`,
            [ text("tickets still unused", remaining_view(adj)) ], "call");
        let out = visit(adj, next, route);
        adj = out[0];
        route = out[1];
    }

    route.push(airport);
    log(`finish ${airport}`);
    step("append", `${airport} has no tickets left, so it is finished. Record it now — being recorded early means being placed late.`,
        [ array("route (reversed)", route), text("tickets still unused", remaining_view(adj)) ],
        "return");
    leave(airport);
    [adj, route]
}

fn trace(input) {
    let tickets = parse_tickets(input.tickets);

    enter("findItinerary", #{ tickets: N(tickets.len()) });

    let adj = map_new();
    for t in tickets {
        let outs = if map_has(adj, t[0]) { map_get(adj, t[0]) } else { [] };
        outs.push(t[1]);
        map_set(adj, t[0], outs);
    }
    step("build", "Group the tickets by departure airport.",
        [ text("tickets still unused", remaining_view(adj)) ]);

    for e in adj { map_set(adj, e[0], sorted_copy(e[1])); }
    step("sort", "Sort each list so that 'take the first' means 'take the lexicographically smallest'.",
        [ text("tickets still unused", remaining_view(adj)) ]);

    let route = [];
    step("route", "The route is built backwards: finished airports first.",
        [ array("route (reversed)", route) ]);

    step("start", "Begin at JFK.",
        [ text("tickets still unused", remaining_view(adj)) ], "call");
    let out = visit(adj, "JFK", route);
    route = out[1];

    let rev = [];
    let k = route.len() - 1;
    while k >= 0 { rev.push(route[k]); k -= 1; }
    step("reverse", `Recorded ${join(route, ", ")} — dead ends first. Reverse it.`,
        [ array("route (reversed)", route), array("itinerary", rev) ]);

    let answer = join(rev, " ");
    log(`answer ${answer}`, "result");
    result(answer);
    step("ret", `Every ticket used exactly once: ${answer}.`,
        [ array("itinerary", rev) ], "return");
    leave(answer);
}
