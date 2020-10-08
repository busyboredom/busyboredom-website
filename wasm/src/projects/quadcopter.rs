use wasm_bindgen::prelude::*;

use crate::{active_tab, goto_page};

#[wasm_bindgen]
pub async fn quadcopter() {
    // Set active tab.
    active_tab("");

    // Go to the page.
    goto_page(
        "/projects/quadcopter",
        "/api/projects/quadcopter/quadcopter.html",
        "Quadcopter",
    )
    .await;
}
