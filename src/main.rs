use futures::future::join_all;
use itertools::Itertools;
use regex::Regex;
use reqwest::redirect::Policy;
use reqwest::Client;
use std::borrow::Cow;
use std::collections::HashSet;
use std::fs::File;
use std::io::prelude::*;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::time::Instant;

fn remove_url_protocol(url: &str) -> Cow<'_, str> {
    let re = Regex::new(r"^(https?)://").unwrap();
    re.replace(url, "")
}

fn remove_www(url: &str) -> Cow<'_, str> {
    let re = Regex::new(r"(www\.)").unwrap();
    re.replace(url, "")
}

fn sanitize_url(url: &str) -> String {
    remove_www(remove_url_protocol(url).deref()).into_owned()
}

fn build_url_variations(url: &str) -> Vec<String> {
    let partial_url = sanitize_url(url);
    // NOTE: order matters here
    let mut variations = vec![
        format!("https://{}", partial_url),
        format!("https://www.{}", partial_url),
        format!("http://{}", partial_url),
        format!("http://www.{}", partial_url),
    ];
    if !variations.contains(&url.to_string()) {
        variations.push(url.to_owned()); // AS IS
    }
    variations
}

async fn hit_url(url: &str) -> Result<String, reqwest::Error> {
    println!("hitting {}", url);
    let now = Instant::now();
    let final_url = Arc::new(Mutex::new(url.to_owned()));
    let client = Client::builder()
        .redirect(Policy::custom({
            let final_url = Arc::clone(&final_url);
            move |attempt| {
                let new_url = attempt.url().to_string();
                println!(
                    "redirecting to {} from {}",
                    new_url,
                    final_url.lock().unwrap().as_str()
                );
                *final_url.lock().unwrap() = new_url;
                attempt.follow() // Follow the redirect
            }
        }))
        .timeout(Duration::from_secs(120))
        .build()?;

    client.get(url).send().await?;
    println!("{} secs for {}", now.elapsed().as_secs(), url);
    let final_url = final_url.lock().unwrap().to_owned();
    Ok(final_url)
}

struct CheckedUrl {
    is_valid: bool,
    url: String,
}

async fn check_url(url: &str) -> CheckedUrl {
    let variations = build_url_variations(url);
    for variation in variations {
        if let Ok(final_url) = hit_url(&variation).await {
            return CheckedUrl {
                is_valid: true,
                url: final_url,
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
    let mut file = File::open("job-links.txt")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let urls = contents
        .split('\n')
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_owned())
            }
        })
        .unique()
        .collect::<Vec<String>>();

    let valid_urls = Arc::new(Mutex::new(HashSet::new()));
    let invalid_urls = Arc::new(Mutex::new(HashSet::new()));

    // Create a vector of futures (tasks) to execute in parallel
    let tasks = urls.into_iter().map(|url| {
        let valid_urls_pointer = Arc::clone(&valid_urls);
        let invalid_urls_pointer = Arc::clone(&invalid_urls);
        tokio::spawn(async move {
            let checked = check_url(&url).await;
            if checked.is_valid {
                valid_urls_pointer.lock().unwrap().insert(checked.url);
            } else {
                invalid_urls_pointer.lock().unwrap().insert(checked.url);
            }
        })
    });

    join_all(tasks).await;

    let valid_urls = valid_urls.lock().unwrap();
    let invalid_urls = invalid_urls.lock().unwrap();

    let mut output_file = File::create("output.txt")?;
    output_file.write_all(
        format!(
            "valid urls:\n{}\n\ninvalid urls:\n{}",
            valid_urls.iter().sorted().dedup().join("\n"),
            invalid_urls.iter().sorted().dedup().join("\n")
        )
        .as_bytes(),
    )?;

    println!(
        "valid urls:\n{}\n\ninvalid urls:\n{}",
        valid_urls.iter().sorted().dedup().join("\n"),
        invalid_urls.iter().sorted().dedup().join("\n")
    );

    Ok(())
}

