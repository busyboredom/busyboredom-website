use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response};

use crate::active_tab;

#[wasm_bindgen]
pub async fn amplifier_optimizer() {
    active_tab("");

    let window = web_sys::window().expect("No global `window` exists");
    let document = window.document().expect("Should have a document on window");
    let history = window.history().expect("Could not get history");

    let mut req = RequestInit::new();
    req.method("GET");
    let request = Request::new_with_str_and_init(
        "/api/projects/amplifier_optimizer/amplifier_optimizer.html",
        &req,
    )
    .expect("Request could not be created");
    request
        .headers()
        .set("Accept", "text/html")
        .expect("Headers could not be set");

    let response = JsFuture::from(window.fetch_with_request(&request))
        .await
        .expect("Could not unwrap response");

    // `response` is a `Response` object.
    assert!(response.is_instance_of::<Response>());
    let resp: Response = response.dyn_into().unwrap();

    // Convert this other `Promise` into a rust `Future`.
    let page = JsFuture::from(resp.text().unwrap())
        .await
        .unwrap()
        .as_string()
        .unwrap();

    // Show the new content.
    document
        .get_element_by_id("page")
        .unwrap()
        .set_inner_html(&page);

    // Close the project dropdown menu.
    let dropdown = document
        .get_element_by_id("projects_dropdown")
        .expect("Could not get 'dropdown' element");
    let drop_symbol = document
        .get_element_by_id("drop_symbol")
        .expect("Could not get 'drop_symbol' element");

    dropdown.set_class_name("dropdown-content");
    drop_symbol.set_class_name("arrow down");

    // Remove the history entry pushed on page load, and replace it.
    if history.state().expect("Could not get history state") != "/projects/amplifier_optimizer" {
        history
            .push_state_with_url(
                &JsValue::from_str("/projects/amplifier_optimizer"),
                "Amplifier Optimizer",
                Some("/projects/amplifier_optimizer"),
            )
            .expect("Could not push state to history");
    }

    document.set_title("Amplifier Optimizer");
}
