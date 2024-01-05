use std::fmt::Write;

use ic_cdk::api::management_canister::http_request::{
    http_request, CanisterHttpRequestArgument, HttpMethod, HttpResponse, TransformArgs,
    TransformContext,
};

use crate::{schedule_message, state, state_mut};

const CYCLES: u128 = 30_000_000_000;

#[ic_cdk_macros::query]
fn transform_wg_response(mut args: TransformArgs) -> HttpResponse {
    args.response.headers.clear();
    args.response.body = String::from_utf8_lossy(&args.response.body)
        .split('\n')
        .filter(|message| message.contains("JUST IN"))
        .map(strip_html)
        .map(|msg| msg.replace("@WatcherGuru", ""))
        .map(|msg| msg.replace("JUST IN:", "**JUST IN**:"))
        .map(|msg| msg.replace("&#036;", "$"))
        .map(|msg| msg.replace("&#39;", "'"))
        .collect::<Vec<_>>()
        .join("\n")
        .as_bytes()
        .to_vec();
    args.response
}

pub async fn go() {
    let request = CanisterHttpRequestArgument {
        url: "https://t.me/s/WatcherGuru".to_string(),
        method: HttpMethod::GET,
        transform: Some(TransformContext::from_name(
            "transform_wg_response".to_string(),
            Default::default(),
        )),
        ..Default::default()
    };

    let log_error = |(r, m)| {
        state_mut().logs.push_back(format!(
            "HTTP request to WatcherGuru failed with rejection code={r:?}, Error: {m}"
        ))
    };

    match http_request(request, CYCLES).await {
        Ok((response,)) => {
            let last_msg = state().last_wg_message.clone();
            let body = String::from_utf8_lossy(&response.body);
            let messages = body.split('\n');
            let next_new_message_id = messages
                .clone()
                .position(|msg| msg == last_msg)
                .map(|n| n + 1)
                .unwrap_or(0);
            let messages = messages.collect::<Vec<_>>().split_off(next_new_message_id);

            let state = state_mut();
            for message in messages {
                schedule_message(format!("{}  \n#WatcherGuru", message), Some("NEWS".into()));
                state.last_wg_message = message.to_string();
            }
        }
        Err(err) => log_error(err),
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
