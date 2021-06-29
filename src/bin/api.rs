use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, StatusCode};
use rir_lir::{Addr, JsonBuilder, Prefix, Store};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::str::FromStr;
use std::{env, process, thread};
use tokio::sync::{mpsc, oneshot};

//------------ process_tasks -------------------------------------------------

fn process_tasks(
    store: Store,
    mut queue: mpsc::Receiver<(Prefix, oneshot::Sender<Response<Body>>)>,
) {
    while let Some((prefix, tx)) = queue.blocking_recv() {
        let recs = store.match_longest_prefix(prefix);
        let cc = recs.clone();

        let exact_match = cc.iter().last().and_then(|(p, r)| {
            if p.len == prefix.len {
                Some((p, r))
            } else {
                None
            }
        });
        let less_spec_results = cc.iter().filter(|(p, _r)| p.len != prefix.len);
        println!("exact match {:?}", exact_match);

        let res = JsonBuilder::build(|builder| {
            builder.member_str("type", "exact-match");
            builder.member_str("prefix", prefix);
            builder.member_array("results", |builder| {
                if let Some((pfx, value)) = exact_match {
                    if let Some(ext_rec) = value {
                        builder.array_object(|builder| {
                            builder.member_str("prefix", pfx);
                            builder.member_array("results", |builder| {
                                // rir delegated extended records
                                match &ext_rec.0 {
                                    Some(rir_del_ext_r) => {
                                        builder.array_object(|builder| {
                                            builder.member_str("source", "rir_alloc");
                                            builder.member_str("rir", rir_del_ext_r.rir);
                                        });
                                    }
                                    None => {}
                                }
                                // rishwhois records
                                match &ext_rec.1 {
                                    Some(ris_whois_r) => {
                                        builder.array_object(|builder| {
                                            builder.member_str("source", "bgp");
                                            builder.member_array("origin_asns", |builder| {
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
                        });
                    }
                }
            });

            // Look for the longest-matching prefix with a DelExtRecord.
            // The vecs in a RecordSet are ordered from least to most specific,
            // hence the reverse.
            if let Some(lmp_rel_rec) = recs.reverse().iter().find_map(|(_p, r)| match r {
                Some(rec) => rec.0.as_ref(),
                None => None,
            }) {
                println!("lmp rec {:?}", lmp_rel_rec);
                let rel_rec = store.get_related_prefixes(&lmp_rel_rec);
                builder.member_array("relations", |builder| {
                    for (pfx, value) in rel_rec.iter() {
                        builder.array_object(|builder| {
                            builder.member_str("prefix", pfx);
                            builder.member_str("type", "same-org");
                            builder.member_array("results", |builder| {
                                if let Some(ext_rec) = value {
                                    match &ext_rec.0 {
                                        Some(rir_del_ext_r) => {
                                            builder.array_object(|builder| {
                                                builder.member_str("source", "rir_alloc");
                                                builder.member_str("rir", rir_del_ext_r.rir);
                                            });
                                        }
                                        None => {}
                                    }
                                    match &ext_rec.1 {
                                        Some(ris_whois_r) => {
                                            builder.array_object(|builder| {
                                                builder.member_str("source", "bgp");
                                                builder.member_array("origin_asns", |builder| {
                                                    for asn in ris_whois_r.origin_asns.0.iter() {
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
                    for (pfx, value) in less_spec_results {
                        builder.array_object(|builder| {
                            builder.member_str("prefix", pfx);
                            builder.member_str("type", "less-specific");
                            builder.member_array("results", |builder| {
                                if let Some(ext_rec) = value {
                                    match &ext_rec.0 {
                                        Some(rir_del_ext_r) => {
                                            builder.array_object(|builder| {
                                                builder.member_str("source", "rir_alloc");
                                                builder.member_str("rir", rir_del_ext_r.rir);
                                            });
                                        }
                                        None => {}
                                    }
                                    match &ext_rec.1 {
                                        Some(ris_whois_r) => {
                                            builder.array_object(|builder| {
                                                builder.member_str("source", "bgp");
                                                builder.member_array("origin_asns", |builder| {
                                                    for asn in ris_whois_r.origin_asns.0.iter() {
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
                })
            }
        });
        let _err = tx.send(
            Response::builder()
                .status(hyper::StatusCode::OK)
                .header(hyper::header::CONTENT_TYPE, "application/json")
                .body(hyper::Body::from(res))
                .unwrap(),
        );
    }
}

//------------ process_request -----------------------------------------------

async fn process_request(
    req: Request<Body>,
    tx: mpsc::Sender<(Prefix, oneshot::Sender<Response<Body>>)>,
) -> Result<Response<Body>, Infallible> {
    println!("got request");
    let mut url = req.uri().path().split("/");
    println!("{:?}", req.uri().path());
    println!("{:?}", url);
    let _slash = url.next();

    let addr = match url.next().and_then(|s| {
        println!("s {}", s);
        Addr::from_str(s).ok()
    }) {
        Some(addr) => addr,
        None => {
            println!("no parse addr");
            return not_found();
        }
    };
    let len = match url.next().and_then(|s| u8::from_str(s).ok()) {
        Some(len) => len,
        None => {
            println!("no parse len");
            return not_found();
        }
    };
    if url.next().as_ref() != Some(&"search") {
        println!("no parse action");
        return not_found();
    }
    if url.next().is_some() {
        println!("found too much crap");
        return not_found();
    }
    println!("got here");

    let (resp_tx, resp_rx) = oneshot::channel();
    if tx.send((Prefix::new(addr, len), resp_tx)).await.is_err() {
        return Ok(internal_server_error());
    }
    Ok(resp_rx.await.unwrap_or_else(|_| internal_server_error()))
}

fn not_found() -> Result<Response<Body>, Infallible> {
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"results": null, "error": true, "error_msg": "Cannot parse the query"}"#
                .to_string(),
        ))
        .unwrap())
}

fn internal_server_error() -> Response<Body> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Body::empty())
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

    let mut store = Store::new();
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

    thread::spawn(move || {
        process_tasks(store, rx);
    });

    let make_svc = make_service_fn(|_conn| {
        let tx = tx.clone();
        async move {
            // service_fn converts our function into a `Service`
            Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                let tx = tx.clone();
                process_request(req, tx)
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
