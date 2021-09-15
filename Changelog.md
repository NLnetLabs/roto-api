# Change Log

## 0.2.0

Released 2021-09-15

New

* Add `/asns` endpoint for query by BGP origin ASNs.

Breaking Changes

* `relations` holds an Array of relation types ("less-specific", "more-specific", "same-org") instead
  of a flat list with `type` fields.
* Removed `type` field on `prefix` resources.
* `sources` resource moved into `/status` endpoint (harmonized with Routinator).
* `/sources` endpoint removed.

## 0.1.0

Released 2021-07-08

New

* `/prefix` resource endpoint
* `/status` endpoint (with sources)
* `/` endpoint with description of endpoints