use super::{post_to_taggr, state, state_mut};
use ic_ledger_types::MAINNET_CYCLES_MINTING_CANISTER_ID;

pub async fn go() {
    let modulation = state().modulation;
    let (response,): (Result<i32, String>,) = ic_cdk::call(
        MAINNET_CYCLES_MINTING_CANISTER_ID,
        "neuron_maturity_modulation",
        ((),),
    )
    .await
    .expect("couldn't call cmc");
    let new_modulation = response.expect("couldn't get the modulation");
    let state = state_mut();
    state.modulation = new_modulation;
    state
        .logs
        .push(format!("Modulation: {} -> {}", modulation, new_modulation));
    let message = if new_modulation > 0
        && new_modulation > modulation
        && (modulation <= 0 || new_modulation / 100 > modulation / 100)
    {
        let rockets = (0..new_modulation / 100)
            .map(|_| "🚀".to_string())
            .collect::<Vec<_>>()
            .join("");
        format!(
            "📈 The neuron maturity #modulation is now `{}` {}",
            100.0 + (new_modulation as f32 / 100.0),
            rockets
        )
    } else if new_modulation < 0 && modulation >= 0 {
        "📉 The neuron maturity #modulation is now below `100` ".to_owned()
    } else {
        return;
    };
    post_to_taggr(message, None).await;
}
