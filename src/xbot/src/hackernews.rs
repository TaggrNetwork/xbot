use ic_cdk::api::management_canister::http_request::{
    http_request, CanisterHttpRequestArgument, HttpMethod,
};
use serde::Deserialize;

use crate::{post_to_taggr, state, state_mut};

#[derive(Deserialize)]
struct Story {
    id: u32,
    kids: Vec<u32>,
    score: u32,
    title: String,
    url: String,
    // time: u64,
    // descendants: u32,
    // by: String,
}

const CYCLES: u128 = 30_000_000_000;

pub async fn go() {
    let request = CanisterHttpRequestArgument {
        url: "https://hacker-news.firebaseio.com/v0/beststories.json".to_string(),
        method: HttpMethod::GET,
        ..Default::default()
    };

    let log_error = |(r, m)| {
        state_mut().logs.push_back(format!(
            "HTTP request to HN failed with rejection code={r:?}, Error: {m}"
        ))
    };

    match http_request(request, CYCLES).await {
        Ok((response,)) => {
            let best_stories: Vec<u64> = match serde_json::from_slice(&response.body) {
                Ok(val) => val,
                Err(err) => {
                    state_mut()
                        .logs
                        .push_back(format!("couldn't deserialize JSON response: {:?}", err));
                    return;
                }
            };

            let best_story = best_stories[0];
            if best_story == state().last_best_story {
                return;
            }
            state_mut().last_best_story = best_story;
            let request = CanisterHttpRequestArgument {
                url: format!(
                    "https://hacker-news.firebaseio.com/v0/item/{}.json",
                    best_story
                ),
                method: HttpMethod::GET,
                ..Default::default()
            };
            match http_request(request, CYCLES).await {
                Ok((response,)) => {
                    let Story {
                        id,
                        title,
                        score,
                        kids,
                        url,
                        ..
                    } = match serde_json::from_slice(&response.body) {
                        Ok(val) => val,
                        Err(err) => {
                            state_mut().logs.push_back(format!(
                                "couldn't deserialize JSON response: {:?}",
                                err
                            ));
                            return;
                        }
                    };
                    let publisher = url::Url::parse(&url)
                        .ok()
                        .and_then(|u| u.host_str().map(|host| host.to_string()))
                        .unwrap_or_default();
                    let message = format!(
                        "## [{}]({}) ({})\n`{}` upvotes, [{} comments](https://news.ycombinator.com/item?id={})\n#HackerNews",
                        title, url, publisher, score, kids.len(), id
                    );
                    let _ = post_to_taggr(message, None).await;
                }
                Err(err) => log_error(err),
            }
        }
        Err(err) => log_error(err),
    }
}
