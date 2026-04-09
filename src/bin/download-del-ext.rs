use std::{error::Error, io, process::{Command, Output}, time::SystemTime};

use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() {
    let Ok(roto_api_peer) = std::env::var("ROTO_API_PEER") else {
        eprintln!("ROTO_API_PEER not set!");
        return;
    };

    let afrinic = download("afrinic", "https://ftp.afrinic.net/pub/stats/afrinic/delegated-afrinic-extended-latest");
    let apnic = download("apnic", "https://ftp.apnic.net/stats/apnic/delegated-apnic-extended-latest");
    let arin = download("arin", "https://ftp.arin.net/pub/stats/arin/delegated-arin-extended-latest");
    let lacnic = download("lacnic", "https://ftp.lacnic.net/pub/stats/lacnic/delegated-lacnic-extended-latest");
    let ripencc = download("ripencc", "https://ftp.ripe.net/pub/stats/ripencc/delegated-ripencc-extended-latest");

    let mut new = false;
    let mut abort = false;
    for rir in [afrinic, apnic, arin, lacnic, ripencc] {
        match rir.await {
            Ok(true) => new = true,
            Ok(false) => {},
            Err(e) => {
                eprintln!("Error: {}", e);
                abort = true;
            }
        }
    }

    if abort {
        eprintln!("Aborting update due to problem with one of the RIRs");
        return;
    }

    if new {
        let mut concatenated = String::new();
        for rir in ["afrinic", "apnic", "arin", "lacnic", "ripencc"] {
            let path = format!("downloads/del_ext/delegated-{}-extended-latest.txt", rir);
            concatenated.push_str(&tokio::fs::read_to_string(path).await.unwrap_or_default());
        }
        if let Err(e) = tokio::fs::write("data/delegated_all.csv", concatenated).await {
            eprintln!("Could not write to delegated_all.csv: {}", e);
        }
        println!("Data updated");

        if let Err(e) = timestamps().await {
            eprintln!("Could not parse timestamps: {}", e);
        }

        let restart_roto = Command::new("systemctl")
            .args([
                "--user",
                "restart",
                "roto-api"
            ])
            .env("XDG_RUNTIME_DIR", "/run/user/1000")
            .env("DBUS_SESSION_BUS_ADDRESS", "unix:path=/run/user/1000/bus")
            .output();
        let rsync = Command::new("rsync")
            .args([
                "-Cavz", 
                "--delete", 
                "data/", 
                &format!("{}:/home/roto/ris_alloc_api/data/", roto_api_peer)
            ]).output();
        let ssh = Command::new("ssh")
            .args([
                &format!("roto@{}", roto_api_peer),
                "systemctl --user restart roto-api"
            ]).output();

        fn print_output_or_error(output_or_error: Result<Output, io::Error>) {
            match output_or_error {
                Ok(output) => println!("{}\n\n{}", 
                    String::from_utf8_lossy(&output.stdout), 
                    String::from_utf8_lossy(&output.stderr)
                ),
                Err(e) => eprintln!("{}", e)
            }
        }
        println!("Restarting roto");
        print_output_or_error(restart_roto);
        println!("rsync");
        print_output_or_error(rsync);
        println!("ssh");
        print_output_or_error(ssh);
    } else {
        println!("No data was updated");
    }  
}

async fn header<'a>(search: &str, rir: &str) -> Result<Option<String>, Box<dyn Error>> {
    let headers_path = format!("downloads/{}_h.txt", rir);
    let headers = tokio::fs::read_to_string(&headers_path).await?;
    for header in headers.split("\r\n") {
        if let Some((name, value)) = header.split_once(": ") {
            if name.to_uppercase() == search.to_uppercase() {
                return Ok(Some(value.to_string()));
            }
        }
    }
    Ok(None)
}

async fn timestamps() -> Result<bool, Box<dyn Error>> {
    let path = "data/del_ext.timestamps.json";
    let mut file = tokio::fs::File::create(path).await?;
    file.write("rir,file_timestamp,last_modified_header\n".as_bytes()).await?;
    for rir in ["afrinic", "apnic", "arin", "lacnic", "ripencc"] {
        let metadata_path = format!("downloads/del_ext/delegated-{}-extended-latest.txt", rir);
        let metadata = tokio::fs::metadata(metadata_path).await?;
        let file_modified = metadata
            .modified()?
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();

        let last_modified = header("LAST-MODIFIED", rir).await?.unwrap_or_default();

        let entry = format!("{},{},\"{}\"\n", 
            rir, 
            file_modified,
            last_modified
        );

        file.write(entry.as_bytes()).await?;
    }

    Ok(true)
}

async fn download(rir: &str, url: &str) -> Result<bool, Box<dyn Error>> {
    let path = format!("downloads/del_ext/delegated-{}-extended-latest.txt", rir);
    let previous = tokio::fs::File::open(&path).await;
    let previous = match previous {
        Ok(mut file) => {
            let mut contents = Vec::new();
            file.read_to_end(&mut contents).await?;
            contents
        },
        Err(_) => Vec::new()
    };

    let mut current = Vec::new();
    let mut builder = reqwest::Client::new()
        .get(url)
        .header("User-Agent", "roto-api");

    if let Ok(Some(etag)) = header("ETag", rir).await {
        builder = builder.header("If-None-Match", etag);
    }

    let mut response = builder.send().await?;
    if response.status() == 304 {
        println!("{} has not changed", rir);
        return Ok(false);
    }

    let mut file = tokio::fs::File::create(&path).await?;
    while let Some(chunk) = response.chunk().await? {
        current.extend_from_slice(&chunk.to_vec());
        file.write_all(&chunk).await?;
    }

    let headers_path = format!("downloads/{}_h.txt", rir);
    let mut headers_file = tokio::fs::File::create(&headers_path).await?;
    for (name, value) in response.headers() {
        let header = format!("{}: {}\r\n", name.as_str(), value.to_str().unwrap_or_default());
        headers_file.write(header.as_bytes()).await?;
    }
    println!("{} downloaded", rir);
    Ok(previous != current)
}