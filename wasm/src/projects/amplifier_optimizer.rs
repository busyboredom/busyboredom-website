use wasm_bindgen::prelude::*;

use crate::{active_tab, goto_page};

#[wasm_bindgen]
pub async fn amplifier_optimizer() {
    // Set active tab.
    active_tab("");

    // Go to the page.
    goto_page(
        "/projects/amplifier_optimizer",
        "/api/projects/amplifier_optimizer/amplifier_optimizer.html",
        "Amplifier Optimizer",
    )
    .await;
}
