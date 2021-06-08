use rotonda_store::common::{AddressFamily, MergeUpdate, Prefix};
use rotonda_store::{InMemNodeId, InMemStorage, SizedStrideNode, TreeBitMap};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fs::File;
use std::process;

use num::PrimInt;
use std::fmt::Debug;

use ansi_term::Colour;
use rustyline::error::ReadlineError;
use rustyline::Editor;
#[derive(Debug)]
enum Rir {
    Afrinic,
    Apnic,
    Arin,
    Lacnic,
    RipeNcc,
    Unknown,
}

#[derive(Debug)]
pub struct ExtPrefixRecord(Option<RirDelExtRecord>, Option<RisWhoisRecord>);

impl MergeUpdate for ExtPrefixRecord {
    fn merge_update(
        self: &mut Self,
        update_record: ExtPrefixRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if update_record.0.is_some() {
            self.0 = update_record.0
        }

        if update_record.1.is_some() {
            self.1 = update_record.1
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct RirDelExtRecord {
    group_id: String,
    rir: Rir,
}
#[derive(Debug)]
pub struct RisWhoisRecord {
    origin_as: Vec<String>,
}

impl Rir {
    fn match_from_str(str: &str) -> Self {
        match str {
            "afrinic" => Self::Afrinic,
            "apnic" => Self::Apnic,
            "arin" => Self::Arin,
            "lacnic" => Self::Lacnic,
            "ripencc" => Self::RipeNcc,
            _ => Self::Unknown,
        }
    }
}

type NodeType = InMemNodeId;

pub fn get_related_prefixes<'a>(
    prefixes: &'a Vec<rotonda_store::common::Prefix<u32, ExtPrefixRecord>>,
    pfx: &'a Prefix<u32, ExtPrefixRecord>,
) -> Vec<&'a Prefix<u32, ExtPrefixRecord>> {
    match pfx.meta.as_ref() {
        // There's no meta on the prefix we're searching related prefixes for.
        // return with empty vec.
        None => vec![],
        // meta exists on the prefix we're searching others for.
        Some(prefix_rec) => match &prefix_rec.0 {
            // Meta exists, but there's no RirDelExtRecord, the record that
            // holds the group_id that we use to relate other prefixes.
            None => vec![],
            Some(meta) => prefixes
                .iter()
                .filter(|&rel_p| {
                    if let Some(rel_p_meta) = rel_p.meta.as_ref() {
                        if let Some(rel_p_meta_rde) = rel_p_meta.0.as_ref() {
                            rel_p_meta_rde.group_id == meta.group_id
                        } else {
                            return false;
                        }
                    } else {
                        return false;
                    }
                })
                .collect(),
        },
    }
}

fn load_riswhois(
    tree_v4: &mut TreeBitMap<InMemStorage<u32, ExtPrefixRecord>>,
    _tree_v6: &mut TreeBitMap<InMemStorage<u128, ExtPrefixRecord>>,
) -> Result<(), Box<dyn Error>> {
    // Build the CSV reader and iterate over each record.
    let mut count: u64 = 0;
    let file_path = get_second_arg()?;
    let file = File::open(file_path)?;
    let mut rdr = csv::Reader::from_reader(file);
    for result in rdr.records() {
        count += 1;
        if count % 10_000 == 0 {
            print!("..{}", count)
        }
        let record = result?;
        let ip: Vec<_> = record[0]
            .split(".")
            .map(|o| -> u8 { o.parse().unwrap() })
            .collect();
        let net = std::net::Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]);
        let len: u8 = record[1].parse().unwrap();
        let asn = record[2].to_string();

        let pfx = Prefix::<u32, ExtPrefixRecord>::new_with_meta(
            net.into(),
            len,
            ExtPrefixRecord(
                None,
                Some(RisWhoisRecord {
                    origin_as: vec![asn],
                }),
            ),
        );
        tree_v4.insert(pfx)?;
    }
    Ok(())
}

fn load_prefixes(
    tree_v4: &mut TreeBitMap<InMemStorage<u32, ExtPrefixRecord>>,
    tree_v6: &mut TreeBitMap<InMemStorage<u128, ExtPrefixRecord>>,
) -> Result<(), Box<dyn Error>> {
    // Build the CSV reader and iterate over each record.
    let file_path = get_first_arg()?;

    let mut asns: HashMap<String, Option<String>> = HashMap::new();

    let file = File::open(file_path)?;
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b'|')
        .flexible(true)
        .trim(csv::Trim::Headers)
        .from_reader(file);

    let mut tmp_records: Vec<csv::StringRecord> = vec![];

    // loop through all looking for ASNs
    for result in rdr.records() {
        let record = result?;

        if record[0].bytes().nth(0) == Some(b'#')
            || (record.get(7).is_none() && record[2] == "asn".to_string())
        {
            continue;
        }

        if record[2] == "asn".to_string()
            && record[5] != "summary".to_string()
            && record[6] != "reserved".to_string()
        {
            asns.insert(record[7].to_string(), Some(record[3].to_string()));
        } else {
            tmp_records.push(record);
        }
    }

    for record in tmp_records {
        if record[2] == "asn".to_string()
            && record[5] != "summary".to_string()
            && record[6] != "reserved".to_string()
            && record.get(7).is_some()
            && record[7] != "reserved".to_string()
        {
            asns.insert(record[7].to_string(), Some(record[3].to_string()));
        }

        if record[0].bytes().nth(0) != Some(b'#')
            // && record[2] == "ipv4".to_string()
            && record[5] != "summary".to_string()
            && record[6] != "reserved".to_string()
            && record[6] != "available".to_string()
        {
            match &record[2] {
                "ipv4" => {
                    let net: std::net::Ipv4Addr = record[3].parse().unwrap();

                    let len_base = record[4].parse::<u32>().unwrap();
                    let len: u8 = (len_base.leading_zeros() + 1) as u8;
                    let pfx = Prefix::<u32, ExtPrefixRecord>::new_with_meta(
                        net.into(),
                        len,
                        ExtPrefixRecord(
                            Some(RirDelExtRecord {
                                group_id: record[7].to_string(),
                                rir: Rir::match_from_str(&record[0]),
                            }),
                            None,
                        ),
                    );

                    if len_base.leading_zeros() + len_base.trailing_zeros() + 1 != 32 {
                        print!(".");
                    }
                    tree_v4.insert(pfx)?;
                }
                "ipv6" => {
                    let net: std::net::Ipv6Addr = record[3].parse().unwrap();
                    // let len_base = record[4].parse::<u128>().unwrap();
                    // let len: u8 = (len_base.leading_zeros() + 1) as u8;
                    let pfx = Prefix::<u128, ExtPrefixRecord>::new_with_meta(
                        net.into(),
                        record[4].parse::<u8>().unwrap(),
                        ExtPrefixRecord(
                            Some(RirDelExtRecord {
                                group_id: record[7].to_string(),
                                rir: Rir::match_from_str(&record[0]),
                            }),
                            None,
                        ),
                    );
                    tree_v6.insert(pfx)?;
                }
                _ => {}
            }
        }
    }
    println!("");
    Ok(())
}

fn output_stats<AF>(tree_bitmap: &TreeBitMap<InMemStorage<AF, ExtPrefixRecord>>)
where
    AF: AddressFamily + PrimInt + Debug,
{
    let total_nodes = tree_bitmap.stats.iter().fold(0, |mut acc, c| {
        acc += c.created_nodes.iter().fold(0, |mut sum, l| {
            sum += l.count;
            sum
        });
        acc
    });
    println!("prefix vec size {}", tree_bitmap.store.prefixes.len());
    println!("finished building tree...");
    println!("{:?} nodes created", total_nodes);
    println!(
        "size of node: {} bytes",
        std::mem::size_of::<SizedStrideNode<AF, NodeType>>()
    );
    println!(
        "memory used by nodes: {}kb",
        total_nodes * std::mem::size_of::<SizedStrideNode<AF, NodeType>>() / 1024
    );
    println!(
        "size of prefix: {} bytes",
        std::mem::size_of::<Prefix<AF, ExtPrefixRecord>>()
    );
    println!(
        "memory used by prefixes: {}kb",
        tree_bitmap.store.prefixes.len() * std::mem::size_of::<Prefix<AF, ExtPrefixRecord>>()
            / 1024
    );
    println!("stride division  {:?}", tree_bitmap.strides);

    for s in &tree_bitmap.stats {
        println!("{:?}", s);
    }

    println!(
        "level\t[{}|{}] nodes occupied/max nodes percentage_max_nodes_occupied prefixes",
        Colour::Blue.paint("nodes"),
        Colour::Green.paint("prefixes")
    );
    let bars = ["▏", "▎", "▍", "▌", "▋", "▊", "▉"];
    let mut stride_bits = [0, 0];
    const SCALE: u32 = 5500;

    for stride in tree_bitmap.strides.iter().enumerate() {
        // let level = stride.0;
        stride_bits = [stride_bits[1] + 1, stride_bits[1] + stride.1];
        let nodes_num = tree_bitmap
            .stats
            .iter()
            .find(|s| s.stride_len == *stride.1)
            .unwrap()
            .created_nodes[stride.0]
            .count as u32;

        if nodes_num > 0 {
            let prefixes_num = tree_bitmap
                .stats
                .iter()
                .find(|s| s.stride_len == *stride.1)
                .unwrap()
                .prefixes_num[stride.0]
                .count as u32;

            let n = (nodes_num / SCALE) as usize;
            let max_pfx: u128 = u128::pow(2, stride_bits[1] as u32);

            print!("{}-{}\t", stride_bits[0], stride_bits[1]);

            for _ in 0..n {
                print!("{}", Colour::Blue.paint("█"));
            }

            print!(
                "{}",
                Colour::Blue.paint(bars[((nodes_num % SCALE) / (SCALE / 7)) as usize]) //  = scale / 7
            );

            print!(
                " {}/{} {:.2}%",
                nodes_num,
                max_pfx,
                (nodes_num as f64 / max_pfx as f64) * 100.0
            );
            print!("\n\t");

            let n = (prefixes_num / SCALE) as usize;
            for _ in 0..n {
                print!("{}", Colour::Green.paint("█"));
            }

            print!(
                "{}",
                Colour::Green.paint(bars[((nodes_num % SCALE) / (SCALE / 7)) as usize]) //  = scale / 7
            );

            println!(" {}", prefixes_num);
        }
    }
}

fn get_first_arg() -> Result<OsString, Box<dyn Error>> {
    match env::args_os().nth(1) {
        None => Err(From::from("expected 2 arguments, but got none")),
        Some(file_path) => Ok(file_path),
    }
}

fn get_second_arg() -> Result<OsString, Box<dyn Error>> {
    match env::args_os().nth(2) {
        None => Err(From::from(
            "expected 2 arguments, but didn't get a second one",
        )),
        Some(file_path) => Ok(file_path),
    }
}

fn main() {
    let mut tree_v4: TreeBitMap<InMemStorage<u32, ExtPrefixRecord>> = TreeBitMap::new(vec![4]);

    let mut tree_v6: TreeBitMap<InMemStorage<u128, ExtPrefixRecord>> = TreeBitMap::new(vec![4]);

    if let Err(err) = load_prefixes(&mut tree_v4, &mut tree_v6) {
        println!("error running example: {}", err);
        process::exit(1);
    }

    println!("loading riswhois...");
    if let Err(err) = load_riswhois(&mut tree_v4, &mut tree_v6) {
        println!("error running example: {}", err);
        process::exit(1);
    }

    println!("v4 prefix vec size {}", tree_v4.store.prefixes.len());
    println!("v6 prefix vec size {}", tree_v6.store.prefixes.len());

    let mut rl = Editor::<()>::new();
    if rl.load_history("/tmp/rotonda-store-history.txt").is_err() {
        println!("No previous history.");
    }

    output_stats(&tree_v4);
    output_stats(&tree_v6);

    loop {
        let readline = rl.readline("(rotonda-store)> ");
        match readline {
            Ok(line) => {
                let s_pref: Vec<&str> = line.split("/").collect();

                if s_pref.len() < 2 {
                    println!(
                        "Error: can't parse prefix {:?}. Maybe add a /<LEN> part?",
                        s_pref
                    );
                    continue;
                }

                let len = s_pref[1].parse::<u8>().unwrap();
                let ip: Result<std::net::Ipv4Addr, _> = s_pref[0].parse();
                let pfx;

                match ip {
                    Ok(ip) => {
                        rl.add_history_entry(line.as_str());
                        println!("Searching for prefix: {}/{}", ip, len);
                        pfx = Prefix::<u32, ExtPrefixRecord>::new(ip.into(), len);
                        let lmp_pfx = tree_v4.match_longest_prefix(&pfx.strip_meta());
                        println!(
                            "Found less-specific and exactly matching prefixes: {:#?}",
                            lmp_pfx
                        );
                        if lmp_pfx.len() > 0 {
                            let related_pfxs =
                                get_related_prefixes(&tree_v4.store.prefixes, lmp_pfx[0]);
                            println!(
                                "Found prefixes allocated to same organisation as prefix {}/{}:",
                                lmp_pfx[0].net, lmp_pfx[0].len
                            );
                            println!("{:#?}", related_pfxs);
                        }
                        else {
                            println!("No related prefixes found.");
                        }
                    }
                    Err(err) => {
                        println!("Error: Can't parse address part. {:?}: {}", s_pref[0], err);
                    }
                };
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
