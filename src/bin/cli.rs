use roto_api::{Addr, MatchOptions, MatchType, Prefix, Store, RecordSet};
use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::str::FromStr;
use std::{env, process};

fn lookup_related_prefixes_for_lmp_in<'a>(rec: &'a RecordSet) -> Option<(roto_api::Prefix, &'a roto_api::RirDelExtRecord)> {
    rec.iter()
        .filter_map(|item| {
            item.1
                .and_then(|item| item.0.as_ref())
                .map(|some| (item.0, some))
        })
        .max_by_key(|item| item.0.len)
}

fn main() {
    let match_options = MatchOptions {
        match_type: MatchType::EmptyMatch,
        include_less_specifics: true,
        include_more_specifics: true,
    };
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
            eprintln!("Usage: {} <prefixes-file> <ris-file> [<ris-file> ...]", cmd);
            process::exit(1);
        }
    };

    let mut store: Store = Default::default();
    if let Err(err) = store.load_prefixes(prefix_path.as_ref()) {
        eprintln!("Failed to load {}: {}", prefix_path, err);
        process::exit(1);
    }
    for path in args {
        if let Err(err) = store.load_riswhois(path.as_ref()) {
            eprintln!("Failed to load {}: {}", path, err);
            process::exit(1);
        }
    }

    let mut rl = Editor::<()>::new();
    if rl.load_history("/tmp/rotonda-store-history.txt").is_err() {
        eprintln!("No previous history.");
    }

    store.output_stats();

    loop {
        let readline = rl.readline("(roto-api-cli)> ");
        match readline {
            Ok(line) => {
                let s_pref: Vec<&str> = line.split('/').collect();

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
                        eprintln!("Error: Can't parse address part. {:?}: {}", s_pref[0], err);
                        continue;
                    }
                };

                rl.add_history_entry(line.as_str());
                println!("Searching for prefix: {}/{}", ip, len);

                let lmp_pfx = match ip {
                    Addr::V4(_addr) => {
                        store.match_longest_prefix::<u32>(Prefix::new(ip, len), &match_options)
                    }
                    Addr::V6(_addr) => {
                        store.match_longest_prefix::<u128>(Prefix::new(ip, len), &match_options)
                    }
                };
                let pfx_str = lmp_pfx
                    .prefix
                    .map_or("none".to_string(), |pfx| pfx.to_string());
                println!("Found Prefix ({:?}): {}", match_options.match_type, pfx_str);
                println!("Meta data for prefix: {:?}", lmp_pfx.prefix_meta);
                println!("less-specifics: {:#?}", lmp_pfx.less_specifics);
                println!("more-specifics: {:#?}", lmp_pfx.more_specifics);

                // Find longest prefix.
                let key_pfx = match lmp_pfx.prefix_meta {
                    Some(meta) => match &meta.0 {
                        Some(rir_rec) => Some((lmp_pfx.prefix.unwrap(), rir_rec)),
                        _ => lookup_related_prefixes_for_lmp_in(&lmp_pfx.less_specifics),
                    },
                    _ => lookup_related_prefixes_for_lmp_in(&lmp_pfx.less_specifics),
                };

                if let Some(key_pfx) = key_pfx {
                    let related_pfxs = store.get_related_prefixes(key_pfx.1);
                    println!(
                        "Found prefixes allocated to same organisation as prefix {}/{}:",
                        key_pfx.0.addr, key_pfx.0.len
                    );
                    println!("{:#?}", related_pfxs);
                } else {
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
