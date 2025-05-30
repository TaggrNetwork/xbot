use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    time::Duration,
};

use candid::{CandidType, Principal};
use ic_cdk::{
    api::{call::CallResult, stable},
    spawn,
};
use ic_cdk_timers::set_timer_interval;
use serde::{Deserialize, Serialize};

thread_local! {
    static STATE: RefCell<State> = Default::default();
}

fn read<F, R>(f: F) -> R
where
    F: FnOnce(&State) -> R,
{
    STATE.with(|cell| f(&cell.borrow()))
}

fn mutate<F, R>(f: F) -> R
where
    F: FnOnce(&mut State) -> R,
{
    STATE.with(|cell| f(&mut cell.borrow_mut()))
}

const POSTING_FREQ_MIN: u64 = 15;
const MAX_MSG_MEMORY: usize = 500;

mod hackernews;
mod modulation;
mod rss;
mod watcherguru;
mod whalealert;

#[derive(Default, CandidType, Serialize, Deserialize)]
pub struct State {
    pub message_queue: VecDeque<(String, Option<String>)>,
    pub logs: VecDeque<String>,
    pub last_block: u64,
    pub last_best_story: u64,
    pub last_rss_story_timestamp: HashMap<String, u64>,
    pub modulation: i32,
    pub seen_messages: VecDeque<String>,
}

fn schedule_message<T: ToString>(state: &mut State, body: T, realm: Option<String>) {
    let msg = body.to_string();
    if state.seen_messages.contains(&msg) {
        return;
    }
    state.message_queue.push_back((msg.clone(), realm));
    state.seen_messages.push_front(msg);
    while state.seen_messages.len() > MAX_MSG_MEMORY {
        state.seen_messages.pop_back();
    }
}

async fn send_message<T: ToString>(body: T, realm: Option<String>) -> Result<u64, String> {
    let blobs: Vec<(String, Vec<u8>)> = Default::default();
    let parent: Option<u64> = None;
    let poll: Option<Vec<u8>> = None;
    let result: CallResult<(Result<u64, String>,)> = ic_cdk::call(
        Principal::from_text("6qfxa-ryaaa-aaaai-qbhsq-cai").unwrap(),
        "add_post",
        (body.to_string(), blobs, parent, realm, poll),
    )
    .await;
    result
        .map_err(|err| format!("{:?}", err))
        .and_then(|(val,)| val)
}

// CANISTER METHODS

#[ic_cdk_macros::query]
fn info(opcode: String) -> Vec<String> {
    if opcode == "logs" {
        read(|state| state.logs.iter().cloned().collect::<Vec<_>>())
    } else {
        read(|s| {
            vec![
                format!("Logs: {}", s.logs.len(),),
                format!("LastBlock: {}", s.last_block,),
                format!("Modulation: {}", s.modulation,),
                format!("LastBestStory: {}", s.last_best_story),
                format!("LastRSSTimestamps: {:?}", &s.last_rss_story_timestamp),
                format!("SeenMessages={}", s.seen_messages.len(),),
                format!(
                    "Message Queue ({}): {:?}",
                    s.message_queue.len(),
                    &s.message_queue
                ),
            ]
        })
    }
}

fn set_timer() {
    let _id = set_timer_interval(Duration::from_secs(4 * 60 * 60), || spawn(hourly_tasks()));
    let _id = set_timer_interval(Duration::from_secs(24 * 60 * 60), || spawn(daily_tasks()));
    // We're sending one message per half an hour at most
    let _id = set_timer_interval(Duration::from_secs(60 * POSTING_FREQ_MIN), || {
        spawn(process_one_message())
    });
}

async fn daily_tasks() {
    mutate(|state| {
        let logs = &mut state.logs;
        while logs.len() > 500 {
            logs.pop_front();
        }
    });
    log_if_error(modulation::go().await);
    log_if_error(hackernews::go().await);
}

async fn hourly_tasks() {
    log_if_error(watcherguru::go().await);
    log_if_error(rss::go("BBC", "https://feeds.bbci.co.uk/news/world/rss.xml", "NEWS").await);
    log_if_error(
        rss::go(
            "CoinTelegraph",
            "https://cointelegraph.com/editors_pick_rss",
            "CRYPTO",
        )
        .await,
    );
    log_if_error(whalealert::go().await);
}

async fn process_one_message() {
    let Some((message, realm)) = mutate(|state| state.message_queue.pop_front()) else {
        return;
    };
    if let Err(err) = send_message(&message, realm.clone()).await {
        mutate(|state| {
            state
                .logs
                .push_back(format!("Taggr response to message {}: {:?}", message, err));
            state.message_queue.push_front((message, realm));
        })
    }
}

#[ic_cdk_macros::init]
fn init() {
    set_timer();
}

#[ic_cdk_macros::update]
async fn fixture() {}

// #[ic_cdk_macros::update]
// async fn update_state() {
//     hourly_tasks().await;
//     daily_tasks().await;
//     mutate(|s| {
//         s.last_block = 23036000;
//         s.message_queue.clear();
//     });
// }

#[ic_cdk_macros::pre_upgrade]
fn pre_upgrade() {
    let buffer: Vec<u8> = read(|s| bincode::serialize(s)).expect("couldn't serialize the state");
    let writer = &mut stable::StableWriter::default();
    let _ = writer.write(&buffer);
}

#[ic_cdk_macros::post_upgrade]
fn post_upgrade() {
    let bytes = stable::stable_bytes();
    let state: State = bincode::deserialize(&bytes).unwrap();
    STATE.with(|cell| cell.replace(state));
    set_timer();
}

fn log_if_error<T>(result: Result<T, String>) {
    if let Err(err) = result {
        mutate(|state| state.logs.push_back(format!("Error: {}", err)))
    }
}
