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
