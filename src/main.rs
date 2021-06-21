use std::{env, process};
use std::str::FromStr;
use rir_lir::{Addr, Prefix, Store};
use rustyline::Editor;
use rustyline::error::ReadlineError;

fn main() {
    let mut args = env::args();
    let cmd = match args.next() {
        Some(cmd) => cmd,
        None => {
            eprintln!("Fatal: failed to understand command line.");
            process::exit(1);
        }
    };
    let prefix_path = match args.next() {
        Some(path) => path,
        None => {
            eprintln!(
                "Usage: {} <prefixes-file> <ris-file> [<ris-file> ...]",
                cmd
            );
            process::exit(1);
        }
    };

    let mut store = Store::new();
    if let Err(err) = store.load_prefixes(prefix_path.as_ref()) {
        eprintln!(
            "Failed to load {}: {}",
            prefix_path, err
        );
        process::exit(1);
    }
    for path in args {
        if let Err(err) = store.load_riswhois(path.as_ref()) {
            eprintln!(
                "Failed to load {}: {}",
                path, err
            );
            process::exit(1);
        }
    }

    let mut rl = Editor::<()>::new();
    if rl.load_history("/tmp/rotonda-store-history.txt").is_err() {
        eprintln!("No previous history.");
    }

    store.output_stats();

    loop {
        let readline = rl.readline("(rotonda-store)> ");
        match readline {
            Ok(line) => {
                let s_pref: Vec<&str> = line.split("/").collect();

                if s_pref.len() < 2 {
                    eprintln!(
                        "Error: can't parse prefix {:?}. \
                        Maybe add a /<LEN> part?",
                        s_pref
                    );
                    continue;
                }

                let len = s_pref[1].parse::<u8>().unwrap();
                let ip = match Addr::from_str(s_pref[0]) {
                    Ok(ip) => ip,
                    Err(err) => {
                        eprintln!(
                            "Error: Can't parse address part. {:?}: {}",
                            s_pref[0], err
                        );
                        continue;
                    }
                };

                rl.add_history_entry(line.as_str());
                println!("Searching for prefix: {}/{}", ip, len);

                let lmp_pfx = store.match_longest_prefix(Prefix::new(ip, len));
                println!(
                    "Found less-specific and exactly matching prefixes: {:#?}",
                    lmp_pfx
                );

                // Find longest prefix.
                let key_pfx = {
                    lmp_pfx
                    .iter()
                    .filter_map(|item| {
                        item.1.and_then(|item| item.0.as_ref()).map(|some| {
                            (item.0, some)
                        })
                    })
                    .max_by_key(|item| item.0.len)
                };

                if let Some(key_pfx) = key_pfx {
                    let related_pfxs = store.get_related_prefixes(key_pfx.1);
                    println!(
                        "Found prefixes allocated to same organisation as prefix {}/{}:",
                        key_pfx.0.addr, key_pfx.0.len
                    );
                    println!("{:#?}", related_pfxs);
                }
                else {
                    println!("No related prefixes found.");
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                continue;
            }
        }
    }
    rl.save_history("/tmp/rotonda-store-history.txt").unwrap();
}
