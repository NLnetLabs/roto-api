use chrono::DateTime;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, StatusCode};
use roto_api::{
    version, Addr, Asn, JsonBuilder, Prefix, SearchByAsnOptions, SearchType, Store,
    TimeStamp, TimeStamps,
};
use rotonda_store::{MatchOptions, MatchType};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::str::FromStr;
use std::{env, process, thread};
use tokio::sync::{mpsc, oneshot};

const CURRENT_API_VERSION: &str = "v1";

//------------ process_tasks -------------------------------------------------

fn process_tasks(store: Store, mut queue: mpsc::Receiver<(Task, oneshot::Sender<Response<Body>>)>) {
    while let Some((task, tx)) = queue.blocking_recv() {
        let res = match task {
            Task::PrefixMatch(MatchPrefixRequest {
                prefix,
                match_options,
            }) => match_prefix_output(&store, prefix, match_options),
            Task::ByAsnSearch(SearchByAsnRequest {
                asns,
                search_options,
            }) => search_by_bgp_asn_output(&store, asns, search_options),
        };

        let _err = tx.send(
            Response::builder()
                .status(hyper::StatusCode::OK)
                .header(hyper::header::CONTENT_TYPE, "application/json")
                .header(hyper::header::ACCESS_CONTROL_ALLOW_METHODS, "GET, OPTIONS")
                .header(hyper::header::ACCESS_CONTROL_ALLOW_HEADERS,"DNT,User-Agent,X-Requested-With,If-Modified-Since,Cache-Control,Content-Type,Range")
                .header(hyper::header::ACCESS_CONTROL_EXPOSE_HEADERS,"Content-Length,Content-Range")
                .header(hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .body(hyper::Body::from(res))
                .unwrap()
        );
    }
}

pub fn match_prefix_output(store: &Store, prefix: Prefix, match_options: MatchOptions) -> String {
    let recs = match prefix.addr {
        Addr::V4(_addr) => store.match_longest_prefix::<u32>(prefix, &match_options),
        Addr::V6(_addr) => store.match_longest_prefix::<u128>(prefix, &match_options),
    };
    let query_result = recs.clone();

    JsonBuilder::build(|builder| {
        builder.member_str("type", match_options.match_type);
        builder.member_str("prefix", prefix);
        builder.member_object("result", |builder| {
            if let Some(pfx) = query_result.prefix {
                builder.member_str("prefix", pfx);
                if let Some(ext_rec) = query_result.prefix_meta {
                    builder.member_array("meta", |builder| {
                        // rir delegated extended records
                        match &ext_rec.0 {
                            Some(rir_del_ext_r) => {
                                builder.array_object(|builder| {
                                    builder.member_str("sourceType", "rir-alloc");
                                    builder.member_str("sourceID", rir_del_ext_r.rir.to_json_id());
                                });
                            }
                            None => {}
                        }
                        // rishwhois records
                        match &ext_rec.1 {
                            Some(ris_whois_r) => {
                                builder.array_object(|builder| {
                                    builder.member_str("sourceType", "bgp");
                                    builder.member_str("sourceID", "riswhois");
                                    builder.member_array("originASNs", |builder| {
                                        for asn in ris_whois_r.origin_asns.0.iter() {
                                            builder.array_str(asn)
                                        }
                                    });
                                    builder.member_str(
                                        "type",
                                        if prefix.len == pfx.len {
                                            "exact-match"
                                        } else {
                                            "less-specific"
                                        },
                                    )
                                });
                            }
                            None => {}
                        }
                    });
                };
            } else {
                builder.member_raw("prefix", "null");
                builder.member_str("type", &recs.match_type);
                builder.member_raw("meta", "[]");
                builder.member_raw("members", "[]");
            }

            // See whether the result_prefix has a DelExtRecord, if it does
            // then we're using that, if it doesn't we can see if there's a
            // less-specific prefix that has one.
            // The vecs in a RecordSet are ordered from least to most specific,
            // hence the reverse. That way we'll get the longest-matching prefix
            // with a RirDelExtRecord.
            // The resulting prefix is then used to lookup all the related prefixes.

            if let Some(rec) = query_result.prefix_meta {
                let rev = recs.less_specifics.reverse();

                let lmp_rel_rec = if rec.0.is_some() {
                    rec.0.as_ref()
                } else {
                    rev.iter().find_map(|(_p, r)| match r {
                        Some(rec) => rec.0.as_ref(),
                        None => None,
                    })
                };

                if let Some(lmp_rel_rec) = lmp_rel_rec {
                    println!("lmp rec {:?}", lmp_rel_rec);
                    let rel_rec = store.get_related_prefixes(lmp_rel_rec);
                    builder.member_array("relations", |builder| {
                        builder.array_object(|builder| {
                            builder.member_str("type", "same-org");
                            builder.member_array("members", |builder| {
                                for (pfx, value) in rel_rec.iter() {
                                    builder.array_object(|builder| {
                                        builder.member_str("prefix", pfx);
                                        builder.member_array("meta", |builder| {
                                            if let Some(ext_rec) = value {
                                                match &ext_rec.0 {
                                                    Some(rir_del_ext_r) => {
                                                        builder.array_object(|builder| {
                                                            builder.member_str(
                                                                "sourceType",
                                                                "rir-alloc",
                                                            );
                                                            builder.member_str(
                                                                "sourceID",
                                                                rir_del_ext_r.rir.to_json_id(),
                                                            );
                                                        });
                                                    }
                                                    None => {}
                                                }
                                                match &ext_rec.1 {
                                                    Some(ris_whois_r) => {
                                                        builder.array_object(|builder| {
                                                            builder.member_str("sourceType", "bgp");
                                                            builder
                                                                .member_str("sourceID", "riswhois");
                                                            builder.member_array(
                                                                "originASNs",
                                                                |builder| {
                                                                    for asn in ris_whois_r
                                                                        .origin_asns
                                                                        .0
                                                                        .iter()
                                                                    {
                                                                        builder.array_str(asn)
                                                                    }
                                                                },
                                                            );
                                                        });
                                                    }
                                                    None => {}
                                                }
                                            }
                                        })
                                    });
                                }
                            });
                        });
                        builder.array_object(|builder| {
                            builder.member_str("type", "less-specific");
                            builder.member_array("members", |builder| {
                                for (pfx, value) in query_result.less_specifics.iter() {
                                    builder.array_object(|builder| {
                                        builder.member_str("prefix", pfx);
                                        builder.member_array("meta", |builder| {
                                            if let Some(ext_rec) = value {
                                                match &ext_rec.0 {
                                                    Some(rir_del_ext_r) => {
                                                        builder.array_object(|builder| {
                                                            builder.member_str(
                                                                "sourceType",
                                                                "rir-alloc",
                                                            );
                                                            builder.member_str(
                                                                "sourceID",
                                                                rir_del_ext_r.rir.to_json_id(),
                                                            );
                                                        });
                                                    }
                                                    None => {}
                                                }
                                                match &ext_rec.1 {
                                                    Some(ris_whois_r) => {
                                                        builder.array_object(|builder| {
                                                            builder.member_str("sourceType", "bgp");
                                                            builder
                                                                .member_str("sourceID", "riswhois");
                                                            builder.member_array(
                                                                "originASNs",
                                                                |builder| {
                                                                    for asn in ris_whois_r
                                                                        .origin_asns
                                                                        .0
                                                                        .iter()
                                                                    {
                                                                        builder.array_str(asn)
                                                                    }
                                                                },
                                                            );
                                                        });
                                                    }
                                                    None => {}
                                                }
                                            }
                                        })
                                    });
                                }
                            });
                        });
                        builder.array_object(|builder| {
                            builder.member_str("type", "more-specific");
                            builder.member_array("members", |builder| {
                                for (pfx, value) in query_result.more_specifics.iter() {
                                    builder.array_object(|builder| {
                                        builder.member_str("prefix", pfx);
                                        builder.member_array("meta", |builder| {
                                            if let Some(ext_rec) = value {
                                                match &ext_rec.0 {
                                                    Some(rir_del_ext_r) => {
                                                        builder.array_object(|builder| {
                                                            builder.member_str(
                                                                "sourceType",
                                                                "rir-alloc",
                                                            );
                                                            builder.member_str(
                                                                "sourceID",
                                                                rir_del_ext_r.rir.to_json_id(),
                                                            );
                                                        });
                                                    }
                                                    None => {}
                                                }
                                                match &ext_rec.1 {
                                                    Some(ris_whois_r) => {
                                                        builder.array_object(|builder| {
                                                            builder.member_str("sourceType", "bgp");
                                                            builder
                                                                .member_str("sourceID", "riswhois");
                                                            builder.member_array(
                                                                "originASNs",
                                                                |builder| {
                                                                    for asn in ris_whois_r
                                                                        .origin_asns
                                                                        .0
                                                                        .iter()
                                                                    {
                                                                        builder.array_str(asn)
                                                                    }
                                                                },
                                                            );
                                                        });
                                                    }
                                                    None => {}
                                                }
                                            }
                                        })
                                    });
                                }
                            });
                        })
                    });
                }
            }
        });
    })
}

pub fn search_by_bgp_asn_output(
    store: &Store,
    asns: Vec<Asn>,
    search_options: SearchByAsnOptions,
) -> String {
    let recs = store.get_prefixes_for_bgp_asn(&asns, &search_options);
    // let query_result = recs.clone();

    JsonBuilder::build(|builder| {
        builder.member_str("type", &search_options.search_type);
        builder.member_array("asns", |builder| {
            for asn in asns.into_iter() {
                builder.array_str(asn);
            }
        });
        builder.member_raw("meta", "null");
        builder.member_object("result", |builder| {
            builder.member_array("relations", |builder| {
                builder.array_object(|builder| {
                    // if let Some(asn) = query_result.asns.get(0) {
                    // builder.member_array("asns", |builder| {
                    //     builder.array_str(asn);
                    // });
                    builder.member_str("type", "bgp-origin-asn");
                    builder.member_array("members", |builder| {
                        for (pfx, value) in recs.prefixes.iter() {
                            builder.array_object(|builder| {
                                builder.member_str("prefix", pfx);
                                builder.member_array("meta", |builder| {
                                    if let Some(ext_rec) = value {
                                        match &ext_rec.0 {
                                            Some(rir_del_ext_r) => {
                                                builder.array_object(|builder| {
                                                    builder.member_str("sourceType", "rir-alloc");
                                                    builder.member_str(
                                                        "sourceID",
                                                        rir_del_ext_r.rir.to_json_id(),
                                                    );
                                                });
                                            }
                                            None => {}
                                        }
                                        match &ext_rec.1 {
                                            Some(ris_whois_r) => {
                                                builder.array_object(|builder| {
                                                    builder.member_str("sourceType", "bgp");
                                                    builder.member_str("sourceID", "riswhois");
                                                    builder.member_array("originASNs", |builder| {
                                                        for asn in ris_whois_r.origin_asns.0.iter()
                                                        {
                                                            builder.array_str(asn)
                                                        }
                                                    });
                                                });
                                            }
                                            None => {}
                                        }
                                    }
                                })
                            });
                        }
                    });
                    // } else {
                    //     builder.member_raw("asn", "null");
                    //     builder.member_raw("relations", "null");
                    // }
                });
            });
        });
    })
}

pub fn import_timestamps() -> Result<TimeStamps, Box<dyn std::error::Error>> {
    const TIMESTAMPS_FILE_PREFIX: &str = ".timestamps.json";
    let mut timestamps: TimeStamps = Default::default();
    for dataset in &["del_ext", "riswhois"] {
        let f_name = format!("./data/{}{}", dataset, TIMESTAMPS_FILE_PREFIX);
        let ts_file = std::fs::File::open(f_name)?;
        let mut rdr = csv::ReaderBuilder::new()
            .delimiter(b',')
            .flexible(true)
            .trim(csv::Trim::Headers)
            .from_reader(ts_file);

        for record in rdr.records() {
            let record = record?;

            timestamps.push(TimeStamp(
                record[0].into(),
                record[1].parse::<u64>().unwrap(),
                DateTime::parse_from_rfc2822(&record[2]).unwrap(),
            ))?;
        }
    }
    Ok(timestamps)
}

//------------ process_request -----------------------------------------------

struct MatchPrefixRequest {
    prefix: Prefix,
    match_options: MatchOptions,
}

struct SearchByAsnRequest {
    asns: Vec<Asn>,
    search_options: SearchByAsnOptions,
}

enum Task {
    PrefixMatch(MatchPrefixRequest),
    ByAsnSearch(SearchByAsnRequest),
}

async fn process_request(
    req: Request<Body>,
    timestamps: TimeStamps,
    tx: mpsc::Sender<(Task, oneshot::Sender<Response<Body>>)>,
) -> Result<Response<Body>, Infallible> {
    let match_options = MatchOptions {
        match_type: MatchType::LongestMatch,
        include_less_specifics: true,
        include_more_specifics: true,
    };

    let mut url = req.uri().path().split('/');
    println!("{:?}", req.uri().path());

    let _slash = url.next();

    // We're accepting both "/v1" and "/api/v1", as to accomodate
    // both single-domain API/UI and separate domain hosting.
    let api_v1 = url.next();

    if api_v1.as_ref() != Some(&CURRENT_API_VERSION) && api_v1.as_ref() != Some(&"api") {
        return not_found(Some(
            "Cannot parse query. Request url should start with `[api]/v1`".to_string(),
        ));
    }

    if api_v1.as_ref() == Some(&"api") && url.next().as_ref() != Some(&"v1") {
        return not_found(Some(
            "Cannot parse query. Request url should start with `[api]/v1`".to_string(),
        ));
    }

    // ---------------------- Resources Endpoints ----------------------------------

    let resource = url.next();
    match resource {
        // If a call to /[api]/v1 is made with any further stuff, we'll
        // return a short description of the API.
        None | Some("") => {
            let uri = req.uri();
            let host = if let Some(h) = req.headers().get("Host") {
                h.to_str().unwrap()
            } else {
                ""
            };
            Ok(ok_cors_response(JsonBuilder::build(|builder| {
                builder.member_str("version", format!("roto-api/{}", version()));
                builder.member_array("resources", |builder| {
                    builder.array_object(|builder| {
                        builder.member_str("id", "prefix");
                        builder
                            .member_str("description", "Prefix with enriched data from data sources");
                        builder.member_str(
                            "syntax",
                            "/api/v1/prefix/<IP_ADDRESS>/<PREFIX_LENGTH>/search[?include[relations]=[bgp|rir-alloc]]",
                        );
                        builder.member_str("uri", format!("https://{}{}prefix/", host, uri));
                    });
                    builder.array_object(|builder| {
                        builder.member_str("id", "status");
                        builder.member_str("description", "Status of this API");
                        builder.member_raw("syntax", "null");
                        builder.member_str("uri", format!("https://{}{}status", host, uri))
                    });
                });
            })))
        }
        Some("status") => Ok(ok_cors_response(JsonBuilder::build(|builder| {
            builder.member_str("version", format!("roto-api/{}", version()));
            timestamps.to_json_builder(builder);
        }))),
        Some("prefix") => {
            let addr = match url.next().and_then(|s| {
                println!("s {}", s);
                Addr::from_str(s).ok()
            }) {
                Some(addr) => addr,
                None => {
                    println!("address parse failure");
                    return not_found(Some("Cannot parse address part of the prefix. Prefix should be in format <IP_ADDRESS>/<LENGTH>".to_string()));
                }
            };
            let len = match url.next().and_then(|s| u8::from_str(s).ok()) {
                Some(len) => len,
                None => {
                    println!("length parse failure");
                    return not_found(Some("Cannot parse length part of the prefix. Prefix should be in format <IP_ADDRESS>/<LENGTH>".to_string()));
                }
            };
            if url.next().as_ref() != Some(&"search") {
                println!("action parse failure");
                return not_found(Some(
                    "Cannot parse action part of the prefix. Current actions are: `search`."
                        .to_string(),
                ));
            }
            if url.next().is_some() {
                println!("trailing stuff failure");
                return not_found(Some(
                    "Found trailing statements beyon the action part. Please remove those."
                        .to_string(),
                ));
            }
            println!("--- end request ---");

            let (resp_tx, resp_rx) = oneshot::channel();
            if tx
                .send((
                    Task::PrefixMatch(MatchPrefixRequest {
                        prefix: Prefix::new(addr, len),
                        match_options,
                    }),
                    resp_tx,
                ))
                .await
                .is_err()
            {
                return Ok(internal_server_error());
            }
            Ok(resp_rx.await.unwrap_or_else(|_| internal_server_error()))
        }
        Some("asn") => {
            let asns = match url.next().and_then(|s| {
                println!("s {}", s);
                let mut asns = vec![];
                for asn in s.split(',') {
                    if let Ok(asn) = Asn::from_str(asn) {
                        asns.push(asn);
                    } else {
                        return None;
                    }
                }
                Some(asns)
            }) {
                Some(asns) => asns,
                None => {
                    println!("ASNs parse failure");
                    return not_found(Some(
                        "Cannot parse ASN. The ASNs should be comma-separated list of integers 0 < asn < 4,294,967,296"
                            .to_string(),
                    ));
                }
            };
            if url.next().as_ref() != Some(&"search") {
                println!("action parse failure");
                return not_found(Some(
                    "Cannot parse action part of the ASNs. Current actions are: `search`."
                        .to_string(),
                ));
            }
            if url.next().is_some() {
                println!("trailing stuff failure");
                return not_found(Some(
                    "Found trailing statements beyon the action part. Please remove those."
                        .to_string(),
                ));
            }

            let (resp_tx, resp_rx) = oneshot::channel();
            if tx
                .send((
                    Task::ByAsnSearch(SearchByAsnRequest {
                        asns,
                        search_options: SearchByAsnOptions {
                            search_type: SearchType::PrefixesByBgpAsn,
                        },
                    }),
                    resp_tx,
                ))
                .await
                .is_err()
            {
                return Ok(internal_server_error());
            }
            Ok(resp_rx.await.unwrap_or_else(|_| internal_server_error()))
        }
        // 404 Catchall
        _ => not_found(Some(
            "Cannot parse resource. Current resources are: `prefix`,`status`, `asn`".to_string(),
        )),
    }
}

fn not_found(description: Option<String>) -> Result<Response<Body>, Infallible> {
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .header(hyper::header::ACCESS_CONTROL_ALLOW_METHODS, "GET, OPTIONS")
        .header(
            hyper::header::ACCESS_CONTROL_ALLOW_HEADERS,
            "DNT,User-Agent,X-Requested-With,If-Modified-Since,Cache-Control,Content-Type,Range",
        )
        .header(
            hyper::header::ACCESS_CONTROL_EXPOSE_HEADERS,
            "Content-Length,Content-Range",
        )
        .header(hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(Body::from(format!(
            "{{\"results\": null, \"error\": true, \"error_msg\": \"{}\"}}",
            description.unwrap_or_else(|| "cannot parse query".to_string())
        )))
        .unwrap())
}

fn internal_server_error() -> Response<Body> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .header(hyper::header::ACCESS_CONTROL_ALLOW_METHODS, "GET, OPTIONS")
        .header(
            hyper::header::ACCESS_CONTROL_ALLOW_HEADERS,
            "DNT,User-Agent,X-Requested-With,If-Modified-Since,Cache-Control,Content-Type,Range",
        )
        .header(
            hyper::header::ACCESS_CONTROL_EXPOSE_HEADERS,
            "Content-Length,Content-Range",
        )
        .header(hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(Body::empty())
        .unwrap()
}

fn ok_cors_response(body: String) -> Response<Body> {
    Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .header(hyper::header::ACCESS_CONTROL_ALLOW_METHODS, "GET, OPTIONS")
        .header(
            hyper::header::ACCESS_CONTROL_ALLOW_HEADERS,
            "DNT,User-Agent,X-Requested-With,If-Modified-Since,Cache-Control,Content-Type,Range",
        )
        .header(
            hyper::header::ACCESS_CONTROL_EXPOSE_HEADERS,
            "Content-Length,Content-Range",
        )
        .header(hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(Body::from(body))
        .unwrap()
}

#[tokio::main]
async fn main() {
    let mut args = env::args();
    let cmd = match args.next() {
        Some(cmd) => cmd,
        None => {
            eprintln!("Fatal: failed to understand command line.");
            process::exit(1);
        }
    };
    let listen = match args.next().and_then(|s| SocketAddr::from_str(&s).ok()) {
        Some(addr) => addr,
        None => {
            eprintln!(
                "Usage: {} <listen-addr> <prefixes-file> <ris-file> \
                [<ris-file> ...]",
                cmd
            );
            process::exit(1);
        }
    };
    let prefix_path = match args.next() {
        Some(path) => path,
        None => {
            eprintln!(
                "Usage: {} <listen-addr> <prefixes-file> <ris-file> \
                [<ris-file> ...]",
                cmd
            );
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

    let (tx, rx) = mpsc::channel(10);
    let ts = import_timestamps().unwrap_or_else(|_| {
        panic!(
            "{} roto-api Can't handle downloading timestamps. Exiting",
            chrono::Utc::now().to_rfc3339()
        )
    });
    println!("{:#?}", ts);

    thread::spawn(move || {
        process_tasks(store, rx);
    });

    let make_svc = make_service_fn(|_conn| {
        let tx = tx.clone();
        async move {
            // service_fn converts our function into a `Service`
            Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                let tx = tx.clone();
                process_request(req, ts, tx)
            }))
        }
    });

    println!("bind server at {}...", listen);
    let server = Server::bind(&listen).serve(make_svc);

    // Run this server for... forever!
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
