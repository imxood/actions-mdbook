use futures_util::StreamExt;
use reqwest::{Error, Response};
use std::collections::HashMap;
use std::env::{self, temp_dir};
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

use indicatif::{ProgressBar, ProgressStyle};

fn get_os() -> String {
    #[cfg(target_os = "macos")]
    let os: &str = "apple-darwin";
    #[cfg(target_os = "linux")]
    let os: &str = "unknown-linux-gnu";
    #[cfg(target_os = "windows")]
    let os: &str = "pc-windows-msvc";
    String::from(os)
}

async fn download_file(response: Response, target_dir: &PathBuf) -> String {
    let mut data_length = 0;
    let mut filename;

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

#[tokio::main]
async fn main() -> Result<(), Error> {
    let token = env::var("PAGES_GENERATE_TOKEN").unwrap();
    let octocrab = octocrab::OctocrabBuilder::new()
        .add_preview("pages-generator")
        .personal_token(token)
        .build()
        .unwrap();

    // mdbook-katex
    let repo_handler = octocrab.repos("lzanini", "mdbook-katex");
    let releases_handler = repo_handler.releases();
    let release = releases_handler.get_latest().await.unwrap();
    println!("{:#?}", release);

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

    println!("asset.browser_download_url: {}", asset.browser_download_url);

    // download file
    let builder =
        octocrab.request_builder(asset.browser_download_url.clone(), reqwest::Method::GET);
    let response = octocrab.execute(builder).await.unwrap();

    let target_dir = env::current_dir().unwrap();

    let filename = download_file(response, &target_dir).await;

    println!("filename: {}", &filename);

    Ok(())
}
