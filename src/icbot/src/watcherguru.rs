use std::fmt::Write;

use ic_cdk::api::management_canister::http_request::{
    http_request, CanisterHttpRequestArgument, HttpMethod, HttpResponse, TransformArgs,
    TransformContext,
};

use crate::{post_to_taggr, state, state_mut};

const CYCLES: u128 = 30_000_000_000;

#[ic_cdk_macros::query]
fn transform_wg_args(mut args: TransformArgs) -> HttpResponse {
    args.response.headers.clear();
    args.response.body = String::from_utf8_lossy(&args.response.body)
        .split("\n")
        .filter(|message| message.contains("JUST IN"))
        .map(strip_html)
        .map(|msg| msg.replace("@WatcherGuru", ""))
        .map(|msg| msg.replace("JUST IN:", "**JUST IN**:"))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
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
            "transform_wg_args".to_string(),
            Default::default(),
        )),
        ..Default::default()
    };

    let log_error = |(r, m)| {
        state_mut().logs.push(format!(
            "HTTP request to WatcherGuru failed with rejection code={r:?}, Error: {m}"
        ))
    };

    match http_request(request, CYCLES).await {
        Ok((response,)) => {
            let last_msg = state().last_wg_message.clone();
            let body = String::from_utf8_lossy(&response.body);
            let messages = body
                .split("\n")
                .take_while(|message| *message != last_msg.as_str())
                .collect::<Vec<_>>();

            state_mut().last_wg_message = messages[0].clone().to_string();

            for message in messages {
                post_to_taggr(format!("{}\n  #WatcherGuru", message), Some("NEWS".into())).await;
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
