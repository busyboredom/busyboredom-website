use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Request, RequestInit, Response};

// Called when the wasm module is instantiated
#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    init_panic_hook();

    // Use `web_sys`'s global `window` function to get a handle on the global
    // window object.
    let window = web_sys::window().expect("no global `window` exists");

    // Get current URL and load the resulting page.
    route(&window.location().pathname().unwrap()[..]);

    Ok(())
}

#[wasm_bindgen]
pub fn active_tab(tab: &str) {
    let window = web_sys::window().expect("No global `window` exists");
    let document = window.document().expect("Should have a document on window");
    let tabs = document.get_elements_by_class_name("tab");

    for index in 0..tabs.length() {
        let element = tabs.item(index).unwrap();
        if element.id() == tab {
            element.set_class_name("tab active");
        } else {
            element.set_class_name("tab");
        }
    }
}

#[wasm_bindgen]
pub fn nav_expand() {
    let window = web_sys::window().expect("No global `window` exists");
    let document = window.document().expect("Should have a document on window");
    let nav = document
        .get_element_by_id("nav")
        .expect("Could not get 'nav' element");

    if nav.class_name() == "nav" {
        nav.set_class_name("nav responsive");
    } else {
        nav.set_class_name("nav");
    }
}

#[wasm_bindgen]
pub async fn resume() {
    //Set active tab
    active_tab("resume");

    let window = web_sys::window().expect("No global `window` exists");
    let document = window.document().expect("Should have a document on window");
    let history = window.history().expect("Could not get history");

    let mut req = RequestInit::new();
    req.method("GET");
    let request =
        Request::new_with_str_and_init("/api/resume", &req).expect("Request could not be created");
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

    // Remove the history entry pushed on page load, and replace it.
    if history.state().expect("Could not get history state") != "/resume" {
        history
            .push_state_with_url(&JsValue::from_str("/resume"), "Résumé", Some("/resume"))
            .expect("Could not push state (with URL) to history");
    }

    document.set_title("Resume");
}

#[wasm_bindgen]
pub async fn welcome() {
    // Set active tab
    active_tab("");
    let window = web_sys::window().expect("No global `window` exists");
    let document = window.document().expect("Should have a document on window");
    let history = window.history().expect("Could not get history");

    let mut req = RequestInit::new();
    req.method("GET");
    let request =
        Request::new_with_str_and_init("/api/welcome", &req).expect("Request could not be created");
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

    // Remove the history entry pushed on page load, and replace it.
    if history.state().expect("Could not get history state") != "/welcome" {
        history
            .push_state_with_url(&JsValue::from_str("/welcome"), "Welcome!", Some("/welcome"))
            .expect("Could not push state (with URL) to history");
    }

    document.set_title("Welcome!");
}

#[wasm_bindgen]
pub async fn contact() {
    // Set active tab
    active_tab("contact");
    let window = web_sys::window().expect("No global `window` exists");
    let document = window.document().expect("Should have a document on window");
    let history = window.history().expect("Could not get history");

    let mut req = RequestInit::new();
    req.method("GET");
    let request =
        Request::new_with_str_and_init("/api/contact", &req).expect("Request could not be created");
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

    // Remove the history entry pushed on page load, and replace it.
    if history.state().expect("Could not get history state") != "/contact" {
        history
            .push_state_with_url(&JsValue::from_str("/contact"), "Contact", Some("/contact"))
            .expect("Could not push state to history");
    }

    document.set_title("Contact");
}

#[wasm_bindgen]
pub async fn coming_soon() {
    // Set active tab
    active_tab("");
    let window = web_sys::window().expect("No global `window` exists");
    let document = window.document().expect("Should have a document on window");
    let history = window.history().expect("Could not get history");

    let mut req = RequestInit::new();
    req.method("GET");
    let request = Request::new_with_str_and_init("/api/coming_soon", &req)
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

    // Remove the history entry pushed on page load, and replace it.
    if history.state().expect("Could not get history state") != "/coming_soon" {
        history
            .push_state(&JsValue::from_str("/coming_soon"), "Coming Soon!")
            .expect("Could not push state to history");
    }

    document.set_title("Coming Soon!!");
}

#[wasm_bindgen]
pub async fn error_404() {
    let window = web_sys::window().expect("No global `window` exists");
    let document = window.document().expect("Should have a document on window");
    let history = window.history().expect("Could not get history");

    let mut req = RequestInit::new();
    req.method("GET");
    let request = Request::new_with_str_and_init("/api/error_404", &req)
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

    // Remove the history entry pushed on page load, and replace it.
    if history.state().expect("Could not get history state") != "/error_404" {
        history
            .push_state(&JsValue::from_str("/error_404"), "404: Page Not Found")
            .expect("Could not push state to history");
    }

    document.set_title("404: Page Not Found");
}

/// Get current URL and load the resulting page.
#[wasm_bindgen]
pub fn route(rt: &str) {
    match rt {
        "/" => spawn_local(welcome()),
        "/welcome" => spawn_local(welcome()),
        "/resume" => spawn_local(resume()),
        "/contact" => spawn_local(contact()),
        _ => spawn_local(error_404()),
    }
}

#[wasm_bindgen]
pub fn init_panic_hook() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();
}
