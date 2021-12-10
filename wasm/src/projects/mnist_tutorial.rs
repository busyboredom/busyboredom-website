use wasm_bindgen::prelude::*;

use crate::{active_tab, goto_page};

#[wasm_bindgen]
pub async fn mnist_tutorial() {
    // Set active tab.
    active_tab("");

    // Go to the page.
    goto_page(
        "/projects/mnist-tutorial",
        "/api/projects/mnist_tutorial/mnist_tutorial.html?ver=nf3vkHIeDIY",
        "MNIST Tutorial",
    )
    .await;
}
