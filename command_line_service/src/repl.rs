use std::io::{self, BufRead, Write};

use fruit_domain::{
    burner::compute_burn,
    community::{Community, CommunityId},
    community_store::CommunityStore,
    event_log::{EventPayload, HasSequenceId, StateMutation},
    event_log_store::EventLogStore,
    fruit::FRUITS,
    gifter::compute_gift,
    granter::Granter,
    member::Member,
    random_granter::RandomGranter,
};
use fruit_in_memory_db::{
    community_repo::InMemoryCommunityRepo, event_log_repo::InMemoryEventLogRepo,
};

/// Runs the interactive REPL for the fruit game.
///
/// Creates a single in-memory community and enters a read-eval-print loop.
/// Type `help` at the prompt for a list of available commands.
pub fn run() {
    let event_log_repo = InMemoryEventLogRepo::new();
    let event_log = EventLogStore::new(&event_log_repo);
    let store = CommunityStore::new(InMemoryCommunityRepo::new(), &event_log_repo);
    let community_id = store.init().unwrap().id;
    let mut granter = RandomGranter::new(rand::thread_rng());
    let mut show_help = false;
    let mut show_log_lines: Option<usize> = None;

    let stdin = io::stdin();
    loop {
        print!("\x1b[2J\x1b[H");

        print_community(&fetch(&store, community_id));
        println!();
        if show_help {
            print_help();
            println!();
        } else {
            println!("type 'help' for commands.");
        }

        if let Some(log_lines) = show_log_lines {
            cmd_log(&event_log, community_id, log_lines)
        }

        print!("> ");
        io::stdout().flush().unwrap();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap() == 0 {
            break;
        }

        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }

        show_help = tokens[0] == "help";
        if tokens[0] != "log" {
            show_log_lines = None
        }
        match tokens[0] {
            "add" => cmd_add(&event_log, community_id, &tokens[1..]),
            "remove" => cmd_remove(&store, &event_log, community_id, &tokens[1..]),
            "grant" => cmd_grant(&store, &event_log, community_id, &mut granter, &tokens[1..]),
            "gift" => cmd_gift(&store, &event_log, community_id, &tokens[1..]),
            "burn" => cmd_burn(&store, &event_log, community_id, &tokens[1..]),
            "luck" => cmd_luck(&store, &event_log, community_id, &tokens[1..]),
            "log" => {
                match &tokens[1..].first().and_then(|s| s.parse::<usize>().ok()) {
                    Some(n) => show_log_lines = Some(*n),
                    None => {
                        println!("usage: grant <count>");
                    }
                };
            }
            "help" => {}
            "quit" | "exit" => break,
            cmd => println!("unknown command '{cmd}'. Type 'help' for commands."),
        }
    }
}

type Store<'a> = CommunityStore<InMemoryCommunityRepo, &'a InMemoryEventLogRepo>;

fn cmd_add(event_log: &EventLogStore<&InMemoryEventLogRepo>, id: CommunityId, args: &[&str]) {
    if args.is_empty() {
        println!("usage: add <name>");
        return;
    }
    let name = args.join(" ");
    let member = Member::new(&name);
    let event = event_log
        .append_event(
            id,
            EventPayload::AddMember {
                display_name: name.clone(),
            },
        )
        .unwrap();
    let mutations = vec![StateMutation::AddMember { member }];
    event_log.append_effect(event.id, id, mutations).unwrap();
}

fn cmd_remove(
    store: &Store<'_>,
    event_log: &EventLogStore<&InMemoryEventLogRepo>,
    id: CommunityId,
    args: &[&str],
) {
    if args.is_empty() {
        println!("usage: remove <name>");
        return;
    }
    let name = args.join(" ");
    let community = fetch(store, id);
    match community
        .members
        .values()
        .find(|m| m.display_name == name)
        .map(|m| m.id)
    {
        Some(member_id) => {
            let event = event_log
                .append_event(id, EventPayload::RemoveMember { member_id })
                .unwrap();
            let mutations = vec![StateMutation::RemoveMember { member_id }];
            event_log.append_effect(event.id, id, mutations).unwrap();
            println!("removed {name}");
        }
        None => println!("no member named '{name}'"),
    }
}

fn cmd_grant(
    store: &Store<'_>,
    event_log: &EventLogStore<&InMemoryEventLogRepo>,
    id: CommunityId,
    granter: &mut RandomGranter<rand::rngs::ThreadRng>,
    args: &[&str],
) {
    let count = match args.first().and_then(|s| s.parse::<usize>().ok()) {
        Some(n) => n,
        None => {
            println!("usage: grant <count>");
            return;
        }
    };
    let event = event_log
        .append_event(id, EventPayload::Grant { count })
        .unwrap();
    let mut community = fetch(store, id);
    let mutations = granter.grant(&community, count);
    let effect = event_log.append_effect(event.id, id, mutations).unwrap();
    community.apply_effects([effect]);
}

fn cmd_gift(
    store: &Store<'_>,
    event_log: &EventLogStore<&InMemoryEventLogRepo>,
    id: CommunityId,
    args: &[&str],
) {
    // usage: gift <sender> <emoji> <recipient>
    if args.len() < 3 {
        println!("usage: gift <sender> <emoji> <recipient>");
        return;
    }
    let sender_name = args[0];
    let emoji = args[1];
    let recipient_name = args[2..].join(" ");

    let fruit = match FRUITS.iter().find(|f| f.emoji == emoji) {
        Some(f) => *f,
        None => {
            println!("unknown fruit emoji '{emoji}'");
            return;
        }
    };

    let community = fetch(store, id);

    let sender_id = match community
        .members
        .values()
        .find(|m| m.display_name == sender_name)
        .map(|m| m.id)
    {
        Some(mid) => mid,
        None => {
            println!("no member named '{sender_name}'");
            return;
        }
    };
    let recipient_id = match community
        .members
        .values()
        .find(|m| m.display_name == recipient_name)
        .map(|m| m.id)
    {
        Some(mid) => mid,
        None => {
            println!("no member named '{recipient_name}'");
            return;
        }
    };

    let event = event_log
        .append_event(
            id,
            EventPayload::Gift {
                sender_id,
                recipient_id,
                fruit,
            },
        )
        .unwrap();
    let mutations = compute_gift(&community, sender_id, recipient_id, fruit);
    if mutations.is_empty() {
        println!("{sender_name} does not hold {emoji}");
    }
    event_log.append_effect(event.id, id, mutations).unwrap();
}

fn cmd_burn(
    store: &Store<'_>,
    event_log: &EventLogStore<&InMemoryEventLogRepo>,
    id: CommunityId,
    args: &[&str],
) {
    // usage: burn <name> <emoji> [<emoji>...]
    if args.len() < 2 {
        println!("usage: burn <name> <emoji> [<emoji>...]");
        return;
    }
    let name = args[0];
    let emoji_args = &args[1..];

    let mut fruits = Vec::new();
    for emoji in emoji_args {
        match FRUITS.iter().find(|f| f.emoji == *emoji) {
            Some(f) => fruits.push(*f),
            None => {
                println!("unknown fruit emoji '{emoji}'");
                return;
            }
        }
    }

    let community = fetch(store, id);

    let member_id = match community
        .members
        .values()
        .find(|m| m.display_name == name)
        .map(|m| m.id)
    {
        Some(mid) => mid,
        None => {
            println!("no member named '{name}'");
            return;
        }
    };

    let event = event_log
        .append_event(
            id,
            EventPayload::Burn {
                member_id,
                fruits: fruits.clone(),
            },
        )
        .unwrap();
    let mutations = compute_burn(&community, member_id, &fruits);
    if mutations.is_empty() {
        println!("{name} holds none of the requested fruits");
    }
    event_log.append_effect(event.id, id, mutations).unwrap();
}

fn cmd_luck(
    store: &Store<'_>,
    event_log: &EventLogStore<&InMemoryEventLogRepo>,
    id: CommunityId,
    args: &[&str],
) {
    if args.is_empty() {
        println!("usage: luck <value>  |  luck <name> <value>");
        return;
    }

    let value_str = args.last().unwrap();
    let luck_f64: f64 = match value_str.parse() {
        Ok(v) if (0.0..=1.0).contains(&v) => v,
        Ok(_) => {
            println!("luck must be in [0.0, 1.0]");
            return;
        }
        Err(_) => {
            println!("'{value_str}' is not a valid luck value");
            return;
        }
    };
    let luck = (luck_f64 * u8::MAX as f64).round() as u8;

    let community = fetch(store, id);

    if args.len() == 1 {
        let event = event_log
            .append_event(id, EventPayload::SetCommunityLuck { luck })
            .unwrap();
        let mutations = vec![StateMutation::SetCommunityLuck { luck }];
        let effect = event_log.append_effect(event.id, id, mutations).unwrap();
        let mut community = community;
        community.apply_effects([effect]);
        println!("community luck set to {luck_f64}");
    } else {
        let name = args[..args.len() - 1].join(" ");
        match community
            .members
            .values()
            .find(|m| m.display_name == name)
            .map(|m| m.id)
        {
            Some(member_id) => {
                let event = event_log
                    .append_event(id, EventPayload::SetMemberLuck { member_id, luck })
                    .unwrap();
                let mutations = vec![StateMutation::SetMemberLuck { member_id, luck }];
                let effect = event_log.append_effect(event.id, id, mutations).unwrap();
                let mut community = community;
                community.apply_effects([effect]);
                println!("{name} luck set to {luck_f64}");
            }
            None => println!("no member named '{name}'"),
        }
    }
}

fn cmd_log(event_log: &EventLogStore<&InMemoryEventLogRepo>, id: CommunityId, n: usize) {
    let records = event_log
        .get_records_before()
        .community_id(id)
        .limit(n)
        .call()
        .unwrap();
    if records.is_empty() {
        println!("no events recorded");
        return;
    }
    for record in &records {
        println!("[{}] {:#?}", record.sequence_id(), record);
    }
}

fn fetch(store: &Store<'_>, id: CommunityId) -> Community {
    store
        .get_latest(id)
        .expect("storage error")
        .expect("community not found")
}

fn print_community(community: &Community) {
    println!("community luck: {:.3}", community.luck());
    if community.members.is_empty() {
        println!("  (no members)");
        return;
    }
    let mut members: Vec<&Member> = community.members.values().collect();
    members.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    for member in members {
        println!("  {} (luck: {:.3}):", member.display_name, member.luck());
        let mut fruits: Vec<_> = member.bag.iter().collect();
        fruits.sort_by(|a, b| {
            a.0.category.cmp(&b.0.category).then(
                a.0.rarity()
                    .partial_cmp(&b.0.rarity())
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
        });
        if fruits.is_empty() {
            println!("    (empty bag)");
        } else {
            for (fruit, count) in fruits {
                println!("    {} {} ×{count}", fruit.emoji, fruit.name);
            }
        }
    }
}

fn print_help() {
    println!("commands:");
    println!("  add <name>           add a member");
    println!("  remove <name>        remove a member");
    println!("  grant <count>        grant N fruits to each member");
    println!("  gift <sender> <emoji> <recipient>  gift a fruit");
    println!("  burn <name> <emoji> [<emoji>...]  burn fruits (+community luck)");
    println!("  luck <value>         set community luck  (0.0–1.0)");
    println!("  luck <name> <value>  set member luck     (0.0–1.0)");
    println!("  log <n>              show the N most recent events");
    println!("  help                 show this message");
    println!("  quit / exit          quit");
}
