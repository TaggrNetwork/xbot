use std::fmt::Write;

use ic_cdk::api::management_canister::http_request::{
    http_request, CanisterHttpRequestArgument, HttpMethod, HttpResponse, TransformArgs,
    TransformContext,
};

use crate::{log_call_error, mutate, schedule_message};

const CYCLES: u128 = 30_000_000_000;
const MAX_MSG_BACKLOG: usize = 1000;

#[ic_cdk_macros::query]
fn transform_wg_response(mut args: TransformArgs) -> HttpResponse {
    args.response.headers.clear();
    let re = regex::Regex::new(r#"<a.+?href="([^"]+)".*?>.*?Full Story.*?<\/a>"#).unwrap();
    let line_formating = |line: &str| {
        let line = if line.contains("Full Story") {
            &re.replace_all(line, " More [here]($1).")
        } else {
            line
        };
        strip_html(line)
            .replace("@WatcherGuru", "")
            .replace("JUST IN:", "**JUST IN**:")
            .replace("&#036;", "$")
            .replace("&#39;", "'")
    };
    args.response.body = String::from_utf8_lossy(&args.response.body)
        .split('\n')
        .filter(|message| message.contains("JUST IN"))
        .map(line_formating)
        .collect::<Vec<_>>()
        .join("\n")
        .as_bytes()
        .to_vec();
    args.response
}

pub async fn go() {
    let request = CanisterHttpRequestArgument {
        url: "https://t.me/s/WatcherGuru".to_string(),
        max_response_bytes: Some(100_000),
        method: HttpMethod::GET,
        transform: Some(TransformContext::from_name(
            "transform_wg_response".to_string(),
            Default::default(),
        )),
        ..Default::default()
    };

    match http_request(request, CYCLES).await {
        Ok((response,)) => {
            let body = String::from_utf8_lossy(&response.body);
            let messages = body.split('\n');
            let time = ic_cdk::api::time();

            mutate(|state| {
                for message in messages {
                    if state.wg_messages.contains(message) {
                        continue;
                    }
                    schedule_message(
                        state,
                        format!("{}  \n#WatcherGuru", message),
                        Some("NEWS".into()),
                    );
                    state.wg_messages.insert(message.to_string());
                    let entry = state.wg_messages_timestamps.entry(time).or_default();
                    entry.push(message.to_string());
                }

                while state.wg_messages.len() > MAX_MSG_BACKLOG {
                    let Some((_, msgs)) = state.wg_messages_timestamps.pop_first() else {
                        break;
                    };
                    for msg in msgs {
                        state.wg_messages.remove(&msg);
                    }
                }
            })
        }
        Err(err) => log_call_error(err),
    }
}

fn strip_html(input: &str) -> String {
    let mut result = String::new();
    let mut tag = false;
    for c in input.chars() {
        if tag {
            if c == '>' {
                tag = false;
            }
            continue;
        }
        if c == '<' {
            tag = true;
            continue;
        }
        let _ = result.write_char(c);
    }
    result
}
