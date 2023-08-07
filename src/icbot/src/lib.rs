use accounts::get_accounts;
use candid::{CandidType, Principal};
use ic_cdk::{
    api::{call::CallResult, stable},
    spawn,
};
use ic_cdk_timers::set_timer_interval;
use ic_ledger_types::{
    Operation, Tokens, MAINNET_CYCLES_MINTING_CANISTER_ID, {GetBlocksArgs, QueryBlocksResponse},
};
use num_format::{Locale, ToFormattedString};
use serde::{Deserialize, Serialize};
use std::time::Duration;

mod accounts;

#[derive(Default, CandidType, Serialize, Deserialize)]
pub struct State<'a> {
    pub logs: Vec<String>,
    pub last_block: u64,
    pub modulation: i32,
    #[serde(skip)]
    pub address_book: std::collections::HashMap<&'a str, &'a str>,
}

static mut STATE: Option<State> = None;

const WHALE_ALERT: Tokens = Tokens::from_e8s(10000000000000);

async fn post_to_taggr<T: ToString>(body: T, realm: Option<String>) -> String {
    let blobs: Vec<(String, Vec<u8>)> = Default::default();
    let parent: Option<u64> = None;
    let poll: Option<Vec<u8>> = None;
    let result: CallResult<(Result<u64, String>,)> = ic_cdk::call(
        Principal::from_text("6qfxa-ryaaa-aaaai-qbhsq-cai").unwrap(),
        "add_post",
        (body.to_string(), blobs, parent, realm, poll),
    )
    .await;
    format!("{:?}", result)
}

fn state<'a>() -> &'a State<'static> {
    unsafe { STATE.as_ref().expect("No state was initialized") }
}

fn state_mut<'a>() -> &'a mut State<'static> {
    unsafe { STATE.as_mut().expect("No state was initialized") }
}

async fn fetch_modulation() {
    let modulation = state().modulation;
    let (response,): (Result<i32, String>,) = ic_cdk::call(
        MAINNET_CYCLES_MINTING_CANISTER_ID,
        "neuron_maturity_modulation",
        ((),),
    )
    .await
    .expect("couldn't call cmc");
    let new_modulation = response.expect("couldn't get the modulation");
    state_mut().modulation = new_modulation;
    state_mut()
        .logs
        .push(format!("New modulation: {}", new_modulation));
    let message = if new_modulation > modulation
        && (modulation <= 0 || new_modulation / 100 > modulation / 100)
    {
        let rockets = (0..new_modulation / 100)
            .map(|_| "🚀".to_string())
            .collect::<Vec<_>>()
            .join("");
        format!(
            "The neuron maturity #modulation is now {}! 📈{}",
            new_modulation, rockets
        )
    } else if new_modulation < 0 && modulation >= 0 {
        "The neuron maturity #modulation is now below 100. 📉".to_owned()
    } else {
        return;
    };
    post_to_taggr(message, None).await;
}

async fn fetch_txs() {
    let mut total_blocks = 0;
    let mut max_amount = 0;
    loop {
        let start = state().last_block;
        let args = GetBlocksArgs {
            start,
            length: 1000,
        };
        let icp = |tokens: Tokens| {
            (tokens.e8s() / Tokens::SUBDIVIDABLE_BY).to_formatted_string(&Locale::de_CH)
        };
        let (response,): (QueryBlocksResponse,) = ic_cdk::call(
            Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai")
                .expect("couldn't parse the ledger canister id"),
            "query_blocks",
            (args,),
        )
        .await
        .expect("couldn't call ledger");
        state_mut().last_block = response.first_block_index + response.blocks.len() as u64;
        total_blocks += response.blocks.len();
        let address_book = &state().address_book;
        let resolver = |acc: &str| {
            address_book
                .get(acc)
                .unwrap_or(&acc[0..6].to_string().as_str())
                .to_string()
        };
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
            post_to_taggr(full_msg.clone(), Some("TAGGR".into())).await;
            state_mut().logs.push(full_msg);
        }
        if response.blocks.len() < 50 {
            break;
        }
    }
    state_mut().logs.push(format!(
        "Total transactions pulled: {} (max e8s: {})",
        total_blocks, max_amount
    ));
}

// CANISTER METHODS

#[ic_cdk_macros::query]
fn info(opcode: String) -> String {
    if opcode == "logs" {
        state().logs.join("\n")
    } else {
        let s = state();
        format!(
            "Logs={}, LastBlock={}, Modulation={}",
            s.logs.len(),
            s.last_block,
            s.modulation,
        )
    }
}

fn set_timer() {
    let _id = set_timer_interval(Duration::from_secs(60 * 60), || spawn(fetch_txs()));
    let _id = set_timer_interval(Duration::from_secs(24 * 60 * 60), || {
        spawn(fetch_modulation())
    });
}

#[ic_cdk_macros::init]
fn init() {
    let state: State = State {
        address_book: get_accounts(),
        last_block: 6557316,
        ..Default::default()
    };
    unsafe {
        STATE = Some(state);
    }
    set_timer();
}

#[ic_cdk_macros::pre_upgrade]
fn pre_upgrade() {
    let buffer: Vec<u8> = bincode::serialize(state()).expect("couldn't serialize the state");
    let writer = &mut stable::StableWriter::default();
    let _ = writer.write(&buffer);
}

#[ic_cdk_macros::post_upgrade]
fn post_upgrade() {
    let bytes = stable::stable_bytes();
    let mut state: State = bincode::deserialize(&bytes).unwrap();
    state.address_book = get_accounts();
    unsafe {
        STATE = Some(state);
    }
    set_timer();
}
