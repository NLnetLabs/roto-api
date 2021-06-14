## start, load files and present a CLI

```cargo run --release -- ./data/delegated-all ./data/uniq_pfx_asn_dfz_v4.csv```

delegated-all is the concatenation of all RIR delegated extended files.
uniq_pfx_asn_dfz_v4.csv is the uniq filtered RISWhois dump file.

## API TODO

This sample output from the CLI:

```
(rotonda-store)> 193.0.10.0/24
Searching for prefix: 193.0.10.0/24
Found less-specific and exactly matching prefixes: [
    193.0.0.0/20 with Some(ExtPrefixRecord(Some(RirDelExtRecord { group_id: "4db2a3e7-2296-45c2-83dd-f9195bb76d14", rir: RipeNcc }), None)),
    193.0.10.0/23 with Some(ExtPrefixRecord(None, Some(RisWhoisRecord { origin_as: ["3333"] }))),
]
Found prefixes allocated to same organisation as prefix 3238002688/20:
[
    193.0.0.0/20 with Some(ExtPrefixRecord(Some(RirDelExtRecord { group_id: "4db2a3e7-2296-45c2-83dd-f9195bb76d14", rir: RipeNcc }), None)),
    193.0.16.0/21 with Some(ExtPrefixRecord(Some(RirDelExtRecord { group_id: "4db2a3e7-2296-45c2-83dd-f9195bb76d14", rir: RipeNcc }), None)),
    84.205.64.0/19 with Some(ExtPrefixRecord(Some(RirDelExtRecord { group_id: "4db2a3e7-2296-45c2-83dd-f9195bb76d14", rir: RipeNcc }), None)),
    93.175.144.0/21 with Some(ExtPrefixRecord(Some(RirDelExtRecord { group_id: "4db2a3e7-2296-45c2-83dd-f9195bb76d14", rir: RipeNcc }), None)),
    93.175.159.0/24 with Some(ExtPrefixRecord(Some(RirDelExtRecord { group_id: "4db2a3e7-2296-45c2-83dd-f9195bb76d14", rir: RipeNcc }), Some(RisWhoisRecord { origin_as: ["12859"] }))),
]
```

should turn into:

`/193.0.10.0/24/search`

```
{
  results: [
    { source: "bgp", result: null },
    { source: "rir_alloc", result: null }
  ],
  relations: [
    {
      type: "less-specific",
      source: "rir_alloc",
      prefix: "193.0.0.0/20",
      origin_asn: null,
      lmp: false,
      rir: "ripe"
    },
    {
      type: "less-specific",
      source: "bgp",
      prefix: "193.0.10.0/23",
      origin_asn: "AS3333",
      lmp: true,
      rir: "ripe"
    },
    {
      type: "same_org",
      source: "rir_alloc",
      prefix: "193.0.16.0/21",
      origin_asn: null,
      rir: "ripe"
    },
    {
      type: "same_org",
      source: "rir_alloc",
      prefix: "84.205.64.0/19",
      origin_asn: null,
      rir: "ripe"
    },
    {
      type: "same_org",
      source: "rir_alloc",
      prefix: "93.175.144.0/21",
      origin_asn: null,
      rir: "ripe"
    },
    {
      type: "same_org",
      source: "rir_alloc",
      prefix: "93.175.159.0/24",
      origin_asn: "12859",
      rir: "ripe"
    }
  ]
}
```

(yes, sorry, this is actual JS, should be JSON, but you get the gist)
