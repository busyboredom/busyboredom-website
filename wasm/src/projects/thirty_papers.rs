use js_sys::Promise;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::console;

use crate::{active_tab, goto_page};

// First, tell Rust about the JavaScript functions we need to call
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = loadScript, catch)]
    fn load_script(url: &str) -> Result<Promise, JsValue>;

    #[wasm_bindgen(js_name = init30PapersPage, catch)]
    fn init_30_papers_page() -> Result<(), JsValue>;
}

#[wasm_bindgen]
pub async fn thirty_papers() {
    active_tab("");
    goto_page(
        "/projects/thirty-papers",
        "/api/projects/thirty_papers/thirty_papers.html?ver=TskhaRX_9FI",
        "30 Papers in 30 Days",
    )
    .await;

    // Call the loader to fetch and execute our page-specific script
    match load_script("/api/projects/thirty_papers/thirty_papers.js?ver=fa3jrLLkOls") {
        Ok(promise) => {
            // Wait for the script to fully load
            if let Err(e) = JsFuture::from(promise).await {
                console::error_2(&"Failed to load thirty_papers.js:".into(), &e);
                return;
            }

            // The script is loaded. Now call its initialization function.
            if let Err(e) = init_30_papers_page() {
                console::error_2(&"Error running init30PapersPage:".into(), &e);
            }
        }
        Err(e) => {
            console::error_2(&"Error calling loadScript function:".into(), &e);
        }
    }
}
