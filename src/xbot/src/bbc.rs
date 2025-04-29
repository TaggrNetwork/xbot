use chrono::DateTime;
use ic_cdk::api::management_canister::http_request::{
    http_request, CanisterHttpRequestArgument, HttpMethod, HttpResponse, TransformArgs,
    TransformContext,
};

use crate::{mutate, read, schedule_message};

const CYCLES: u128 = 30_000_000_000;

#[ic_cdk_macros::query]
fn transform_bbc_response(mut args: TransformArgs) -> HttpResponse {
    args.response.headers.clear();
    args.response
}

pub async fn go() -> Result<(), String> {
    let request = CanisterHttpRequestArgument {
        url: "https://feeds.bbci.co.uk/news/rss.xml".to_string(),
        method: HttpMethod::GET,
        max_response_bytes: Some(40000),
        transform: Some(TransformContext::from_name(
            "transform_bbs_response".to_string(),
            Default::default(),
        )),
        ..Default::default()
    };
    let last_timestamp = read(|s| s.last_bbc_story_timestamp);

    let (response,) = http_request(request, CYCLES)
        .await
        .map_err(|err| format!("http_request failed: {:?}", err))?;

    for (timestamp, message) in parse_items(response.body)?
        .into_iter()
        .filter(|(t, _)| *t > last_timestamp)
    {
        mutate(|state| {
            schedule_message(state, message, Some("NEWS".into()));
            state.last_bbc_story_timestamp = timestamp;
        })
    }

    Ok(())
}

fn parse_items(body: Vec<u8>) -> Result<Vec<(u64, String)>, String> {
    let body = String::from_utf8(body).map_err(|err| format!("body parsing failed: {:?}", err))?;
    let doc = roxmltree::Document::parse(body.as_str())
        .map_err(|err| format!("xml parsing failed: {:?}", err))?;

    let channel = doc
        .descendants()
        .find(|n| n.tag_name().name() == "channel")
        .ok_or("no channel found")?;

    let items = channel
        .descendants()
        .filter(|n| n.tag_name().name() == "item")
        .map(|item| {
            let get = |name| {
                item.descendants()
                    .find(|n| n.tag_name().name() == name)
                    .map(|n| n.text())
                    .flatten()
                    .map(|t| t.trim())
                    .unwrap_or_default()
            };

            let title = get("title");
            let description = get("description");
            let link = get("link");
            let timestamp = DateTime::parse_from_rfc2822(get("pubDate"))
                .map(|t| t.timestamp() as u64)
                .unwrap_or_default();

            (timestamp, format!("[{title}]({link}): {description}  #BBC"))
        })
        .collect();

    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::parse_items;

    #[test]
    fn test_parsing() {
        let data = "
            <rss xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:content=\"http://purl.org/rss/1.0/modules/content/\" xmlns:atom=\"http://www.w3.org/2005/Atom\" xmlns:media=\"http://search.yahoo.com/mrss/\" version=\"2.0\">
<div id=\"in-page-channel-node-id\" data-channel-name=\"in_page_channel_pcbfPf\"/>
<channel>
<title>
<![CDATA[ BBC News ]]>
</title>
<description>
<![CDATA[ BBC News - News Front Page ]]>
</description>
<link>https://www.bbc.co.uk/news</link>
<image>
<url>https://news.bbcimg.co.uk/nol/shared/img/bbc_news_120x60.gif</url>
<title>BBC News</title>
<link>https://www.bbc.co.uk/news</link>
</image>
<generator>RSS for Node</generator>
<lastBuildDate>Tue, 29 Apr 2025 07:40:29 GMT</lastBuildDate>
<atom:link href=\"https://feeds.bbci.co.uk/news/rss.xml\" rel=\"self\" type=\"application/rss+xml\"/>
<copyright>
<![CDATA[ Copyright: (C) British Broadcasting Corporation, see https://www.bbc.co.uk/usingthebbc/terms-of-use/#15metadataandrssfeeds for terms and conditions of reuse. ]]>
</copyright>
<language>
<![CDATA[ en-gb ]]>
</language>
<ttl>15</ttl>
<item>
<title>
<![CDATA[ Trump made Carney's turnaround victory possible ]]>
</title>
<description>
<![CDATA[ Mark Carney's party pull off an election win that once looked near-impossible, until the US president targeted Canada. ]]>
</description>
<link>https://www.bbc.com/news/articles/c5ypz7yx73wo</link>
<guid isPermaLink=\"false\">https://www.bbc.com/news/articles/c5ypz7yx73wo#0</guid>
<pubDate>Tue, 29 Apr 2025 06:30:58 GMT</pubDate>
<media:thumbnail width=\"240\" height=\"135\" url=\"https://ichef.bbci.co.uk/ace/standard/240/cpsprodpb/10a4/live/d92f8c40-24c3-11f0-b26b-ab62c890638b.jpg\"/>
</item>
<item>
<title>
<![CDATA[ Prince Andrew's firm linked to controversial PPE millionaire ]]>
</title>
<description>
<![CDATA[ Documents show Doug Barrowman-linked company owned the prince's start-up competition for two years. ]]>
</description>
<link>https://www.bbc.com/news/articles/c9vep0p877wo</link>
<guid isPermaLink=\"false\">https://www.bbc.com/news/articles/c9vep0p877wo#0</guid>
<pubDate>Tue, 29 Apr 2025 05:13:43 GMT</pubDate>
<media:thumbnail width=\"240\" height=\"135\" url=\"https://ichef.bbci.co.uk/ace/standard/240/cpsprodpb/b279/live/2214b920-245c-11f0-8f57-b7237f6a66e6.jpg\"/>
</item>
</channel>
</rss>";

        match parse_items(data.as_bytes().into_iter().cloned().collect()) {
            Err(err) => unreachable!("unexpected error: {}", err),
            Ok(items) => {
                dbg!(&items);
                assert_eq!(items.len(), 2);
            }
        }
    }
}
