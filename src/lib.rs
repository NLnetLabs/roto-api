use ansi_term::Colour;
use chrono::{DateTime, Utc};
use num::PrimInt;
use rotonda_store::common::{AddressFamily, MergeUpdate, Prefix as RotondaPrefix};
use rotonda_store::{InMemNodeId, InMemStorage, SizedStrideNode, TreeBitMap};
use std::error::Error;
use std::fmt::Write;
use std::fs::File;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::Path;
use std::str::FromStr;
use std::{fmt, slice};

//------------ Addr ----------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub enum Addr {
    V4(u32),
    V6(u128),
}

impl From<Ipv4Addr> for Addr {
    fn from(addr: Ipv4Addr) -> Self {
        Self::V4(addr.into())
    }
}

impl From<Ipv6Addr> for Addr {
    fn from(addr: Ipv6Addr) -> Self {
        Self::V6(addr.into())
    }
}

impl From<IpAddr> for Addr {
    fn from(addr: IpAddr) -> Self {
        match addr {
            IpAddr::V4(addr) => addr.into(),
            IpAddr::V6(addr) => addr.into(),
        }
    }
}

impl FromStr for Addr {
    type Err = <IpAddr as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        IpAddr::from_str(s).map(Into::into)
    }
}

impl fmt::Display for Addr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Addr::V4(addr) => write!(f, "{}", std::net::Ipv4Addr::from(*addr)),
            Addr::V6(addr) => write!(f, "{}", std::net::Ipv6Addr::from(*addr)),
        }
    }
}

//------------ Prefix --------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub struct Prefix {
    pub addr: Addr,
    pub len: u8,
}

impl Prefix {
    pub fn new(addr: Addr, len: u8) -> Self {
        Prefix { addr, len }
    }
}

impl fmt::Display for Prefix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.addr, self.len)
    }
}

//------------ RecordSet -----------------------------------------------------

#[derive(Clone, Debug)]
pub struct RecordSet<'a> {
    v4: Vec<&'a RotondaPrefix<u32, ExtPrefixRecord>>,
    v6: Vec<&'a RotondaPrefix<u128, ExtPrefixRecord>>,
}

impl<'a> RecordSet<'a> {
    pub fn is_empty(&self) -> bool {
        self.v4.is_empty() && self.v6.is_empty()
    }

    pub fn iter(&self) -> RecordSetIter {
        RecordSetIter {
            v4: if self.v4.is_empty() {
                None
            } else {
                Some(self.v4.iter())
            },
            v6: self.v6.iter(),
        }
    }

    pub fn reverse(mut self) -> RecordSet<'a> {
        self.v4.reverse();
        self.v6.reverse();
        self
    }
}

//------------ RecordSetIter -------------------------------------------------

#[derive(Clone, Debug)]
pub struct RecordSetIter<'a, 'b> {
    v4: Option<slice::Iter<'a, &'b RotondaPrefix<u32, ExtPrefixRecord>>>,
    v6: slice::Iter<'a, &'b RotondaPrefix<u128, ExtPrefixRecord>>,
}

impl<'a, 'b> Iterator for RecordSetIter<'a, 'b> {
    type Item = (Prefix, Option<&'b ExtPrefixRecord>);

    fn next(&mut self) -> Option<Self::Item> {
        // V4 is already done.
        if self.v4.is_none() {
            return self.v6.next().map(|res| {
                (
                    Prefix {
                        addr: Addr::V6(res.net),
                        len: res.len,
                    },
                    res.meta.as_ref(),
                )
            });
        }

        if let Some(res) = self.v4.as_mut().and_then(|v4| v4.next()) {
            return Some((
                Prefix {
                    addr: Addr::V4(res.net),
                    len: res.len,
                },
                res.meta.as_ref(),
            ));
        }
        self.v4 = None;
        self.next()
    }
}

//------------ Rir -----------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub enum Rir {
    Afrinic,
    Apnic,
    Arin,
    Lacnic,
    RipeNcc,
    Unknown,
}

impl<'a> From<&'a str> for Rir {
    fn from(str: &str) -> Self {
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

impl<'a> fmt::Display for Rir {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Rir::Afrinic => write!(f, "AFRINIC"),
            Rir::Apnic => write!(f, "APNIC"),
            Rir::Arin => write!(f, "ARIN"),
            Rir::Lacnic => write!(f, "LACNIC"),
            Rir::RipeNcc => write!(f, "RIPE NCC"),
            Rir::Unknown => write!(f, "Unknown"),
        }
    }
}

//------------ ExtPrefixRecord -----------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct ExtPrefixRecord(pub Option<RirDelExtRecord>, pub Option<RisWhoisRecord>);

impl MergeUpdate for ExtPrefixRecord {
    fn merge_update(
        self: &mut Self,
        update_record: ExtPrefixRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if update_record.0.is_some() {
            self.0 = update_record.0
        }

        if update_record.1.is_some() {
            match &mut self.1 {
                Some(ris_whois_rec) => {
                    if let Some(update_ris_rec) = update_record.1 {
                        ris_whois_rec
                            .origin_asns
                            .0
                            .push(update_ris_rec.origin_asns.0[0]);
                    }
                }
                None => {
                    self.1 = update_record.1;
                }
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct RirDelExtRecord {
    group_id: String,
    pub rir: Rir,
}

// Not really used right now, since the
// impl Display isn't used either. May make sense
// to redefine Asn to be an enum that can either
// be a u32 or a PRIVATE_ASN.
#[derive(Clone, Debug)]
pub struct AsnArray(pub Vec<Asn>);

impl fmt::Display for AsnArray {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let arr_str = self.0.iter().fold("".to_string(), |as_arr, asn| {
            let asn_str: &str = &asn.to_string();
            as_arr + "AS" + asn_str
        });
        write!(f, "{}", arr_str)
    }
}
#[derive(Clone, Debug)]
pub struct RisWhoisRecord {
    pub origin_asns: AsnArray,
}
#[derive(Copy, Clone, Debug)]
pub struct Asn(u32);

impl fmt::Display for Asn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AS{}", self.0)
    }
}

impl Asn {
    fn from_str(as_str: &str) -> Result<Asn, std::num::ParseIntError> {
        as_str.parse::<u32>().map(|asn| Asn(asn))
    }
}

//------------ Store ---------------------------------------------------------

pub struct Store {
    v4: TreeBitMap<InMemStorage<u32, ExtPrefixRecord>>,
    v6: TreeBitMap<InMemStorage<u128, ExtPrefixRecord>>,
    updated: DateTime<Utc>,
}

impl Store {
    pub fn new() -> Self {
        Store {
            v4: TreeBitMap::new(vec![4]),
            v6: TreeBitMap::new(vec![4]),
            updated: Utc::now(),
        }
    }

    pub fn updated(&self) -> DateTime<Utc> {
        self.updated
    }

    pub fn load_riswhois(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        let file = File::open(path)?;
        let mut rdr = csv::Reader::from_reader(file);
        for result in rdr.records() {
            let record = result?;
            let net = Addr::from_str(&record[0]).unwrap_or_else(|_| {
                println!("Error parsing {}/{}", &record[0], &record[1]);
                panic!("can't continue parsing")
            });
            let len = u8::from_str(&record[1]).unwrap_or_else(|_| {
                println!("Error parsing {}/{}", &record[0], &record[1]);
                panic!("can't continue parsing")
            });
            let asn: Asn = Asn::from_str(&record[2]).unwrap_or_else(|e| {
                println!("{:?}", e);
                println!(
                    "Error parsing {}/{} with asn AS{}",
                    &record[0], &record[1], &record[2]
                );
                panic!("can't continue parsing")
            });
            let meta = ExtPrefixRecord(
                None,
                Some(RisWhoisRecord {
                    origin_asns: AsnArray(vec![asn]),
                }),
            );

            match net {
                Addr::V4(net) => self
                    .v4
                    .insert(RotondaPrefix::new_with_meta(net, len, meta))
                    .unwrap_or_else(|_| {
                        println!("Error parsing {} {}", net, len);
                        panic!("can't continue parsing")
                    }),
                Addr::V6(net) => self
                    .v6
                    .insert(RotondaPrefix::new_with_meta(net, len, meta))
                    .unwrap_or_else(|_| {
                        println!("Error parsing {} {}", net, len);
                        panic!("can't continue parsing")
                    }),
            }
        }
        self.updated = Utc::now();
        Ok(())
    }

    pub fn load_prefixes(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        let file = File::open(path)?;
        let mut rdr = csv::ReaderBuilder::new()
            .delimiter(b'|')
            .flexible(true)
            .trim(csv::Trim::Headers)
            .from_reader(file);

        for record in rdr.records() {
            let record = record?;

            if record[0].starts_with("#")
                || &record[5] == "summary"
                || &record[6] == "reserved"
                || &record[6] == "available"
            {
                continue;
            }

            let group_id = match record.get(7) {
                Some(id) => id.to_string(),
                None => continue,
            };

            let meta = ExtPrefixRecord(
                Some(RirDelExtRecord {
                    group_id,
                    rir: record[0].into(),
                }),
                None,
            );

            match &record[2] {
                "ipv4" => {
                    let net = Ipv4Addr::from_str(&record[3])?;

                    // record[4] is the number of addresses in the allocation.
                    // We assume that proper prefixes are allocated and then
                    // can do this:
                    let len_base = u32::from_str(&record[4])?;
                    let len: u8 = (len_base.leading_zeros() + 1) as u8;

                    self.v4
                        .insert(RotondaPrefix::new_with_meta(net.into(), len, meta))?;
                }
                "ipv6" => {
                    let net = Ipv6Addr::from_str(&record[3])?;

                    // record[4] is just the prefix length here. No shenanigans
                    // necessary.
                    let len = u8::from_str(&record[4])?;

                    self.v6
                        .insert(RotondaPrefix::new_with_meta(net.into(), len, meta))?;
                }
                _ => {}
            }
        }
        self.updated = Utc::now();
        Ok(())
    }

    pub fn match_longest_prefix(&self, prefix: Prefix) -> RecordSet {
        match prefix.addr {
            Addr::V4(addr) => RecordSet {
                v4: self
                    .v4
                    .match_longest_prefix(&RotondaPrefix::new(addr, prefix.len)),
                v6: Vec::new(),
            },
            Addr::V6(addr) => RecordSet {
                v4: Vec::new(),
                v6: self
                    .v6
                    .match_longest_prefix(&RotondaPrefix::new(addr, prefix.len)),
            },
        }
    }

    pub fn get_related_prefixes(&self, meta: &RirDelExtRecord) -> RecordSet {
        RecordSet {
            v4: Self::_get_related_prefixes(&self.v4, meta),
            v6: Self::_get_related_prefixes(&self.v6, meta),
        }
    }

    fn _get_related_prefixes<'b, T: AddressFamily>(
        tree: &'b TreeBitMap<InMemStorage<T, ExtPrefixRecord>>,
        meta: &RirDelExtRecord,
    ) -> Vec<&'b RotondaPrefix<T, ExtPrefixRecord>> {
        tree.store
            .prefixes
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
            .collect()
    }

    pub fn output_stats(&self) {
        println!("IPv4\n----");
        Self::output_tree_stats(&self.v4);
        println!("\nIPv6\n----");
        Self::output_tree_stats(&self.v6);
    }

    fn output_tree_stats<AF: AddressFamily + PrimInt + fmt::Debug>(
        tree_bitmap: &TreeBitMap<InMemStorage<AF, ExtPrefixRecord>>,
    ) {
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
            std::mem::size_of::<SizedStrideNode<AF, InMemNodeId>>()
        );
        println!(
            "memory used by nodes: {}kb",
            total_nodes * std::mem::size_of::<SizedStrideNode<AF, InMemNodeId>>() / 1024
        );
        println!(
            "size of prefix: {} bytes",
            std::mem::size_of::<RotondaPrefix<AF, ExtPrefixRecord>>()
        );
        println!(
            "memory used by prefixes: {}kb",
            tree_bitmap.store.prefixes.len()
                * std::mem::size_of::<RotondaPrefix<AF, ExtPrefixRecord>>()
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
}

//------------ JsonBuilder ---------------------------------------------------

/// A helper type for building a JSON-encoded string on the fly.
///
/// Note that the builder only supports strings without control characters.
pub struct JsonBuilder<'a> {
    target: &'a mut String,
    indent: usize,
    empty: bool,
}

impl JsonBuilder<'static> {
    pub fn build<F: FnOnce(&mut JsonBuilder)>(op: F) -> String {
        let mut target = String::new();
        JsonBuilder {
            target: &mut target,
            indent: 0,
            empty: true,
        }
        .array_object(op);
        target
    }
}

impl<'a> JsonBuilder<'a> {
    pub fn member_object<F: FnOnce(&mut JsonBuilder)>(&mut self, key: impl fmt::Display, op: F) {
        self.append_key(key);
        self.target.push_str("{\n");
        op(&mut JsonBuilder {
            target: self.target,
            indent: self.indent + 1,
            empty: true,
        });
        self.target.push('\n');
        self.append_indent();
        self.target.push('}');
    }

    pub fn member_array<F: FnOnce(&mut JsonBuilder)>(&mut self, key: impl fmt::Display, op: F) {
        self.append_key(key);
        self.target.push_str("[\n");
        op(&mut JsonBuilder {
            target: self.target,
            indent: self.indent + 1,
            empty: true,
        });
        self.target.push('\n');
        self.append_indent();
        self.target.push(']');
    }

    pub fn member_str(&mut self, key: impl fmt::Display, value: impl fmt::Display) {
        self.append_key(key);
        self.target.push('"');
        write!(
            JsonString {
                target: self.target
            },
            "{}",
            value
        )
        .unwrap();
        self.target.push('"');
    }

    pub fn member_raw(&mut self, key: impl fmt::Display, value: impl fmt::Display) {
        self.append_key(key);
        write!(
            JsonString {
                target: self.target
            },
            "{}",
            value
        )
        .unwrap();
    }

    pub fn array_object<F: FnOnce(&mut JsonBuilder)>(&mut self, op: F) {
        self.append_array_head();
        self.append_indent();
        self.target.push_str("{\n");
        op(&mut JsonBuilder {
            target: self.target,
            indent: self.indent + 1,
            empty: true,
        });
        self.target.push('\n');
        self.append_indent();
        self.target.push('}');
    }

    pub fn array_array<F: FnOnce(&mut JsonBuilder)>(&mut self, op: F) {
        self.append_array_head();
        self.append_indent();
        self.target.push_str("[\n");
        op(&mut JsonBuilder {
            target: self.target,
            indent: self.indent + 1,
            empty: true,
        });
        self.target.push('\n');
        self.append_indent();
        self.target.push(']');
    }

    pub fn array_str(&mut self, value: impl fmt::Display) {
        self.append_array_head();
        self.append_indent();
        self.target.push('"');
        write!(
            JsonString {
                target: self.target
            },
            "{}",
            value
        )
        .unwrap();
        self.target.push('"');
    }

    pub fn array_raw(&mut self, value: impl fmt::Display) {
        self.append_array_head();
        self.append_indent();
        write!(
            JsonString {
                target: self.target
            },
            "{}",
            value
        )
        .unwrap();
    }

    fn append_key(&mut self, key: impl fmt::Display) {
        if self.empty {
            self.empty = false
        } else {
            self.target.push_str(",\n");
        }
        self.append_indent();
        self.target.push('"');
        write!(
            JsonString {
                target: self.target
            },
            "{}",
            key
        )
        .unwrap();
        self.target.push('"');
        self.target.push_str(": ");
    }

    fn append_array_head(&mut self) {
        if self.empty {
            self.empty = false
        } else {
            self.target.push_str(",\n");
        }
    }

    fn append_indent(&mut self) {
        for _ in 0..self.indent {
            self.target.push_str("   ");
        }
    }
}

//------------ JsonString ----------------------------------------------------

struct JsonString<'a> {
    target: &'a mut String,
}

impl<'a> fmt::Write for JsonString<'a> {
    fn write_str(&mut self, mut s: &str) -> Result<(), fmt::Error> {
        while let Some(idx) = s.find(|ch| ch == '"' || ch == '\\') {
            self.target.push_str(&s[..idx]);
            self.target.push('\\');
            self.target.push(char::from(s.as_bytes()[idx]));
            s = &s[idx + 1..];
        }
        self.target.push_str(s);
        Ok(())
    }
}
