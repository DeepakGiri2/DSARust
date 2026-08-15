fn hpush_min(heap, key, label) {
    heap.push([key, label]);
    let i = heap.len() - 1;
    while i > 0 {
        let p = (i - 1) / 2;
        if heap[p][0] <= heap[i][0] { break; }
        swap_at(heap, p, i);
        i = p;
    }
    heap
}

fn hpop_min(heap) {
    let n = heap.len();
    heap[0] = heap[n - 1];
    heap.truncate(n - 1);
    let i = 0;
    loop {
        let l = 2 * i + 1;
        let r = 2 * i + 2;
        let small = i;
        if l < heap.len() && heap[l][0] < heap[small][0] { small = l; }
        if r < heap.len() && heap[r][0] < heap[small][0] { small = r; }
        if small == i { break; }
        swap_at(heap, i, small);
        i = small;
    }
    heap
}

fn hpush_max(heap, key, label) {
    heap.push([key, label]);
    let i = heap.len() - 1;
    while i > 0 {
        let p = (i - 1) / 2;
        if heap[p][0] >= heap[i][0] { break; }
        swap_at(heap, p, i);
        i = p;
    }
    heap
}

fn hpop_max(heap) {
    let n = heap.len();
    heap[0] = heap[n - 1];
    heap.truncate(n - 1);
    let i = 0;
    loop {
        let l = 2 * i + 1;
        let r = 2 * i + 2;
        let big = i;
        if l < heap.len() && heap[l][0] > heap[big][0] { big = l; }
        if r < heap.len() && heap[r][0] > heap[big][0] { big = r; }
        if big == i { break; }
        swap_at(heap, i, big);
        i = big;
    }
    heap
}

fn shown(heap) {
    let out = [];
    for e in heap { out.push(e[1]); }
    out
}

// Design Twitter — a heap merges sorted lists without merging them.
//
// Each user's tweets are already in time order, so a feed is a k-way merge of
// sorted lists. A heap does that lazily: it only ever holds a few candidates
// at a time, and each pop yields the globally newest remaining tweet. Nothing
// is sorted, and no user's full history is ever scanned — which is why the
// cost depends on how many people you follow, not on how much they have
// posted.

fn feed_view(tweets) {
    let out = [];
    for e in tweets {
        let labels = [];
        for t in e[1] { labels.push(`t${t[1]}@${t[0]}`); }
        out.push(`user ${e[0]}: ${if labels.len() == 0 { "(nothing)" } else { join(labels, ", ") }}`);
    }
    if out.len() == 0 { out.push("(no tweets yet)"); }
    out
}

fn follow_view(follows) {
    let out = [];
    for e in follows {
        out.push(`${e[0]} follows ${if e[1].len() == 0 { "(nobody)" } else { join(e[1], ", ") }}`);
    }
    if out.len() == 0 { out.push("(nobody follows anybody)"); }
    out
}

fn trace(input) {
    let clock = 0;
    let tweets = map_new();
    let follows = map_new();
    let feeds = [];

    enter("Twitter", #{});
    set(#{ clock: N(clock) });
    step("init", "One global counter. It is what makes tweets from different users comparable.",
        [ text("timeline", [`clock = ${clock}`]) ]);
    step("tweets", "Each user's tweets, in posting order.", [ text("tweets", feed_view(tweets)) ]);
    step("follows", "And who follows whom.", [ text("follows", follow_view(follows)) ]);

    for raw in input.ops.split(",") {
        let parts = words(raw);
        if parts.len() == 0 { continue; }

        if parts[0] == "post" {
            let user = num(parts[1]);
            let tweet = num(parts[2]);
            enter(`postTweet(${user}, ${tweet})`, #{ user: N(user), tweet: N(tweet) });
            clock += 1;
            set(#{ clock: N(clock) });
            step("clock", `Stamp it: time ${clock}.`, [ text("timeline", [`clock = ${clock}`]) ]);
            let list = if map_has(tweets, user) { map_get(tweets, user) } else { [] };
            list.push([clock, tweet]);
            map_set(tweets, user, list);
            log(`user ${user} posts ${tweet} at ${clock}`);
            step("post", `Appended to user ${user}'s list, which therefore stays sorted by time for free.`,
                [ text("tweets", feed_view(tweets)) ], "return");
            leave("");
            continue;
        }

        if parts[0] == "follow" {
            let a = num(parts[1]);
            let b = num(parts[2]);
            enter(`follow(${a}, ${b})`, #{});
            let set_a = if map_has(follows, a) { map_get(follows, a) } else { [] };
            let has = false;
            for x in set_a { if x == b { has = true; } }
            if !has { set_a.push(b); }
            map_set(follows, a, set_a);
            log(`${a} follows ${b}`);
            step("follow", `User ${a} now follows ${b}.`, [ text("follows", follow_view(follows)) ], "return");
            leave("");
            continue;
        }

        if parts[0] == "unfollow" {
            let a = num(parts[1]);
            let b = num(parts[2]);
            enter(`unfollow(${a}, ${b})`, #{});
            let set_a = if map_has(follows, a) { map_get(follows, a) } else { [] };
            let kept = [];
            for x in set_a { if x != b { kept.push(x); } }
            map_set(follows, a, kept);
            log(`${a} unfollows ${b}`);
            step("unfollow", `User ${a} no longer follows ${b}.`,
                [ text("follows", follow_view(follows)) ], "return");
            leave("");
            continue;
        }

        let user = num(parts[1]);
        enter(`getNewsFeed(${user})`, #{ user: N(user) });

        let h = [];
        step("heapInit", "A max-heap keyed by timestamp — its top is always the newest tweet among the candidates.",
            [ heap("candidates (newest on top)", shown(h)) ]);

        let sources = [user];
        step("self", `Your own tweets count too, so user ${user} is the first source.`,
            [ text("sources", [join(sources, ", ")]) ]);

        let followed = if map_has(follows, user) { map_get(follows, user) } else { [] };
        for f in followed { sources.push(f); }
        step("followed", `Plus everyone followed: ${join(sources, ", ")}.`,
            [ text("sources", [join(sources, ", ")]), text("follows", follow_view(follows)) ]);

        for s in sources {
            let list = if map_has(tweets, s) { map_get(tweets, s) } else { [] };
            step("collect", `User ${s} has ${plural(list.len(), "tweet")}.`,
                [ text("tweets", feed_view(tweets)) ]);

            let i = list.len() - 1;
            let taken = 0;
            while i >= 0 && taken < 10 {
                step("recent", `Their tweet ${list[i][1]}, posted at time ${list[i][0]}.`,
                    [ heap("candidates (newest on top)", shown(h)) ]);
                h = hpush_max(h, list[i][0], `t${list[i][1]}@${list[i][0]}`);
                step("push", "Into the heap. Only the ten newest per user can matter, so older ones are never even looked at.",
                    [ heap("candidates (newest on top)", shown(h)) ]);
                i -= 1;
                taken += 1;
            }
        }

        let out = [];
        step("out", "Now take the newest ten overall.",
            [ heap("candidates (newest on top)", shown(h)), text("feed", out) ]);

        while out.len() < 10 && h.len() > 0 {
            step("drain", `${plural(h.len(), "candidate")} left.`,
                [ heap("candidates (newest on top)", shown(h)) ]);
            let top = h[0];
            h = hpop_max(h);
            step("pop", `The heap's top is ${top[1]} — the newest of everything still in play.`,
                [ heap("candidates (newest on top)", shown(h)) ]);
            let bits = top[1].split("@");
            out.push(bits[0].sub_string(1, bits[0].len() - 1));
            step("emit", `Feed so far: ${join(out, ", ")}.`, [ text("feed", out) ]);
        }

        feeds.push(join(out, " "));
        log(`feed for ${user}: ${join(out, " ")}`, "result");
        step("ret", `${plural(out.len(), "tweet")} returned.`, [ text("feed", out) ], "return");
        leave(join(out, " "));
    }

    let answer = join(feeds, " | ");
    result(answer);
    leave(answer);
}
