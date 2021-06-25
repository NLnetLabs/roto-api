use std::{env, process, thread};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::str::FromStr;
use hyper::{Body, Request, Response, Server, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use rir_lir::{Addr, Prefix, RecordSet, Store};
use tokio::sync::{mpsc, oneshot};


//------------ process_tasks -------------------------------------------------

fn process_tasks(
    store: Store,
    mut queue: mpsc::Receiver<(Prefix, oneshot::Sender<Response<Body>>)>,
) {
    while let Some((prefix, tx)) = queue.blocking_recv() {
        // Build response. Push response to oneshot.
        unimplemented!()
    }
}


//------------ process_request -----------------------------------------------

async fn process_request(
    req: Request<Body>,
    tx: mpsc::Sender<(Prefix, oneshot::Sender<Response<Body>>)>
) -> Result<Response<Body>, Infallible> {
    let mut url = req.uri().path().split("/");
    let addr = match url.next().and_then(|s| Addr::from_str(s).ok()) {
        Some(addr) => addr,
        None => return not_found(),
    };
    let len = match url.next().and_then(|s| u8::from_str(s).ok()) {
        Some(len) => len,
        None => return not_found()
    };
    if url.next().as_ref() != Some(&"search") {
        return not_found()
    }
    if url.next().is_some() {
        return not_found()
    }

    let (resp_tx, resp_rx) = oneshot::channel();
    if tx.send((Prefix::new(addr, len), resp_tx)).await.is_err() {
        return Ok(internal_server_error());
    }
    Ok(resp_rx.await.unwrap_or_else(|_| internal_server_error()))
}

fn not_found() -> Result<Response<Body>, Infallible> {
    Ok(
        Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::empty())
        .unwrap()
    )
}

fn internal_server_error() -> Response<Body> {
        Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
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
        eprintln!(
            "Failed to load {}: {}",
            prefix_path, err
        );
        process::exit(1);
    }
    for path in args {
        if let Err(err) = store.load_riswhois(path.as_ref()) {
            eprintln!(
                "Failed to load {}: {}",
                path, err
            );
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

    let server = Server::bind(&listen).serve(make_svc);

    // Run this server for... forever!
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

