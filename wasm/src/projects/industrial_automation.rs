use wasm_bindgen::prelude::*;

use crate::{active_tab, goto_page};

#[wasm_bindgen]
pub async fn industrial_automation() {
    // Set active tab.
    active_tab("");

    // Go to the page.
    goto_page(
        "/projects/industrial_automation",
        "/api/projects/industrial_automation/industrial_automation.html",
        "Industrial Automation",
    )
    .await;
}
