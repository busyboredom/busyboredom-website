use wasm_bindgen::prelude::*;

use crate::{active_tab, goto_page};

#[wasm_bindgen]
pub async fn contact() {
    // Set active tab.
    active_tab("contact");

    // Go to the page.
    goto_page("/contact", "/api/contact.html?ver=jVzO5FIt2hU", "Contact").await;
}

#[wasm_bindgen]
pub fn contact_submit() {
    let window = web_sys::window().expect("No global `window` exists");
    let document = window.document().expect("Should have a document on window");

    // Remove submit button.
    document
        .get_element_by_id("submit")
        .expect("Could not get element with id 'submit'")
        .remove();

    // Show loading text.
    let loading = document
        .get_element_by_id("contact-loading")
        .expect("Could not get element with id 'contact-loading'");
    loading.set_class_name("contact-loading show");
}

#[wasm_bindgen]
pub async fn contact_submitted() {
    // Set active tab.
    active_tab("contact");

    // Go to the page.
    goto_page(
        "/contact-submitted",
        "/api/contact_submitted.html?ver=ypBIrFi5QPY",
        "Submitted",
    )
    .await;
}
