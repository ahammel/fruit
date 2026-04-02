use std::io::{self, BufRead, Write};

use gib_fruit_domain::{
    community::{Community, CommunityId},
    granter::Granter,
    member::Member,
    random_granter::RandomGranter,
    store::CommunityStore,
};
use gib_fruit_in_memory_db::community_repo::InMemoryCommunityRepo;

pub fn run() {
    let store = CommunityStore::new(InMemoryCommunityRepo::new());
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
        match tokens[0] {
            "add" => cmd_add(&store, community_id, &tokens[1..]),
            "remove" => cmd_remove(&store, community_id, &tokens[1..]),
            "grant" => cmd_grant(&store, community_id, &mut granter, &tokens[1..]),
            "luck" => cmd_luck(&store, community_id, &tokens[1..]),
            "help" => {}
            "quit" | "exit" => break,
            cmd => println!("unknown command '{cmd}'. Type 'help' for commands."),
        }
    }
}

fn cmd_add(store: &CommunityStore<InMemoryCommunityRepo>, id: CommunityId, args: &[&str]) {
    if args.is_empty() {
        println!("usage: add <name>");
        return;
    }
    let name = args.join(" ");
    let mut community = fetch(store, id);
    community.add_member(Member::new(&name));
    store.put(community).unwrap();
    println!("added {name}");
}

fn cmd_remove(store: &CommunityStore<InMemoryCommunityRepo>, id: CommunityId, args: &[&str]) {
    if args.is_empty() {
        println!("usage: remove <name>");
        return;
    }
    let name = args.join(" ");
    let mut community = fetch(store, id);
    match community
        .members
        .values()
        .find(|m| m.display_name == name)
        .map(|m| m.id)
    {
        Some(member_id) => {
            community.remove_member(member_id);
            store.put(community).unwrap();
            println!("removed {name}");
        }
        None => println!("no member named '{name}'"),
    }
}

fn cmd_grant(
    store: &CommunityStore<InMemoryCommunityRepo>,
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
    let mut community = fetch(store, id);
    granter.grant(&mut community, count);
    store.put(community).unwrap();
}

fn cmd_luck(store: &CommunityStore<InMemoryCommunityRepo>, id: CommunityId, args: &[&str]) {
    if args.is_empty() {
        println!("usage: luck <value>  |  luck <name> <value>");
        return;
    }

    let value_str = args.last().unwrap();
    let value = match value_str.parse::<f64>() {
        Ok(v) if v.is_finite() => v,
        Ok(_) => {
            println!("luck must be finite");
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
        let community = community.with_luck(value);
        store.put(community).unwrap();
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
                community.members.insert(member_id, member.with_luck(value));
                store.put(community).unwrap();
                println!("{name} luck set to {value}");
            }
            None => println!("no member named '{name}'"),
        }
    }
}

fn fetch(store: &CommunityStore<InMemoryCommunityRepo>, id: CommunityId) -> Community {
    store
        .get(id)
        .expect("storage error")
        .expect("community not found")
}

fn print_community(community: &Community) {
    println!("community luck: {:.2}", community.luck());
    if community.members.is_empty() {
        println!("  (no members)");
        return;
    }
    let mut members: Vec<&Member> = community.members.values().collect();
    members.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    for member in members {
        println!("  {} (luck: {:.2}):", member.display_name, member.luck());
        let mut fruits: Vec<_> = member.bag.iter().collect();
        fruits.sort_by(|a, b| {
            a.0.category
                .cmp(&b.0.category)
                .then(a.0.rarity.cmp(&b.0.rarity))
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
    println!("  luck <value>         set community luck");
    println!("  luck <name> <value>  set member luck");
    println!("  help                 show this message");
    println!("  quit / exit          quit");
}
