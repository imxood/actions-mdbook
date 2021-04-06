#![feature(str_split_as_str)]
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;

use clap::{self, App, Arg};
use reqwest::Error;
use reqwest::{self, header};
use serde::Deserialize;
use futures_util::StreamExt;

#[derive(Deserialize, Debug)]
struct RepoInfo {
    tag_name: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let matches = App::new("get git release program")
        .version("1.0")
        .author("maxu")
        .arg(
            Arg::with_name("owner")
                .takes_value(true)
                .required(true)
                .help("Sets github repo's owner"),
        )
        .arg(
            Arg::with_name("repo")
                .takes_value(true)
                .required(true)
                .help("Sets github repo name"),
        )
        .arg(
            Arg::with_name("version")
                .takes_value(true)
                .default_value("latest")
                .help("Sets github repo release version"),
        )
        .get_matches();
    let owner = matches.value_of("owner").unwrap();
    let repo = matches.value_of("repo").unwrap();
    let version = matches.value_of("version").unwrap();

    github_release(owner, repo, version).await?;
    Ok(())
}

async fn github_repo_version(owner: &str, repo: &str, version: &str) -> Result<String, Error> {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_static("imxood"),
    );

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    let url = format!(
        "https://api.github.com/repos/{owner}/{repo}/releases/{version}",
        owner = owner,
        repo = repo,
        version = version,
    );

    println!("url: {}", url);

    let response = client.get(&url).send().await?;

    let info: RepoInfo = response.json().await?;

    Ok(info.tag_name)
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

async fn gei_download_url(owner: &str, repo: &str, version: &str) -> Result<String, Error> {
    let os = get_os();
    let version = github_repo_version(owner, repo, version).await?;
    println!("version: {}", &version);
    let mdbook_name = format!(
        "{repo}-{version}-x86_64-{os}.tar.gz",
        repo = repo,
        version = version,
        os = os
    );
    let repo_url = format!(
        "https://github.com/{owner}/{repo}/releases/download/{version}/{mdbook_name}",
        owner = owner,
        repo = repo,
        version = version,
        mdbook_name = mdbook_name
    );
    Ok(repo_url)
}

async fn github_release(owner: &str, repo: &str, version: &str) -> Result<(), Error> {
    let download_url = gei_download_url(owner, repo, version).await?;
    println!("repo_url: {}", &download_url);
    let tmpdir = std::env::temp_dir();
    let response = reqwest::get(download_url).await?;

    let mut file = {
        let length = response.content_length().unwrap();
        let hash_query: HashMap<_, _> = response.url().query_pairs().into_owned().collect();
        let v = hash_query.get("response-content-disposition").unwrap();

        let s: Vec<_> = v.split("filename=").collect();
        let filename = s.get(1).unwrap();

        println!("file to download: '{}'", filename);
        let filename = tmpdir.join(filename);
        println!("will be located under: '{:?}'", filename);
        File::create(filename).unwrap()
    };

    let mut stream = response.bytes_stream();

    while let Some(item) = stream.next().await {
        file.write_all(item?.as_ref()).unwrap();
        println!("done");
    }

    Ok(())
}
