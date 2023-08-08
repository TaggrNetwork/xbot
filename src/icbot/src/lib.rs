use std::time::Duration;

use candid::{CandidType, Principal};
use ic_cdk::{
    api::{call::CallResult, stable},
    spawn,
};
use ic_cdk_timers::set_timer_interval;
use serde::{Deserialize, Serialize};

mod hackernews;
mod modulation;
mod whalealert;

#[derive(Default, CandidType, Serialize, Deserialize)]
pub struct State {
    pub logs: Vec<String>,
    pub last_block: u64,
    #[serde(default)]
    pub last_best_story: u64,
    pub modulation: i32,
}

static mut STATE: Option<State> = None;

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

fn state() -> &'static State {
    unsafe { STATE.as_ref().expect("No state was initialized") }
}

fn state_mut() -> &'static mut State {
    unsafe { STATE.as_mut().expect("No state was initialized") }
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
    let _id = set_timer_interval(Duration::from_secs(60 * 60), || spawn(whalealert::go()));
    let _id = set_timer_interval(Duration::from_secs(24 * 60 * 60), || spawn(daily_tasks()));
}

async fn daily_tasks() {
    modulation::go().await;
    hackernews::go().await;
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
