use core::str::FromStr;
use std::io::{self, BufRead, Read, Write};

use semilattice::SemiLattice;
use threads::{detailed::Detailed, Actor, ActorID, Root, Slice};

fn usage(code: i32) -> ! {
    print!(
        "\
Dummy client to interact with threads, from the perspective of each actor.

USAGE:
  cargo run -- SUBCOMMAND

FLAGS:
  -h, --help            Prints help information
  -a, --actor           Who are you? {{Alice, Bob, Carol, Dave, Eve}}

SUBCOMMANDS:
  list          List all threads
  new           Create a new thread
  reply         Reply to any message (WARNING: can create cycles if you lie)
  edit          Edit your own message
  react         React to any message
  reset         Reset the actor's slice
  dump          Debug print the root object
"
    );

    std::process::exit(code);
}

fn parse_actor(s: &str) -> ActorID {
    ActorID::from_str(s).unwrap_or_else(|_| {
        println!("I don't know who {} is.", s);
        usage(1);
    })
}

fn main() -> Result<(), pico_args::Error> {
    let mut pargs = pico_args::Arguments::from_env();

    if pargs.contains(["-h", "--help"]) {
        usage(0);
    }

    let actor_name: String = pargs.value_from_str(["-a", "--actor"])?;

    let repo = git2::Repository::open_bare(format!("{}/actors/shared", env!("CARGO_MANIFEST_DIR")))
        .expect(
            "Failed to open repository. Did you forget to run `git init --bare threads/actors/shared`?",
        );

    repo.set_namespace("threads")
        .expect("Failed to enter the 'threads's namespace.");

    let mut root = Root::default();
    /* // Load materialized cache.
    let mut root: Root = minicbor::decode(
        repo.find_reference("refs/threads-materialized")
            .map(|r| r.peel_to_blob().expect("Expected blob"))
            .expect("Failed to lookup reference")
            .content(),
    )
    .expect("Failed to decode");
    */

    // Import each writer's slice.
    let threads_tree = repo
        .find_reference("refs/threads")
        .and_then(|r| r.peel_to_tree());

    if let Ok(ref tree) = threads_tree {
        tree.walk(git2::TreeWalkMode::PreOrder, |_, entry| {
            let actor = parse_actor(entry.name().expect("Invalid reference name"));
            root.entry(actor).join_assign(
                minicbor::decode(
                    entry
                        .to_object(&repo)
                        .expect("Failed to lookup blob")
                        .peel_to_blob()
                        .expect("Expected blob!")
                        .content(),
                )
                .expect("Invalid CBOR"),
            );
            git2::TreeWalkResult::Ok
        })
        .expect("Failed to walk tree.");
    }

    let mut actor = {
        let aid = parse_actor(&actor_name);
        Actor::new(root.entry(aid.clone()), aid, 0)
    };

    let input = io::stdin();
    let mut input = input.lock();

    fn read_line(prompt: &str, input: &mut impl BufRead) -> String {
        print!("{} ", prompt);
        io::stdout().flush().unwrap();

        let mut tmp = String::new();
        input.read_line(&mut tmp).expect("Failed to read line");
        tmp
    }

    fn read_to_string(prompt: &str, input: &mut impl Read) -> String {
        print!("{} ", prompt);
        io::stdout().flush().unwrap();

        let mut tmp = String::new();
        input
            .read_to_string(&mut tmp)
            .expect("Failed to read to string");
        tmp
    }

    let mut buffer = Vec::new();

    match &*pargs.subcommand()?.expect("Expected subcommand!") {
        "dump" => {
            let mut dump = Vec::new();

            minicbor::encode(&root, &mut dump).expect("Failed to encode dump to CBOR");

            println!("{}", minicbor::display(&dump));
        }
        "list" => {
            Detailed::default().join(root).display();
        }
        "new" => {
            actor.new_thread(
                read_line("Title:", &mut input).trim().to_owned(),
                read_to_string("Body:", &mut input).trim().to_owned(),
                [],
            );

            minicbor::encode(&actor.slice, &mut buffer)
                .expect("Failed to CBOR encode actor slice.");
        }
        "reply" => {
            actor.reply(
                (
                    parse_actor(read_line("Reply to who?", &mut input).trim()),
                    read_line("Message ID:", &mut input)
                        .trim()
                        .parse()
                        .expect("Invalid number"),
                ),
                read_to_string("Body:", &mut input).trim().to_owned(),
            );

            minicbor::encode(&actor.slice, &mut buffer)
                .expect("Failed to CBOR encode actor slice.");
        }
        "edit" => {
            actor.edit(
                read_line("Message ID:", &mut input)
                    .trim()
                    .parse()
                    .expect("Invalid number"),
                read_to_string("Body:", &mut input).trim().to_owned(),
            );

            minicbor::encode(&actor.slice, &mut buffer)
                .expect("Failed to CBOR encode actor slice.");
        }
        "redact" => {
            actor.redact(
                read_line("Message ID:", &mut input)
                    .trim()
                    .parse()
                    .expect("Invalid number"),
                read_line("Version:", &mut input)
                    .trim()
                    .parse()
                    .expect("Invalid number"),
            );

            minicbor::encode(&actor.slice, &mut buffer)
                .expect("Failed to CBOR encode actor slice.");
        }
        "react" => {
            let target_actor =
                parse_actor(read_line("Which actor authored the message?", &mut input).trim());

            let message_id = read_line("Message ID:", &mut input)
                .trim()
                .parse()
                .expect("Invalid number");

            let line = read_line("Reaction:", &mut input);
            let mut reaction = line.trim();
            let mut positive = true;
            if reaction.starts_with('-') {
                reaction = &reaction[1..];
                positive = false;
            }

            actor.react((target_actor, message_id), reaction.to_owned(), positive);

            minicbor::encode(&actor.slice, &mut buffer)
                .expect("Failed to CBOR encode actor slice.");
        }
        "tag" => {
            let message_id = (
                parse_actor(read_line("Which actor started the thread?", &mut input).trim()),
                read_line("Message ID:", &mut input)
                    .trim()
                    .parse()
                    .expect("Invalid number"),
            );

            let line = read_line("Add comma separated tags:", &mut input);
            let additive = line.trim().split(',').map(|x| x.trim().to_owned());

            let line = read_line("Remove comma separated tags:", &mut input);
            let negative = line.trim().split(',').map(|x| x.trim().to_owned());

            actor.adjust_tags(message_id, additive, negative);

            minicbor::encode(&actor.slice, &mut buffer)
                .expect("Failed to CBOR encode actor slice.");
        }
        "reset" => {
            // clear user slice
            minicbor::encode(&Slice::default(), &mut buffer)
                .expect("Failed to CBOR encode actor slice.");
        }
        "import" => {
            /*
            #[derive(serde::Serialize, serde::Deserialize)]
            struct Comment {
                author_id: Option<String>,
                body: String,
            }

            #[derive(serde::Serialize, serde::Deserialize)]
            struct Issue {
                title: String,
                body: String,
                author_id: Option<String>,
                comments: Vec<Comment>,
            }

            use std::fs;

            for path in fs::read_dir(
                env!("GITHUB_ISSUE_IMPORT_PATH"),
            ).expect("Failed to open directory. Does there exist a directory at GITHUB_ISSUE_IMPORT_PATH ?")
            .map(|res| res.map(|e| e.path()))
            {
                let Issue { title, body, author_id, comments } = serde_json::from_str(
                    &fs::read_to_string(path.expect("IO Error?")).expect("Failed to read file")
                ).expect("Failed to decode JSON");

                let author_id = author_id.unwrap_or("ghost".to_owned());
                let thread_id = Actor::new(root.entry(author_id.clone()), author_id, 0).new_thread(title, body, []);

                //print!(">");

                for Comment { author_id, body } in comments {
                let author_id = author_id.unwrap_or("ghost".to_owned());
                    //print!(".");
                    Actor::new(root.entry(author_id.clone()), author_id, 0).reply(thread_id.clone(), body);
                }

                //println!();
            }

            let mut tree = repo
                .treebuilder(threads_tree.ok().as_ref())
                .expect("Failed to create tree.");

            let mut buffer = Vec::new();

            for (name, user) in root.inner {
                buffer.clear();
                minicbor::encode(&user, &mut buffer)
                    .expect("Failed to CBOR encode actor slice.");

                tree.insert(
                    &name,
                    repo.blob(&buffer).expect("Failed to record blob."),
                    0o160000,
                )
                .expect("Failed to insert blob into tree.");
            }

            let tree_oid = tree.write().expect("Failed to write tree.");

            println!(
                "Written state to: {}",
                repo.reference("refs/threads", tree_oid, true, "log msg",)
                    .expect("Failed to update reference")
                    .name()
                    .expect("Invalid reference name?")
            );
            */
        }
        "cache" => {
            let mut buffer = Vec::new();
            minicbor::encode(&root, &mut buffer).expect("Failed to CBOR encode root.");

            println!(
                "Written state to: {}",
                repo.reference(
                    "refs/threads-materialized",
                    repo.blob(&buffer).expect("Failed to write blob"),
                    true,
                    "log msg",
                )
                .expect("Failed to update reference")
                .name()
                .expect("Invalid reference name?")
            );
        }
        _ => usage(2),
    }

    // write back our changes
    if !buffer.is_empty() {
        let mut tree = repo
            .treebuilder(threads_tree.ok().as_ref())
            .expect("Failed to create tree.");

        tree.insert(
            &actor_name,
            repo.blob(&buffer).expect("Failed to record blob."),
            0o160000,
        )
        .expect("Failed to insert blob into tree.");

        let tree_oid = tree.write().expect("Failed to write tree.");

        println!(
            "Written state to: {}",
            repo.reference("refs/threads", tree_oid, true, "log msg",)
                .expect("Failed to update reference")
                .name()
                .expect("Invalid reference name?")
        );
    }

    Ok(())
}
