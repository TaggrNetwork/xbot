use crate::{mutate, read};

use super::schedule_message;
use ic_ledger_types::MAINNET_CYCLES_MINTING_CANISTER_ID;

pub async fn go() -> Result<(), String> {
    let modulation = read(|s| s.modulation);
    let (response,): (Result<i32, String>,) = ic_cdk::call(
        MAINNET_CYCLES_MINTING_CANISTER_ID,
        "neuron_maturity_modulation",
        ((),),
    )
    .await
    .map_err(|err| format!("couldn't call cmc: {:?}", err))?;
    let new_modulation = response.expect("couldn't get the modulation");
    mutate(|state| {
        state.modulation = new_modulation;
        state
            .logs
            .push_back(format!("Modulation: {} -> {}", modulation, new_modulation));
    });
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
        return Ok(());
    };
    mutate(|s| schedule_message(s, message, None));
    Ok(())
}
