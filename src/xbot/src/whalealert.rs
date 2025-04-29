use crate::{mutate, read};

use super::schedule_message;
use ic_ledger_types::{
    Operation, Tokens, MAINNET_LEDGER_CANISTER_ID, {GetBlocksArgs, QueryBlocksResponse},
};
use num_format::{Locale, ToFormattedString};
use std::collections::HashMap;

const WHALE_ALERT: Tokens = Tokens::from_e8s(8000000000000); // 80k ICP
const BATCH_SIZE: u64 = 1000;

pub async fn go() -> Result<(), String> {
    let mut total_blocks = 0;
    let mut max_amount = 0;
    let resolver = |acc: &str| {
        get_accounts()
            .get(acc)
            .unwrap_or(&acc[0..6].to_string().as_str())
            .to_string()
    };
    let start = read(|s| s.last_block);
    let mut last_block = start;

    for step in 0..1000 {
        let start = read(|s| s.last_block);
        let args = GetBlocksArgs {
            start,
            length: BATCH_SIZE,
        };
        let (response,): (QueryBlocksResponse,) =
            ic_cdk::call(MAINNET_LEDGER_CANISTER_ID, "query_blocks", (args,))
                .await
                .map_err(|err| format!("canister call failed: {:?}", err))?;
        if step == 0 && response.blocks.is_empty() {
            mutate(|s| s.last_block = response.first_block_index);
            continue;
        }
        if (response.blocks.len() as u64) < BATCH_SIZE {
            break;
        }
        total_blocks += response.blocks.len();
        last_block = start + total_blocks as u64;
        mutate(|s| s.last_block = last_block);
        let mut msgs = Vec::new();
        for block in &response.blocks {
            if let Some(Operation::Transfer {
                from, to, amount, ..
            }) = block.transaction.operation
            {
                max_amount = max_amount.max(amount.e8s());
                if amount > WHALE_ALERT {
                    msgs.push( format!(
                    "- `{}` ICP transferred from [{}](https://dashboard.internetcomputer.org/account/{}) to [{}](https://dashboard.internetcomputer.org/account/{}).",
                    icp(amount), resolver(&from.to_string()), from, resolver(&to.to_string()), to
                ))
                }
            };
        }
        if !msgs.is_empty() {
            let full_msg = format!("🚨 #WhaleAlert\n\n{}", msgs.join("\n"));
            mutate(|s| schedule_message(s, full_msg.clone(), Some("ICP".into())));
        }
    }

    mutate(|s| {
        s.logs.push_back(format!(
        "Total transactions pulled: {} (max e8s: {}, start: {}, last_block: {}, next_block: {})",
        total_blocks,
        icp(Tokens::from_e8s(max_amount)),
        start,
        start + total_blocks as u64,
        last_block))
    });
    Ok(())
}

pub fn get_accounts<'a>() -> HashMap<&'a str, &'a str> {
    let mut map: HashMap<_, _> = Default::default();

    map.insert(
        "d3e13d4777e22367532053190b6c6ccf57444a61337e996242b1abfb52cf92c8",
        "Binance",
    );
    map.insert(
        "220c3a33f90601896e26f76fa619fe288742df1fa75426edfaf759d39f2455a5",
        "Binance",
    );
    map.insert(
        "935b1a3adc28fd68cacc95afcdec62e985244ce0cfbbb12cdc7d0b8d198b416d",
        "Huobi",
    );
    map.insert(
        "e7a879ea563d273c46dd28c1584eaa132fad6f3e316615b3eb657d067f3519b5",
        "Okex",
    );
    map.insert(
        "4dfa940def17f1427ae47378c440f10185867677109a02bc8374fc25b9dee8af",
        "Coinbase",
    );
    map.insert(
        "a6ed987d89796f921c8a49d275ec7c9aa04e75a8fc8cd2dbaa5da799f0215ab0",
        "Coinbase",
    );
    map.insert(
        "449ce7ad1298e2ed2781ed379aba25efc2748d14c60ede190ad7621724b9e8b2",
        "Coinbase",
    );
    map.insert(
        "660b1680dafeedaa68c1f1f4cf8af42ed1dfb8564646efe935a2b9a48528b605",
        "Coinbase",
    );
    map.insert(
        "dd15f3040edab88d2e277f9d2fa5cc11616ebf1442279092e37924ab7cce8a74",
        "Coinbase",
    );
    map.insert(
        "4878d23a09b554157b31323004e1cc053567671426ca4eec7b7e835db607b965",
        "Coinbase",
    );
    map.insert(
        "8fe706db7b08f957a15199e07761039a7718937aabcc0fe48bc380a4daf9afb0",
        "Gate",
    );
    map.insert(
        "efa01544f509c56dd85449edf2381244a48fad1ede5183836229c00ab00d52df",
        "KuCoin",
    );
    map.insert(
        "040834c30cdf5d7a13aae8b57d94ae2d07eefe2bc3edd8cf88298730857ac2eb",
        "Kraken",
    );
    map.insert(
        "609d3e1e45103a82adc97d4f88c51f78dedb25701e8e51e8c4fec53448aadc29",
        "Binance Cold Storage",
    );

    map
}

fn icp(tokens: Tokens) -> String {
    (tokens.e8s() / Tokens::SUBDIVIDABLE_BY).to_formatted_string(&Locale::de_CH)
}
