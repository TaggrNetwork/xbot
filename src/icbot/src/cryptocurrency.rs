use ic_cdk::api::management_canister::http_request::{
    http_request, CanisterHttpRequestArgument, HttpHeader, HttpMethod,
};
use serde::Deserialize;

use crate::{post_to_taggr, state, state_mut};

#[derive(Deserialize)]
struct Child {
    data: Story,
}

#[derive(Deserialize)]
struct Data {
    children: Vec<Child>,
}

#[derive(Deserialize)]
struct Response {
    data: Data,
}

#[derive(Deserialize)]
struct Story {
    title: String,
    ups: u32,
    num_comments: u32,
    url: String,
    permalink: String,
}

const CYCLES: u128 = 30_000_000_000;

// Reddit doesn't seem to work becasue IC only supports IPv6
#[allow(dead_code)]
pub async fn go() {
    let request = CanisterHttpRequestArgument {
        url: "https://www.reddit.com/r/cryptocurrency/top.json?limit=1".to_string(),
        method: HttpMethod::GET,
        headers: vec![HttpHeader {
            name: "User-Agent".to_string(),
            value: "Taggr.link".to_string(),
        }],
        ..Default::default()
    };

    let log_error = |(r, m)| {
        state_mut().logs.push(format!(
            "HTTP request to Reddit failed with rejection code={r:?}, Error: {m}"
        ))
    };

    match http_request(request, CYCLES).await {
        Ok((response,)) => {
            let response: Response = match serde_json::from_slice(&response.body) {
                Ok(val) => val,
                Err(err) => {
                    state_mut()
                        .logs
                        .push(format!("couldn't deserialize JSON response: {:?}", err));
                    return;
                }
            };

            let Story {
                title,
                ups,
                num_comments,
                url,
                permalink,
            } = &response.data.children[0].data;
            if &state().last_best_post == permalink {
                return;
            }
            state_mut().last_best_post = permalink.into();
            let publisher = url::Url::parse(url)
                .ok()
                .and_then(|u| u.host_str().map(|host| host.to_string()))
                .unwrap_or_default();
            let message = format!(
                        "## [{}]({}) ({})\nToday's best #CryptoCurrencySubreddit story: `{}` upvotes, [{} comments](https://reddit.com/{})",
                        title, url, publisher, ups, num_comments, permalink
                    );
            post_to_taggr(message, None).await;
        }
        Err(err) => log_error(err),
    }
}
