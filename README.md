# ROTO API

This is an HTTP/JSON API that gets data from BGP announcements (through the RIPE Routing Information Collectors system) and the Delegated Extended Statistics files from the five RIRs.

It can be queried on prefix and ASNs.

Everything is hosted from RAM, so this should be ⚡fast.

A hosted version of this API is available via https://rest.bgp-api.net".


## Resources

| Name     | Type          |  Endpoint           |
| -------- | ------------- | ------------------- |
| Prefix   | Prefix        |  `/prefix`          |
| ASNs     | array[Asn]    |  `/asns`            |


`Prefix` is a String with a `/` in it and a unsigned 8-bit integer [0-255] after it. The part before the `/` must be parseable as a valid internet addres, either IPv4 or IPv6. IPv4 notation is full-quads only, so `[0-255].[0-255].[0-255].[0-255]`. IPv6 notations can have the well-known shortcuts, e.g. "2001::", etc.

`ASN` should be parseable to a unsigned 32-bit integer.

----
## Resource/Action: Prefix Search

Retrieve the longest-matching prefix for the requested prefix and retrieve prefixes related to that longest-matching prefix.
### Request 
```api/v1/prefix/<PREFIX>/search```

### Response

| fieldname | type          | description                    |
| --------- | ------------- | ------------------------------ |
| prefix    | Prefix        | the requested prefix           | 
| type      | MatchType     | the requested match type       |
| result    | Result        | the result of the match action |

#### MatchType

| variant        | description |
| -------------- | ----------- |
| longest-match  | the matched prefix is a longest-matching prefix of the requested prefix |
| exact-match    | the matched prefix is the same as the requested prefix |
| empty-match    | there was no longest matching (less specific) or exactly matching prefix for the requested prefix |

#### Result

| fieldname | type            |  description                                                                 |
| --------- | --------------- | ---------------------------------------------------------------------------- |
| prefix    | Prefix          |  the matched prefix                                                          |
| type      | MatchType       |  the type of match that produced the matched prefix                          |
| meta      | MetaObject      |  metadata associated with the matched prefix                                 |
| relations | Array(Relation) |  array of related prefixes collected by relation type                        |

#### Relation

| fieldname | type          |                                                                              |
| --------- | ------------- | ---------------------------------------------------------------------------- |
| prefix    | Prefix        |  the related prefix                                                          |
| type      | RelationType  |  the type of relation between the matched prefix and these related prefixes  |
| meta      | Meta          |  metadata associated with the related prefix                                 |

#### Meta

| fieldname     | type    | description                                                           |
| ------------- | ------- | --------------------------------------------------------------------- |
| sourceType    | Enum    |  the source that contributed this prefix, one of "rir-alloc" or "bgp" |
| sourceID      | String  |  a string that identifies the source                                  |
| originASNs    | Array[ASN] *if sourceType=="bgp"* | The BGP origin ASNs for this prefix         |

---
## Resource/Action: ASNs Search

Retrieve the prefixes that are originated by one of the requested ASNs in BGP.
### Request

```/api/v1/asns/<ASN>[,<ASN>].../search```

#### Response

| fieldname | type          | description                                          |
| --------- | ------------- | ---------------------------------------------------- |
| asns      | Array(ASN)    | the requested ASNs to find prefixes for              | 
| type      | SearchType    | the requested search type (`by-asns` only right now) |
| meta      | MetaObject    | Metadata for the requested ASNs (`null` right now)   |
| result    | ResultObject  | the result of the search action                      |

MetaObject and ResultObject: see above
### Resource Status

Retrieve the current status of this Roto API instance.
#### Request

```api/v1/status```


#### Response

| fieldname | type          | description                                          |
| --------- | ------------- | ---------------------------------------------------- |
| version   | String        | Version of this API instance                         | 
| sources   | Array(Source) | Sources available in this API instance               |

### Source Resource

| fieldname   | type       | description                                       |
| ----------- | ---------- | ------------------------------------------------- |
| type        | SourceType | Type of this source                               |
| id          | String     | Identifying string of the source                  |
| serial      | Integer    | Serial Number (timestamp of downloaded file)      |
| lastUpdated | DateTime   | Last Modified Header of download                  |


### SourceType

| name        | contributors                              | data description                   |
| ----------- | ----------------------------------------- | ---------------------------------- |
| `bgp`       | `riswhois`                                | BGP origin ASNs from announcements |
| `rir-alloc` | `afrinic`,`apnic`, `arin`,`lacnic`,`ripe` | Allocation by RIRs                 |

### RelationType

| name           | source    | Relation Resource Types |
| -------------- | --------- | ----------------------- |
| less-specific  | all       | prefix                  |
| more-specific  | all       | prefix                  |
| same-org       | rir-alloc | prefix                  |
| bgp-origin-as  | bgp       | prefix                  |

### Result

Array(Result)

| fieldname  | content type                      |      |
| ---------- | --------------------------------- | ---- |
| sourceType | Type of Source                    |      |
| sourceID   | ID of Source                      |      |
| originASNs | Array[ASN] *if sourceType=="bgp"* |      |

---
## Endpoint Description

#### ```/<RESOURCE/<ID>/<VERB>```

Note that Query Parameters are not supported yet.

ex.:

query

#### ```https://rest.bgp-api.net/api/v1/asns/2113211/search```

### response

```json
{
   "type": "by-asns",
   "asns": [
      "AS211321"
   ],
   "meta": null,
   "result": {
      "relations": [
         {
            "type": "bgp-origin-asn",
            "members": [
               {
                  "prefix": "151.216.0.0/24",
                  "meta": [
                     {
                        "sourceType": "bgp",
                        "sourceID": "riswhois",
                        "originASNs": [
                           "AS211321"
                        ]
                     }
                  ]
               },
               {
                  "prefix": "185.49.142.0/24",
                  "meta": [
                     {
                        "sourceType": "bgp",
                        "sourceID": "riswhois",
                        "originASNs": [
                           "AS211321"
                        ]
                     }
                  ]
               },
               {
                  "prefix": "2001:7fc::/48",
                  "meta": [
                     {
                        "sourceType": "bgp",
                        "sourceID": "riswhois",
                        "originASNs": [
                           "AS211321"
                        ]
                     }
                  ]
               },
               {
                  "prefix": "2a04:b904::/48",
                  "meta": [
                     {
                        "sourceType": "bgp",
                        "sourceID": "riswhois",
                        "originASNs": [
                           "AS211321"
                        ]
                     }
                  ]
               }
            ]
         }
      ]
   }
}
```

```https://rest.bgp-api.net/api/v1/prefix/193.0.10.0/24/search```

```json
{
   "type": "longest-match",
   "prefix": "193.0.10.0/24",
   "result": {
      "prefix": "193.0.10.0/23",
      "type": "longest-match",
      "meta": [
         {
            "sourceType": "bgp",
            "sourceID": "riswhois",
            "originASNs": [
               "AS3333"
            ],
            "type": "less-specific"
         }
      ],
      "relations": [
         {
            "type": "same-org",
            "members": [
               {
                  "prefix": "193.0.0.0/20",
                  "type": "same-org",
                  "meta": [
                     {
                        "sourceType": "rir-alloc",
                        "sourceID": "ripe"
                     }
                  ]
               },
               {
                  "prefix": "193.0.16.0/21",
                  "type": "same-org",
                  "meta": [
                     {
                        "sourceType": "rir-alloc",
                        "sourceID": "ripe"
                     }
                  ]
               },
               {
                  "prefix": "84.205.64.0/19",
                  "type": "same-org",
                  "meta": [
                     {
                        "sourceType": "rir-alloc",
                        "sourceID": "ripe"
                     }
                  ]
               },
               {
                  "prefix": "93.175.144.0/21",
                  "type": "same-org",
                  "meta": [
                     {
                        "sourceType": "rir-alloc",
                        "sourceID": "ripe"
                     }
                  ]
               },
               {
                  "prefix": "93.175.159.0/24",
                  "type": "same-org",
                  "meta": [
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
                  "meta": [
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
                  "meta": [
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
                  "meta": [
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
                  "meta": [
                     {
                        "sourceType": "rir-alloc",
                        "sourceID": "ripe"
                     }
                  ]
               },
               {
                  "prefix": "2001:7fd::/32",
                  "type": "same-org",
                  "meta": [
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
               }
            ]
         },
         {
            "type": "less-specific",
            "members": [
               {
                  "prefix": "193.0.0.0/20",
                  "type": "less-specific",
                  "meta": [
                     {
                        "sourceType": "rir-alloc",
                        "sourceID": "ripe"
                     }
                  ]
               }
            ]
         },
         {
            "type": "more-specific",
            "members": [

            ]
         }
      ]
   }
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

Documentation: https://ris.ripe.net/docs/27_riswhois.html#riswhois-dumps

- http://www.ris.ripe.net/dumps/riswhoisdump.IPv4.gz
- http://www.ris.ripe.net/dumps/riswhoisdump.IPv6.gz

## Sources

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

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install 1.51
rustup default 1.51
```
The download scripts depend on `ripgrep` to be available in the
dir `/home/roto/.cargo/bin/`. Change the path to the `rg` (ripgrep)
command to either you local `.cargo` directory, or you can install
rg globally and get rid of the path.

```
cargo install ripgrep
```

Clone ths repository

```
git clone git@github.com:NLnetLabs/roto-api.git
```

Move to the repo root. You will have to change the actual
path to `rg` probably in the two scripts that are run:

```
cd roto-api
ROTO_API_PEER=<SOME_HOSTNAME> ./scripts/download-del-ext
ROTO_API_PEER=<SOME_HOSTNAME> ./scripts/download-riswhois
```

Start the API service

```
./scripts/start-roto
```
