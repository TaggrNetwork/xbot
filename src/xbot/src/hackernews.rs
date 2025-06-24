use ic_cdk::api::management_canister::http_request::{
    http_request, CanisterHttpRequestArgument, HttpMethod, HttpResponse, TransformArgs,
    TransformContext,
};
use serde::Deserialize;

use crate::{mutate, read, schedule_message};

#[derive(Deserialize)]
struct Story {
    id: u32,
    title: String,
    url: String,
    // kids: Vec<u32>,
    // score: u32,
    // time: u64,
    // descendants: u32,
    // by: String,
}

const CYCLES: u128 = 30_000_000_000;
const MAX_STORIES_PER_DAY: usize = 6;

#[ic_cdk_macros::query]
fn transform_hn_response(mut args: TransformArgs) -> HttpResponse {
    args.response.headers.clear();
    args.response
}

pub async fn go() -> Result<(), String> {
    let request = CanisterHttpRequestArgument {
        url: "https://hacker-news.firebaseio.com/v0/beststories.json".to_string(),
        method: HttpMethod::GET,
        max_response_bytes: Some(3000),
        transform: Some(TransformContext::from_name(
            "transform_hn_response".to_string(),
            Default::default(),
        )),
        ..Default::default()
    };

    let (response,) = http_request(request, CYCLES)
        .await
        .map_err(|err| format!("http_request failed: {:?}", err))?;
    let best_stories: Vec<u64> = serde_json::from_slice(&response.body)
        .map_err(|err| format!("json parsing failed: {:?}", err))?;

    let mut last_best_story = read(|s| s.last_best_story);
    let mut total = 0;
    for id in best_stories.into_iter() {
        if total >= MAX_STORIES_PER_DAY {
            break;
        }
        if id <= last_best_story {
            continue;
        }
        fetch_story(id).await?;
        last_best_story = id;
        total += 1;
    }
    mutate(|s| s.last_best_story = last_best_story);
    Ok(())
}

async fn fetch_story(id: u64) -> Result<(), String> {
    let request = CanisterHttpRequestArgument {
        max_response_bytes: Some(3000),
        url: format!("https://hacker-news.firebaseio.com/v0/item/{}.json", id),
        method: HttpMethod::GET,
        ..Default::default()
    };
    let (response,) = http_request(request, CYCLES)
        .await
        .map_err(|err| format!("http_request failed: {:?}", err))?;
    let Story { id, title, url, .. } = serde_json::from_slice(&response.body)
        .map_err(|err| format!("json parsing failed: {:?}", err))?;
    let publisher = url::Url::parse(&url)
        .ok()
        .and_then(|u| u.host_str().map(|host| host.to_string()))
        .unwrap_or_default();
    let message = format!(
        "# #HackerNews: [{}]({})\nFrom {}, [Comments](https://news.ycombinator.com/item?id={})",
        title, url, publisher, id
    );
    mutate(|s| schedule_message(s, message, Some("TECHNOLOGY".into())));
    Ok(())
}
