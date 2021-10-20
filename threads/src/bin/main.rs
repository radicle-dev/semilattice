use semilattice::SemiLattice;
use threads::{detailed::Detailed, Root};

fn usage(code: i32) {
    print!(
        "\
unnamed

USAGE:
  cargo run -- SUBCOMMAND

FLAGS:
  -h, --help            Prints help information
  -a, --actor           Who are you? {{Alice, Bob, Carol, Dave, Eve}}

SUBCOMMANDS:
  list
  new
  reply
  edit
  react
"
    );

    std::process::exit(code);
}

fn main() -> Result<(), pico_args::Error> {
    let mut pargs = pico_args::Arguments::from_env();

    if pargs.contains(["-h", "--help"]) {
        usage(0);
    }

    let actor: String = pargs.value_from_str(["-a", "--actor"])?;

    if !["alice", "bob", "carol", "dave", "eve"].contains(&&*actor) {
        println!("I don't know who {} is.", actor);
        usage(1);
    }

    let repo =
        git2::Repository::open_bare(format!("{}/actors/{}", env!("CARGO_MANIFEST_DIR"), actor))
            .expect("Failed to open repository");

    repo.set_namespace("threads")
        .expect("Failed to enter the 'threads's namespace.");

    let mut root = Root::default();

    for actor in [] {
        root.entry(actor).join_assign(Default::default());
    }

    Detailed::default().join(root).display();

    match &*pargs.subcommand()?.expect("Expected subcommand!") {
        "list" => {
            // list threads
        }
        "new" => {
            // new thread, prompt for title and message body. Preview. Confirm.

            let reference = repo.reference(
                "refs/test",
                repo.blob(b"Hello world").expect("Failed to record blob."),
                true,
                "log msg",
            );
        }
        "reply" => {
            // reply to comment. Prompt for message body. Preview. Confirm.
        }
        "edit" => {
            // select message. Prompt for message body. Preview. Confirm.

            // If the new body is empty, ask if they want to clear or redact
            // the message.
        }
        "react" => {
            // select message, select reaction.
        }
        _ => usage(2),
    }

    Ok(())
}
