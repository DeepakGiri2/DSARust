fn parse_triples(spec) {
    let out = [];
    for part in spec.split(",") {
        let w = words(part);
        if w.len() >= 3 { out.push([num(w[0]), num(w[1]), num(w[2])]); }
    }
    out
}

// Cheapest Flights Within K Stops — Bellman-Ford, because rounds count hops.
//
// Dijkstra settles a city permanently at its cheapest price, which is exactly
// wrong here: that price might need too many flights, and the legal answer
// might be dearer. Bellman-Ford does not settle anything — it just relaxes
// every edge once per round, so after r rounds it knows the best price
// reachable in at most r flights. Bounding the rounds bounds the stops.
//
// The copied table matters: relaxing into the *live* table would let a price
// found this round be used again in the same round, quietly allowing an extra
// flight.

fn node_labels(n) {
    let out = [];
    for i in 0..n { out.push(`${i}`); }
    out
}

fn edge_list(flights) {
    let out = [];
    for f in flights { out.push([f[0], f[1], true, f[2]]); }
    out
}

fn price_view(dist) {
    let out = [];
    for v in dist { out.push(if v >= 1000000 { "∞" } else { `${v}` }); }
    out
}

fn reached(dist) {
    let out = [];
    for i in 0..dist.len() { if dist[i] < 1000000 { out.push(i); } }
    out
}

fn trace(input) {
    let n = input.n;
    let flights = parse_triples(input.flights);
    let src = input.src;
    let dst = input.dst;
    let k = input.k;
    let nodes = node_labels(n);
    let edges = edge_list(flights);

    enter("findCheapestPrice", #{ src: N(src), dst: N(dst), k: N(k) });

    let dist = [];
    for i in 0..n { dist.push(1000000); }
    step("init", "Cheapest price known for each city.",
        [ graph("flights", nodes, edges),
          array("cheapest price to each city", price_view(dist)) ]);

    dist[src] = 0;
    step("seed", `Being at city ${src} costs nothing.`,
        [ graph("flights", nodes, edges).cur(src),
          array("cheapest price to each city", price_view(dist)).hl([src]) ]);

    for round in 0..(k + 1) {
        step("rounds", `Round ${round + 1} of ${k + 1}. After this round, every price reachable in ${round + 1} flight(s) will be known.`,
            [ graph("flights", nodes, edges).done(reached(dist)),
              array("cheapest price to each city", price_view(dist)) ]);

        let next = [];
        for v in dist { next.push(v); }
        step("copy", "Work into a copy. Relaxing into the live table would let this round's discoveries be extended again in the same round — one extra flight, smuggled in.",
            [ array("cheapest price to each city", price_view(dist)),
              array("this round's table", price_view(next)) ]);

        for f in flights {
            step("edges", `Flight ${f[0]} → ${f[1]} for ${f[2]}.`,
                [ graph("flights", nodes, edges).active([[f[0], f[1]]]).done(reached(dist)) ]);

            if dist[f[0]] >= 1000000 {
                step("unreached", `City ${f[0]} is not reachable within ${round} flight(s), so this route cannot be taken yet.`,
                    [ array("cheapest price to each city", price_view(dist)).bad([f[0]]) ]);
                continue;
            }

            if dist[f[0]] + f[2] < next[f[1]] {
                step("cheaper", `${dist[f[0]]} + ${f[2]} = ${dist[f[0]] + f[2]}, better than ${if next[f[1]] >= 1000000 { "unreachable" } else { `${next[f[1]]}` }}.`,
                    [ array("this round's table", price_view(next)).ptr("to", f[1]) ]);
                next[f[1]] = dist[f[0]] + f[2];
                log(`city ${f[1]} for ${next[f[1]]} in <= ${round + 1} flight(s)`);
                step("update", `City ${f[1]} now costs ${next[f[1]]}.`,
                    [ array("this round's table", price_view(next)).hl([f[1]]) ]);
            } else {
                step("cheaper", `${dist[f[0]]} + ${f[2]} = ${dist[f[0]] + f[2]} is no better than ${if next[f[1]] >= 1000000 { "unreachable, but still not an improvement within this round" } else { `${next[f[1]]}` }}.`,
                    [ array("this round's table", price_view(next)).ptr("to", f[1]) ]);
            }
        }

        dist = next;
        step("commit", `End of round ${round + 1}.`,
            [ graph("flights", nodes, edges).done(reached(dist)),
              array("cheapest price to each city", price_view(dist)) ]);
    }

    step("check", `After ${k + 1} round(s), city ${dst} costs ${if dist[dst] >= 1000000 { "nothing known" } else { `${dist[dst]}` }}.`,
        [ array("cheapest price to each city", price_view(dist)).ptr("dst", dst) ]);

    if dist[dst] >= 1000000 {
        log("unreachable within the stop limit", "result");
        step("fail", `City ${dst} cannot be reached with at most ${plural(k, "stop")}.`,
            [ graph("flights", nodes, edges).bad([dst]) ], "return");
        result("-1");
        leave("-1");
        return;
    }

    log(`answer ${dist[dst]}`, "result");
    result(`${dist[dst]}`);
    step("ret", `The cheapest legal route costs ${dist[dst]}.`,
        [ graph("flights", nodes, edges).done(reached(dist)),
          array("cheapest price to each city", price_view(dist)).hl([dst]) ], "return");
    leave(`${dist[dst]}`);
}
