#![warn(clippy::pedantic)]
#![warn(clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_panics_doc)]

use js_sys::Date;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{HtmlInputElement, Request, RequestInit, Response};

use crate::{active_tab, goto_page};

#[wasm_bindgen]
pub async fn contact() {
    // Set active tab.
    active_tab("contact");

    // Go to the page.
    goto_page("/contact", "/api/contact.html?ver=dIqPU546Qj4", "Contact").await;
}

#[wasm_bindgen]
pub async fn contact_info() {
    let window = web_sys::window().expect("No global `window` exists");
    let document = window.document().expect("Should have a document on window");

    // Retrieve selected contact method.
    let input_js: JsValue = document
        .get_element_by_id("info-selector")
        .expect("Could not find element 'info-selector'")
        .into();
    let info_selector: HtmlInputElement = input_js.into();
    let selected = info_selector.value();

    let contact_info;

    if selected == "Phone" {
        contact_info = "Dude, it's ".to_owned() + &Date::new_0().get_full_year().to_string() + ".";
    } else if selected == "Select" {
        contact_info = String::new();
    } else {
        let req = RequestInit::new();
        req.set_method("GET");
        let request_string = format!("/api/contact_info?method={selected}");
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
        contact_info = JsFuture::from(resp.text().unwrap())
            .await
            .unwrap()
            .as_string()
            .unwrap();
    }

    // Show contact info.
    let text = document
        .get_element_by_id("info-text")
        .expect("Could not get element with id 'info-text'");
    text.set_inner_html(&contact_info);
}

#[wasm_bindgen]
pub async fn contact_copy() {
    let window = web_sys::window().expect("No global `window` exists");
    let document = window.document().expect("Should have a document on window");
    let navigator = window.navigator();

    // Get the text.
    let text = document
        .get_element_by_id("info-text")
        .expect("Could not get element with id 'submit'")
        .inner_html();

    if text.is_empty() {
        return;
    }

    let clipboard = navigator.clipboard();
    let mut feedback = "Error!"; // Default to error.

    let copy_promise = clipboard.write_text(&text);
    // Convert this `Promise` into a rust `Future`.
    if JsFuture::from(copy_promise).await.is_ok() {
        feedback = "Copied!";
    }

    // Show copied.
    let copy_button = document
        .get_element_by_id("copy-info")
        .expect("Could not get element with id 'copy-info'");
    copy_button.set_inner_html(feedback);

    window
        .set_timeout_with_str_and_timeout_and_unused_0("window.busy.contact_copy_reset()", 1000)
        .expect("Could not set timeout for copy feedback");
}

#[wasm_bindgen]
pub fn contact_copy_reset() {
    let window = web_sys::window().expect("No global `window` exists");
    let document = window.document().expect("Should have a document on window");

    // Show copied.
    let copy_button = document
        .get_element_by_id("copy-info")
        .expect("Could not get element with id 'copy-info'");
    copy_button.set_inner_html("Copy");
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

    let req = RequestInit::new();
    req.set_method("GET");
    let request_string = format!("/api/submit_captcha?captcha={guess}");
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
pub fn captcha_refresh() {
    let window = web_sys::window().expect("No global `window` exists");
    let document = window.document().expect("Should have a document on window");

    let url = format!("/api/generate_captcha?time={}", Date::now());
    document
        .get_element_by_id("captcha-png")
        .expect("Could not find element 'captcha-pass'")
        .set_attribute("src", &url)
        .expect("Could not set hidden attribute");
}
