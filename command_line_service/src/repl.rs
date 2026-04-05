use std::io::{self, BufRead, Write};

use fruit_domain::{
    community::{Community, CommunityId},
    community_store::CommunityStore,
    event_log::{EventPayload, HasSequenceId, StateMutation},
    event_log_store::EventLogStore,
    granter::Granter,
    member::Member,
    random_granter::RandomGranter,
};
use fruit_in_memory_db::{
    community_repo::InMemoryCommunityRepo, event_log_repo::InMemoryEventLogRepo,
};

pub fn run() {
    let event_log_repo = InMemoryEventLogRepo::new();
    let event_log = EventLogStore::new(&event_log_repo);
    let store = CommunityStore::new(InMemoryCommunityRepo::new(), &event_log_repo);
    let community_id = {
        let community = Community::new();
        let id = community.id;
        store
            .put(community)
            .expect("failed to initialise community");
        id
    };
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
            "add" => cmd_add(&store, &event_log, community_id, &tokens[1..]),
            "remove" => cmd_remove(&store, &event_log, community_id, &tokens[1..]),
            "grant" => cmd_grant(&store, &event_log, community_id, &mut granter, &tokens[1..]),
            "luck" => cmd_luck(&store, community_id, &tokens[1..]),
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

fn cmd_add(
    store: &Store<'_>,
    event_log: &EventLogStore<&InMemoryEventLogRepo>,
    id: CommunityId,
    args: &[&str],
) {
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
    let effect = event_log.append_effect(event.id, id, mutations).unwrap();
    let mut community = fetch(store, id);
    effect.apply(&mut community);
    store.replace(community).unwrap();
    println!("added {name}");
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
            let effect = event_log.append_effect(event.id, id, mutations).unwrap();
            let mut community = community;
            effect.apply(&mut community);
            store.replace(community).unwrap();
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
    let community = fetch(store, id);
    let mutations = granter.grant(&community, count);
    let effect = event_log.append_effect(event.id, id, mutations).unwrap();
    let mut community = community;
    effect.apply(&mut community);
    store.replace(community).unwrap();
}

fn cmd_luck(store: &Store<'_>, id: CommunityId, args: &[&str]) {
    if args.is_empty() {
        println!("usage: luck <value>  |  luck <name> <value>");
        return;
    }

    let value_str = args.last().unwrap();
    let value: f64 = match value_str.parse() {
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

    let mut community = fetch(store, id);

    if args.len() == 1 {
        // No name — set community luck.
        let community = community.with_luck_f64(value);
        store.replace(community).unwrap();
        println!("community luck set to {value}");
    } else {
        // args[..args.len()-1] is the member name.
        let name = args[..args.len() - 1].join(" ");
        match community
            .members
            .values()
            .find(|m| m.display_name == name)
            .map(|m| m.id)
        {
            Some(member_id) => {
                let member = community.members.remove(&member_id).unwrap();
                community
                    .members
                    .insert(member_id, member.with_luck_f64(value));
                store.replace(community).unwrap();
                println!("{name} luck set to {value}");
            }
            None => println!("no member named '{name}'"),
        }
    }
}

fn cmd_log(event_log: &EventLogStore<&InMemoryEventLogRepo>, id: CommunityId, n: usize) {
    let records = event_log.get_latest_records(id, n).unwrap();
    if records.is_empty() {
        println!("no events recorded");
        return;
    }
    for record in &records {
        println!("[{}] {:#?}", record.sequence_id(), record)
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
    println!("  luck <value>         set community luck  (0.0–1.0)");
    println!("  luck <name> <value>  set member luck     (0.0–1.0)");
    println!("  log <n>              show the N most recent events");
    println!("  help                 show this message");
    println!("  quit / exit          quit");
}
