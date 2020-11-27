use wasm_bindgen::prelude::*;

use crate::{active_tab, goto_page};

#[wasm_bindgen]
pub async fn industrial_automation() {
    // Set active tab.
    active_tab("");

    // Go to the page.
    goto_page(
        "/projects/industrial-automation",
        "/api/projects/industrial_automation/industrial_automation.html?ver=TeQyXW4Q1R8",
        "Industrial Automation",
    )
    .await;
}
