#!/bin/sh

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

echo concatenate all
cat delegated-afrinic-extended-latest.txt > delegated-all && \
cat delegated-apnic-extended-latest.txt >> delegated-all && \
cat delegated-arin-extended-latest.txt >> delegated-all && \
cat delegated-lacnic-extended-latest.txt >> delegated-all && \
cat delegated-ripencc-extended-latest.txt >> delegated-all

echo RISWHois IPv4
curl -o riswhoisdump-ipv4.gz http://www.ris.ripe.net/dumps/riswhoisdump.IPv4.gz

echo RISWhois IPv6
curl -o riswhoisdump-ipv6.gz http://www.ris.ripe.net/dumps/riswhoisdump.IPv6.gz

echo Refactor RISWhois
rg -e '(\d+)\t(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})/(\d+)\t(\d+)' -N --replace '$2,$3,$1' data/riswhoisdump.IPv4 > pfx_asn_dfz_v4.csv
rg -e '(\d+)\t([0-9abcdef:]+)/(\d{1,3})\t(\d+)$' -N --replace '$2,$3,$1' riswhoisdump.IPv6 > pfx_asn_dfz_v6.csv
cat pfx_asn_dfz_v4.csv > pfx_asn-all.csv && cat pfx_asn_dfz_v6.csv >> pfx_asn-all.csv
