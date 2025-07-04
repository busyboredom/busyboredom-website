use wasm_bindgen::prelude::*;

use crate::{active_tab, goto_page};

#[wasm_bindgen]
pub async fn this_website() {
    // Set active tab.
    active_tab("");

    // Go to the page.
    goto_page(
        "/projects/this-website",
        "/api/projects/this_website/this_website.html?ver=zL90Ot_j8Nw",
        "This Website",
    )
    .await;
}
