use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{HtmlInputElement, Request, RequestInit, Response};
use js_sys::Date;

use crate::{active_tab, goto_page};

#[wasm_bindgen]
pub async fn contact() {
    // Set active tab.
    active_tab("contact");

    // Go to the page.
    goto_page("/contact", "/api/contact.html?ver=Xed1_vkE8sI", "Contact").await;
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

#[wasm_bindgen]
pub async fn captcha_submit() {
    let window = web_sys::window().expect("No global `window` exists");
    let document = window.document().expect("Should have a document on window");

    let input_js: JsValue = document
        .get_element_by_id("captcha-chars")
        .expect("Could not find element 'captcha-chars'")
        .into();
    let captcha_input: HtmlInputElement = input_js.into();
    let guess = captcha_input.value();

    let mut req = RequestInit::new();
    req.method("GET");
    let request_string = format!("/api/submit_captcha?captcha={}", guess);
    let request = Request::new_with_str_and_init(&request_string, &req)
        .expect("Request could not be created");
    request
        .headers()
        .set("Accept", "text/plain")
        .expect("Headers could not be set");

    let response = JsFuture::from(window.fetch_with_request(&request))
        .await
        .expect("Could not cast response as JsFuture");

    // `response` is a `Response` object.
    assert!(response.is_instance_of::<Response>());
    let resp: Response = response.dyn_into().unwrap();

    // Convert this other `Promise` into a rust `Future`.
    let response_content = JsFuture::from(resp.text().unwrap())
        .await
        .unwrap()
        .as_string()
        .unwrap();

    if response_content == "Pass" {
        // Hide captcha control stuff.
        document
            .get_element_by_id("captcha-buttons")
            .expect("Could not find element 'captcha-pass'")
            .remove();
        document
            .get_element_by_id("captcha-chars")
            .expect("Could not find element 'captcha-pass'")
            .set_attribute("hidden", "true")
            .expect("Hidden attribute could not be set");
        // Show Pass Checkmark.
        document
            .get_element_by_id("captcha-pass")
            .expect("Could not find element 'captcha-pass'")
            .remove_attribute("hidden")
            .expect("Hidden attribute not present");
        // Show submit button.
        document
        .get_element_by_id("submit")
        .expect("Could not find element 'captcha-pass'")
        .remove_attribute("hidden")
        .expect("Hidden attribute not present");
    } else {
        // Show try again.
        document
            .get_element_by_id("try-again")
            .expect("Could not find element 'captcha-pass'")
            .remove_attribute("hidden")
            .expect("Hidden attribute not present");
    }
}

#[wasm_bindgen]
pub async fn captcha_refresh() {
    let window = web_sys::window().expect("No global `window` exists");
    let document = window.document().expect("Should have a document on window");

    let url = format!("/api/generate_captcha?time={}", Date::now());
    document
        .get_element_by_id("captcha-png")
        .expect("Could not find element 'captcha-pass'")
        .set_attribute("src", &url)
        .expect("Could not set hidden attribute");
}