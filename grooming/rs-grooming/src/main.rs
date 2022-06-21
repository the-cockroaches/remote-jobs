use futures::{stream, StreamExt};
use itertools::Itertools;
use regex::Regex;
use reqwest::redirect::Policy;
use reqwest::Client;
use std::collections::HashSet;
use std::fs::File;
use std::io::prelude::*;
use std::time::Instant;
use std::time::Duration;

const PARALLEL_REQUESTS: usize = 10;

fn chop_url(url: &str) -> String {
    let re = Regex::new(r"(https?)://(www\.)?(.+)").unwrap();
    let caps = re.captures(url);
    if let Some(caps) = caps {
        caps.get(3).unwrap().as_str().to_string()
    } else {
        url.to_string()
    }
}

fn build_url_variations(url: &str) -> Vec<String> {
    let partial_url = chop_url(url);
    // NOTE: order matters here
    let mut variations = vec![
        format!("https://{}", partial_url),
        format!("http://{}", partial_url),
        format!("https://www.{}", partial_url),
        format!("http://www.{}", partial_url),
    ];
    if !variations.contains(&url.to_string()) {
        variations.push(url.to_owned()); // AS IS
    }
    variations
}

async fn hit_url<'a>(url: &str) -> Result<String, reqwest::Error> {
    println!("hitting {}", url);
    let now = Instant::now();
    let url_local = url.to_owned();
    let client = Client::builder()
        .redirect(Policy::custom(move |attempt| {
            println!(
                "redirecting to {} from {}",
                attempt.url().to_string(), // TODO: use attempt url as return?
                url_local
            );
            attempt.follow()
        }))
        .timeout(Duration::from_secs(120))
        .build()?;

    client.get(url).send().await?;
    println!("{} secs for {}", now.elapsed().as_secs(), url);
    Ok(url.to_owned())
}

struct CheckedUrl {
    is_valid: bool,
    url: String,
}

async fn check_url(url: &str) -> CheckedUrl {
    let variations = build_url_variations(url);
    for variation in variations {
        let result = hit_url(&variation).await;
        if result.is_ok() {
            return CheckedUrl {
                is_valid: true,
                url: variation,
            };
        }
    }
    println!("invalid - {}", url);
    CheckedUrl {
        is_valid: false,
        url: url.to_owned(),
    }
}



#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut file = File::open("../../job-links.txt")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let urls = contents
        .split('\n')
        .into_iter()
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .unique()
        .collect::<Vec<String>>();

    let results = stream::iter(urls)
        .map(|url| tokio::spawn(async move { 
            check_url(&url).await 
        }))
        .buffer_unordered(PARALLEL_REQUESTS);

    let valid_urls = HashSet::new();
    let invalid_urls = HashSet::new();

    struct UrlSets {
        valid_urls: HashSet<String>,
        invalid_urls: HashSet<String>,
    }

    let UrlSets { valid_urls, invalid_urls } = results
        .fold(UrlSets{ valid_urls, invalid_urls }, |mut acc, result| async {
            let UrlSets { valid_urls, invalid_urls } = &mut acc;
            match result {
                Ok(checked) => {
                    if checked.is_valid {
                        valid_urls.insert(checked.url.clone());
                    } else {
                        invalid_urls.insert(checked.url.clone());
                    }
                },
                Err(e) => eprintln!("Got a tokio::JoinError: {}", e),
            }
            acc
        })
        .await;

    println!("valid urls:\n{}", valid_urls.iter().sorted().dedup().join("\n"));
    println!("invalid urls:\n{}", invalid_urls.iter().sorted().dedup().join("\n"));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chop_url() {
        assert_eq!(chop_url("https://www.google.com"), "google.com");
        assert_eq!(chop_url("https://www.google.com/"), "google.com/");
        assert_eq!(
            chop_url("https://www.google.com/search?q=rust&p=1"),
            "google.com/search?q=rust&p=1"
        );
    }

    #[test]
    fn test_build_url_variations() {
        assert_eq!(
            build_url_variations("https://drupaljedi.com"),
            vec![
                "https://drupaljedi.com",
                "http://drupaljedi.com",
                "https://www.drupaljedi.com",
                "http://www.drupaljedi.com",
            ]
        );
        assert_eq!(
            build_url_variations("https://www.google.com/search?q=rust&p=1"),
            vec![
                "https://google.com/search?q=rust&p=1",
                "http://google.com/search?q=rust&p=1",
                "https://www.google.com/search?q=rust&p=1",
                "http://www.google.com/search?q=rust&p=1",
            ]
        );
    }
}


/*

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut file = File::open("../../job-links.txt")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let urls = contents
        .split('\n')
        .into_iter()
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .unique()
        .collect::<Vec<&str>>();

    let url_status_list = join_all(urls.iter().map(|url| check_url(url)).collect::<Vec<_>>()).await;

    let mut valid_urls = HashSet::new();
    let mut invalid_urls = HashSet::new();

    url_status_list.iter().for_each(|x| {
        if x.is_valid {
            valid_urls.insert(x.url.clone());
        } else {
            invalid_urls.insert(x.url.clone());
        }
    });

    println!("valid urls:\n{}", valid_urls.iter().join("\n"));
    println!("invalid urls:\n{}", invalid_urls.iter().join("\n"));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chop_url() {
        assert_eq!(chop_url("https://www.google.com"), "google.com");
        assert_eq!(chop_url("https://www.google.com/"), "google.com/");
        assert_eq!(
            chop_url("https://www.google.com/search?q=rust&p=1"),
            "google.com/search?q=rust&p=1"
        );
    }

    #[test]
    fn test_build_url_variations() {
        assert_eq!(
            build_url_variations("https://www.google.com"),
            vec![
                "https://google.com",
                "http://google.com",
                "https://www.google.com",
                "http://www.google.com",
            ]
        );
        assert_eq!(
            build_url_variations("https://www.google.com/search?q=rust&p=1"),
            vec![
                "https://google.com/search?q=rust&p=1",
                "http://google.com/search?q=rust&p=1",
                "https://www.google.com/search?q=rust&p=1",
                "http://www.google.com/search?q=rust&p=1",
            ]
        );
    }
}

*/