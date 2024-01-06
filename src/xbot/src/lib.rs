use std::{collections::VecDeque, time::Duration};

use candid::{CandidType, Principal};
use ic_cdk::{
    api::{call::CallResult, stable},
    spawn,
};
use ic_cdk_timers::set_timer_interval;
use serde::{Deserialize, Serialize};

mod hackernews;
mod modulation;
mod watcherguru;
mod whalealert;

#[derive(Default, CandidType, Serialize, Deserialize)]
pub struct State {
    pub message_queue: VecDeque<(String, Option<String>)>,
    pub logs: VecDeque<String>,
    pub last_block: u64,
    pub last_best_story: u64,
    pub last_best_post: String,
    pub modulation: i32,
    pub last_wg_message: String,
}

static mut STATE: Option<State> = None;

fn schedule_message<T: ToString>(body: T, realm: Option<String>) {
    state_mut()
        .message_queue
        .push_back((body.to_string(), realm));
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

fn state() -> &'static State {
    unsafe { STATE.as_ref().expect("No state was initialized") }
}

fn state_mut() -> &'static mut State {
    unsafe { STATE.as_mut().expect("No state was initialized") }
}

// CANISTER METHODS

#[ic_cdk_macros::query]
fn info(opcode: String) -> Vec<String> {
    if opcode == "logs" {
        state().logs.iter().cloned().collect::<Vec<_>>()
    } else {
        let s = state();
        vec![
            format!("Logs: {}", s.logs.len(),),
            format!("LastBlock: {}", s.last_block,),
            format!("Modulation: {}", s.modulation,),
            format!("LastBestStory: {}", s.last_best_story,),
            format!("LastWGMsg: {}", s.last_wg_message),
            format!("Message Queue: {}", s.message_queue.len()),
        ]
    }
}

fn set_timer() {
    let _id = set_timer_interval(Duration::from_secs(4 * 60 * 60), || spawn(hourly_tasks()));
    let _id = set_timer_interval(Duration::from_secs(24 * 60 * 60), || spawn(daily_tasks()));
}

async fn daily_tasks() {
    let logs = &mut state_mut().logs;
    while logs.len() > 500 {
        logs.pop_front();
    }
    modulation::go().await;
    hackernews::go().await;
}

async fn hourly_tasks() {
    watcherguru::go().await;
    whalealert::go().await;
    for _ in 0..10 {
        if let Some((message, realm)) = state_mut().message_queue.pop_front() {
            if let Err(err) = send_message(&message, realm.clone()).await {
                let logs = &mut state_mut().logs;
                logs.push_back(format!("Taggr response to message {}: {:?}", message, err));
                state_mut().message_queue.push_front((message, realm));
            }
        }
    }
}

#[ic_cdk_macros::init]
fn init() {
    let state: State = State {
        last_block: 6557316,
        ..Default::default()
    };
    unsafe {
        STATE = Some(state);
    }
    set_timer();
}

// #[ic_cdk_macros::update]
// async fn test() {}

#[ic_cdk_macros::pre_upgrade]
fn pre_upgrade() {
    let buffer: Vec<u8> = bincode::serialize(state()).expect("couldn't serialize the state");
    let writer = &mut stable::StableWriter::default();
    let _ = writer.write(&buffer);
}

#[ic_cdk_macros::post_upgrade]
fn post_upgrade() {
    let bytes = stable::stable_bytes();
    let state: State = bincode::deserialize(&bytes).unwrap();
    unsafe {
        STATE = Some(state);
    }
    set_timer();
}
