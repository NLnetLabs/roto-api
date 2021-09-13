
# DEPRECATED DEPRECATED
# THIS DOCUMENTATION NEEDS TO BE UPDATED TO THE CURRENT STRUCTURE.

# BGP+RPKI+ALLOC API

This is an HTTP/JSON API that gets data from BGP announcements (through the RIPE Routing Information Collectors system) and the Delegated Extended Statistics files from the five RIRs.

It can be queried on prefix(es) for now.
## Resources

| Name     | Type          |      |
| -------- | ------------- | ---- |
| Prefix   | object        |      |
| Sources  | array[source] |      |

### Prefix Search Response

| fieldname | type          |      |
| --------- | ------------- | ---- |
| type      | searchType    |      |
| prefix    | Prefix        |      |
| results   | Array(Result) |      |
| relations | Array(Result) |      |

### Sources Reponse

| fieldname   | type                                         |
| ----------- | -------------------------------------------- |
| type        | type of Source                               |
| id          | ID of source                                 |
| serial      | Serial Number (timestamp of downloaded file) |
| lastUpdated | Last Modified Header of download             |


### Sources

| name        | data description                   | data format                                                  |
| :---------- | ---------------------------------- | ------------------------------------------------------------ |
| `bgp`       | BGP origin ASNs from announcements | originASNs(Array[ASN])                                       |
| `rir-alloc` | Allocation by RIRs                 | Enum((source(source), prefix(prefix), rir(string), relation(relation), relation_resource(resource) \| (source(source), asn(asn), rir(string), relation(relation), relation_resource(resource))) |

### Relation

| fieldname | type          |      |
| --------- | ------------- | ---- |
| prefix    | Prefix        |      |
| type      | RelationType  |      |
| results   | Array(Result) |      |

### RelationType

| name           | source    | Relation Resource Types |
| -------------- | --------- | ----------------------- |
| less-specific  | all       | prefix                  |
| more-specific  | all       | prefix                  |
| same-org       | rir-alloc | prefix, asn             |
| same-origin-as | bgp       | prefix                  |

### Result

Array(Result)

| fieldname  | content type                      |      |
| ---------- | --------------------------------- | ---- |
| sourceType | Type of Source                    |      |
| sourceID   | ID of Source                      |      |
| originASNs | Array[ASN] *if sourceType=="bgp"* |      |

### SearchType

| name        | source | Resource Type |
| ----------- | ------ | ------------- |
| exact-match | all    | prefix        |


## Endpoint Description

#### ```/<RESOURCE/<ID>/<VERB>/?[include=relations]```

ex.:

query

#### ```/prefix/193.0.10.0/24/search```

### response

```json
{
   "type": "exact-match",
   "prefix": "193.0.10.0/24",
   "results": [],
   "relations": [
      {
         "prefix": "193.0.0.0/20",
         "type": "same-org",
         "results": [
            {
               "sourceType": "rir-alloc",
               "sourceID": "ripe"
            }
         ]
      },
      {
         "prefix": "193.0.16.0/21",
         "type": "same-org",
         "results": [
            {
               "sourceType": "rir-alloc",
               "sourceID": "ripe"
            }
         ]
      },
      {
         "prefix": "84.205.64.0/19",
         "type": "same-org",
         "results": [
            {
               "sourceType": "rir-alloc",
               "sourceID": "ripe"
            }
         ]
      },
      {
         "prefix": "93.175.144.0/21",
         "type": "same-org",
         "results": [
            {
               "sourceType": "rir-alloc",
               "sourceID": "ripe"
            }
         ]
      },
      {
         "prefix": "93.175.159.0/24",
         "type": "same-org",
         "results": [
            {
               "sourceType": "rir-alloc",
               "sourceID": "ripe"
            },
            {
               "sourceType": "bgp",
               "sourceID": "riswhois",
               "originASNs": [
                  "AS12859"
               ]
            }
         ]
      },
      {
         "prefix": "2001:67c:e0::/48",
         "type": "same-org",
         "results": [
            {
               "sourceType": "rir-alloc",
               "sourceID": "ripe"
            },
            {
               "sourceType": "bgp",
               "sourceID": "riswhois",
               "originASNs": [
                  "AS197000"
               ]
            }
         ]
      },
      {
         "prefix": "2001:67c:2e8::/48",
         "type": "same-org",
         "results": [
            {
               "sourceType": "rir-alloc",
               "sourceID": "ripe"
            },
            {
               "sourceType": "bgp",
               "sourceID": "riswhois",
               "originASNs": [
                  "AS3333"
               ]
            }
         ]
      },
      {
         "prefix": "2001:67c:2d7c::/48",
         "type": "same-org",
         "results": [
            {
               "sourceType": "rir-alloc",
               "sourceID": "ripe"
            },
            {
               "sourceType": "bgp",
               "sourceID": "riswhois",
               "originASNs": [
                  "AS12859"
               ]
            }
         ]
      },
      {
         "prefix": "2001:7fb::/32",
         "type": "same-org",
         "results": [
            {
               "sourceType": "rir-alloc",
               "sourceID": "ripe"
            }
         ]
      },
      {
         "prefix": "2001:7fd::/32",
         "type": "same-org",
         "results": [
            {
               "sourceType": "rir-alloc",
               "sourceID": "ripe"
            },
            {
               "sourceType": "bgp",
               "sourceID": "riswhois",
               "originASNs": [
                  "AS25152"
               ]
            }
         ]
      },
      {
         "prefix": "193.0.0.0/20",
         "type": "less-specific",
         "results": [
            {
               "sourceType": "rir-alloc",
               "sourceID": "ripe"
            }
         ]
      },
      {
         "prefix": "193.0.10.0/23",
         "type": "less-specific",
         "results": [
            {
               "sourceType": "bgp",
               "sourceID": "riswhois",
               "originASNs": [
                  "AS3333"
               ]
            }
         ]
      }
   ]
}
```

# Source Data Sets

## delegated extended

```bash
echo delegated-afrinic-extended-latest
curl -o delegated-afrinic-extended-latest.txt ftp://ftp.afrinic.net/pub/stats/afrinic/delegated-afrinic-extended-latest

echo delegated-apnic-extended-latest
curl -o delegated-apnic-extended-latest.txt ftp://ftp.apnic.net/pub/stats/apnic/delegated-apnic-extended-latest

echo delegated-arin-extended-latest
curl -o delegated-arin-extended-latest.txt ftp://ftp.arin.net/pub/stats/arin/delegated-arin-extended-latest

echo delegated-lacnic-extended-latest
curl -o delegated-lacnic-extended-latest.txt ftp://ftp.lacnic.net/pub/stats/lacnic/delegated-lacnic-extended-latest

echo delegated-ripencc-extended-latest
curl -o delegated-ripencc-extended-latest.txt ftp://ftp.ripe.net/pub/stats/ripencc/delegated-ripencc-extended-latest
```

## RisWhois

http://www.ris.ripe.net/dumps/riswhoisdump.IPv4.gz
http://www.ris.ripe.net/dumps/riswhoisdump.IPv6.gz
# Sources

### delegated extended

  - https://ftp.afrinic.net/pub/stats/afrinic/delegated-afrinic-extended-latest
  - https://ftp.apnic.net/pub/stats/apnic/delegated-apnic-extended-latest
  - https://ftp.arin.net/pub/stats/arin/delegated-arin-extended-latest
  - https://ftp.lacnic.net/pub/stats/lacnic/delegated-lacnic-extended-latest
  - https://ftp.ripe.net/pub/stats/ripencc/delegated-ripencc-extended-latest

### RisWhois
#### IPv4

https://www.ris.ripe.net/dumps/riswhoisdump.IPv4.gz

#### IPv6

https://www.ris.ripe.net/dumps/riswhoisdump.IPv6.gz


# Installation

# Prepare installation

```
apt install build-essential
```

# Install rust and other dependencies

Currently this API only works with Rust verson <= 1.51:

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install 1.51
rustup default 1.51
```
The download scripts depend on `ripgrep` to be available in the
dir `/home/roto/.cargo/bin/`. Change te path to the `rg` (ripgrep)
command to either you local `.cargo` directory, or you can install
rg globally and get rid of the path.

```
cargo install ripgrep
```

Clone ths repository

```
git clone git@github.com:NLnetLabs/ris_alloc_api.git
```

Move to the repo root. You will have to change the actual
path to `rg` probably in the two scripts that are run:

```
cd ris_alloc_api
ROTO_API_PEER=<SOME_HOSTNAME> ./scripts/download-del-ext
ROTO_API_PEER=<SOME_HOSTNAME> ./scripts/download-riswhois
```

Start the API service

```
./scripts/start-roto
```
