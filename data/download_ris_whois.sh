#!/bin/sh

echo RISWHois IPv4
curl -o riswhoisdump-ipv4.gz http://www.ris.ripe.net/dumps/riswhoisdump.IPv4.gz
gunzip riswhoisdump-ipv4.gz

echo RISWhois IPv6
curl -o riswhoisdump-ipv6.gz http://www.ris.ripe.net/dumps/riswhoisdump.IPv6.gz
gunzip riswhoisdump-ipv6.gz

echo Refactor RISWhois
rg -e '(\d+)\t(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})/(\d+)\t(\d+)' -N --replace '$2,$3,$1' riswhoisdump-ipv4 > pfx_asn_dfz_v4.csv
rg -e '(\d+)\t([0-9abcdef:]+)/(\d{1,3})\t(\d+)$' -N --replace '$2,$3,$1' riswhoisdump-ipv6 > pfx_asn_dfz_v6.csv
cat pfx_asn_dfz_v4.csv > pfx_asn-all.csv && cat pfx_asn_dfz_v6.csv >> pfx_asn-all.csv
