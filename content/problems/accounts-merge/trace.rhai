// Accounts Merge — union the emails, never the accounts.
//
// The name is a red herring: two Johns are two people until an email proves
// otherwise. So the entities being merged are emails, and an account is just a
// claim that all of its emails belong together. Union each email with the
// account's first email and transitivity does the rest — two accounts that
// share any single email end up under one root without ever being compared.

const PARENT = "parent of each email id";

fn find_root(parent, x) {
    enter(`find(${x})`, #{ x: N(x) });
    let cur = x;
    while parent[cur] != cur {
        step("walk", `${cur} is not its own root — its parent is ${parent[cur]}.`,
            [ array(global::PARENT, parent).ptr("x", cur).hl([cur]) ]);
        parent[cur] = parent[parent[cur]];
        step("compress", `Point ${cur} straight at ${parent[cur]}, halving the path for every later find.`,
            [ array(global::PARENT, parent).ptr("x", cur).hl([cur]) ]);
        cur = parent[cur];
        step("up", `Climb to ${cur}.`, [ array(global::PARENT, parent).ptr("x", cur) ]);
    }
    step("root", `${cur} is its own parent — the root, and therefore the name of its whole component.`,
        [ array(global::PARENT, parent).ptr("root", cur).hl([cur]) ], "return");
    leave(`${cur}`);
    [parent, cur]
}

fn parse_accounts(spec) {
    let out = [];
    for part in spec.split(",") {
        let bits = part.split(":");
        if bits.len() < 2 { continue; }
        let name = bits[0];
        name.trim();
        let acc = [name];
        for e in words(bits[1]) { acc.push(e); }
        if acc.len() > 1 { out.push(acc); }
    }
    out
}

fn sorted(items) {
    let out = [];
    for x in items {
        let i = 0;
        while i < out.len() && out[i] < x { i += 1; }
        out.insert(i, x);
    }
    out
}

fn id_map(ids) {
    let m = map_new();
    for pair in ids { map_set(m, pair[0], pair[1]); }
    m
}

fn id_of(ids, email) {
    for pair in ids {
        if pair[0] == email { return pair[1]; }
    }
    -1
}

fn index_of(items, want) {
    for i in 0..items.len() {
        if items[i] == want { return i; }
    }
    -1
}

fn bucket_view(roots, lists) {
    let m = map_new();
    for i in 0..roots.len() { map_set(m, roots[i], join(lists[i], ", ")); }
    m
}

const IDS = "email → id";
const BUCKETS = "component → its emails";

fn trace(input) {
    let accounts = parse_accounts(input.accounts);

    enter("accountsMerge", #{ accounts: N(accounts.len()) });

    let ids = [];        // [email, id] in first-seen order
    let owner = [];      // owner[id] = name
    let parent = [];
    step("init", `${accounts.len()} accounts. Emails get integer ids as they appear, so union-find can stay a plain array.`,
        [ text("accounts", input.accounts.split(",")) ]);

    for k in 0..accounts.len() {
        let acc = accounts[k];
        let name = acc[0];
        set(#{ account: S(name), emails: N(acc.len() - 1) });
        step("scan", `Account ${k}: ${name} with ${plural(acc.len() - 1, "email")}.`,
            [ kv(global::IDS, id_map(ids)), array(global::PARENT, parent) ]);

        for j in 1..acc.len() {
            let e = acc[j];
            step("each", `Email ${e}.`,
                [ kv(global::IDS, id_map(ids)).hl([e]), array(global::PARENT, parent) ]);

            if id_of(ids, e) < 0 {
                let fresh = parent.len();
                ids.push([e, fresh]);
                parent.push(fresh);
                owner.push(name);
                step("new", `${e} is new — it becomes id ${fresh}, its own component, owned (for now) by ${name}.`,
                    [ kv(global::IDS, id_map(ids)).hl([e]),
                      array(global::PARENT, parent).hl([fresh]) ]);
            } else {
                step("new", `${e} already has id ${id_of(ids, e)} — seen in an earlier account. That is the link between them.`,
                    [ kv(global::IDS, id_map(ids)).hl([e]),
                      array(global::PARENT, parent).hl([id_of(ids, e)]) ]);
            }

            let anchor = id_of(ids, acc[1]);
            let mine = id_of(ids, e);
            step("roots", `Union ${e} with this account's first email ${acc[1]} (ids ${mine} and ${anchor}).`,
                [ array(global::PARENT, parent) ], "call");
            let ra = find_root(parent, anchor);
            parent = ra[0];
            let rb = find_root(parent, mine);
            parent = rb[0];

            parent[ra[1]] = rb[1];
            step("union", `parent[${ra[1]}] = ${rb[1]}. If ${e} was already in another account's component, the two accounts are now one.`,
                [ array(global::PARENT, parent).hl([ra[1], rb[1]]),
                  kv(global::IDS, id_map(ids)) ]);
        }
    }

    unset("account"); unset("emails");
    step("group", "Every account is recorded. Now read the structure back: bucket the emails by root.",
        [ kv(global::IDS, id_map(ids)), array(global::PARENT, parent) ]);

    let roots = [];      // component root, in first-seen order
    let lists = [];      // lists[i] = the emails in component roots[i]
    for pair in ids {
        let e = pair[0];
        let ra = find_root(parent, pair[1]);
        parent = ra[0];
        let r = ra[1];
        step("collect", `${e} (id ${pair[1]}) belongs to component ${r}.`,
            [ kv(global::IDS, id_map(ids)).hl([e]), array(global::PARENT, parent).hl([r]) ]);
        let slot = index_of(roots, r);
        if slot < 0 {
            roots.push(r);
            lists.push([e]);
        } else {
            lists[slot].push(e);
        }
        step("bucket", `Component ${r} now holds ${brackets(lists[index_of(roots, r)])}.`,
            [ kv(global::BUCKETS, bucket_view(roots, lists)).hl([r]) ]);
    }

    let rows = [];
    for i in 0..roots.len() {
        let r = roots[i];
        let emails = lists[i];
        step("build", `Component ${r}: ${plural(emails.len(), "email")}, owner ${owner[r]}.`,
            [ kv(global::BUCKETS, bucket_view(roots, lists)).hl([r]) ]);
        emails = sorted(emails);
        step("sort", `Sorted: ${brackets(emails)}. The problem asks for sorted emails, and this is the only part of the algorithm that is not linear.`,
            [ text("this person", [`${owner[r]}`, join(emails, ", ")]) ]);
        rows.push(`${owner[r]}:${join(emails, ",")}`);
        step("row", `Row built: ${rows[rows.len() - 1]}.`,
            [ text("merged so far", rows) ]);
    }

    rows = sorted(rows);
    let out = join(rows, " | ");
    log(out, "result");
    result(out);
    step("ret", `${plural(rows.len(), "person")} after merging ${plural(accounts.len(), "account")}.`,
        [ text("merged so far", rows) ], "return");
    leave(out);
}
