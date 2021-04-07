use flate2::read::GzDecoder;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use octocrab::Octocrab;
use reqwest::Error;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
// use std::process::Command;
use tar::Archive;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let token = env::var("GITHUB_TOKEN").unwrap();
    let octocrab = octocrab::OctocrabBuilder::new()
        .add_preview("pages-generator")
        .personal_token(token)
        .build()
        .unwrap();

    let target_dir = env::current_dir().unwrap();

    // mdBook
    let download_url = get_download_url(&octocrab, "rust-lang", "mdBook").await;
    let filename = download_file(&octocrab, &download_url, &target_dir).await;
    println!("filename: {}", &filename);
    decompress_tar_gz(&filename);
    let path = target_dir.clone();
    env::join_paths(vec![&path]).unwrap();

    // mdbook-katex
    let download_url = get_download_url(&octocrab, "lzanini", "mdbook-katex").await;
    let filename = download_file(&octocrab, &download_url, &target_dir).await;
    println!("filename: {}", &filename);
    decompress_tar_gz(&filename);
    let path = target_dir
        .clone()
        .join("target/x86_64-unknown-linux-gnu/release");
    env::join_paths(vec![&path]).unwrap();

    // mdbook-mermaid
    let download_url = get_download_url(&octocrab, "badboy", "mdbook-mermaid").await;
    let filename = download_file(&octocrab, &download_url, &target_dir).await;
    println!("filename: {}", &filename);
    decompress_tar_gz(&filename);
    let path = target_dir.clone();
    env::join_paths(vec![&path]).unwrap();

    for (key, value) in env::vars() {
        println!("{}: {}", key, value);
    }

    // Command::new("mdbook")
    //     .arg("build")
    //     .output()
    //     .expect("failed to execute process");

    Ok(())
}

fn get_os() -> String {
    #[cfg(target_os = "macos")]
    let os: &str = "apple-darwin";
    #[cfg(target_os = "linux")]
    let os: &str = "unknown-linux-gnu";
    #[cfg(target_os = "windows")]
    let os: &str = "pc-windows-msvc";
    String::from(os)
}

async fn download_file(
    octocrab: &Octocrab,
    download_url: &reqwest::Url,
    target_dir: &PathBuf,
) -> String {
    let mut data_length = 0;
    let mut filename;

    let builder = octocrab.request_builder(download_url.clone(), reqwest::Method::GET);
    let response = octocrab.execute(builder).await.unwrap();

    let mut file = {
        data_length = response.content_length().unwrap();
        let hash_query: HashMap<_, _> = response.url().query_pairs().into_owned().collect();
        let v = hash_query.get("response-content-disposition").unwrap();

        let s: Vec<_> = v.split("filename=").collect();
        let name = s.get(1).unwrap();

        println!("file to download: '{}'", name);
        let path = target_dir.join(name);
        filename = String::from(path.to_str().unwrap());
        println!("will be located under: '{:?}'", path);
        File::create(path).unwrap()
    };

    let mut stream = response.bytes_stream();

    let pb = ProgressBar::new(data_length);

    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{wide_bar.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .progress_chars("#>-"));

    let mut downloaded: u64 = 0;
    while let Some(item) = stream.next().await {
        let data = item.unwrap();
        file.write_all(&data).unwrap();
        downloaded = downloaded + data.len() as u64;
        pb.set_position(downloaded);
    }
    pb.finish_with_message("downloaded");

    filename
}

async fn get_download_url(octocrab: &Octocrab, owner: &str, repo: &str) -> reqwest::Url {
    let repo_handler = octocrab.repos(owner, repo);
    let releases_handler = repo_handler.releases();
    let release = releases_handler.get_latest().await.unwrap();

    let asset = release
        .assets
        .iter()
        .find(|asset| {
            asset
                .browser_download_url
                .as_str()
                .find(get_os().as_str())
                .is_some()
        })
        .unwrap();

    asset.browser_download_url.clone()
}

fn decompress_tar_gz(tar_gz_file: &String) {
    let tar_gz = File::open(tar_gz_file).unwrap();
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(".").unwrap();
}
