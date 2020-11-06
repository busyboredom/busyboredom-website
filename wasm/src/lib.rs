pub mod projects;

use projects::*;

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
pub fn nav_toggle() {
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
pub fn close_dropdowns() {
    let window = web_sys::window().expect("No global `window` exists");
    let document = window.document().expect("Should have a document on window");

    // Get nav dropdown.
    let nav = document
        .get_element_by_id("nav")
        .expect("Could not get 'nav' element");

    // Close nav dropdown (mobile)
    if nav.class_name() == "nav responsive" {
        nav.set_class_name("nav");
    }

    // Get project dropdown.
    let dropdown = document
        .get_element_by_id("projects_dropdown")
        .expect("Could not get 'dropdown' element");
    let drop_symbol = document
        .get_element_by_id("drop_symbol")
        .expect("Could not get 'drop_symbol' element");

    // Close project dropdown.
    if dropdown.class_name() == "dropdown-content show" {
        dropdown.set_class_name("dropdown-content");
        drop_symbol.set_class_name("arrow down")
    }
}

#[wasm_bindgen]
pub fn proj_toggle() {
    let window = web_sys::window().expect("No global `window` exists");
    let document = window.document().expect("Should have a document on window");

    let dropdown = document
        .get_element_by_id("projects_dropdown")
        .expect("Could not get 'dropdown' element");
    let drop_symbol = document
        .get_element_by_id("drop_symbol")
        .expect("Could not get 'drop_symbol' element");

    if dropdown.class_name() == "dropdown-content" {
        dropdown.set_class_name("dropdown-content show");
        drop_symbol.set_class_name("arrow up")
    } else {
        dropdown.set_class_name("dropdown-content");
        drop_symbol.set_class_name("arrow down")
    }
}

pub async fn goto_page(route: &str, resource: &str, title: &str) {
    close_dropdowns();

    let window = web_sys::window().expect("No global `window` exists");
    let document = window.document().expect("Should have a document on window");

    let mut req = RequestInit::new();
    req.method("GET");
    let request =
        Request::new_with_str_and_init(resource, &req).expect("Request could not be created");
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

    window.scroll_to_with_x_and_y(0.0, 0.0);

    let title = title.to_owned() + " | BusyBoredom (Charlie Wilkin)";
    document.set_title(&title);

    // Remove the history entry pushed on page load, and replace it.
    let history = window.history().expect("Could not get history");
    if history.state().expect("Could not get history state") != route {
        history
            .push_state_with_url(&JsValue::from_str(route), &title, Some(route))
            .expect("Could not push state (with URL) to history");
    }
}

#[wasm_bindgen]
pub async fn resume() {
    // Set active tab.
    active_tab("resume");

    // Go to the page.
    goto_page("/resume", "/api/resume.html", "Résumé").await;
}

#[wasm_bindgen]
pub async fn welcome() {
    // Set active tab.
    active_tab("");

    // Go to the page.
    goto_page("/", "/api/welcome.html", "Welcome!").await;
}

#[wasm_bindgen]
pub async fn contact() {
    // Set active tab.
    active_tab("contact");

    // Go to the page.
    goto_page("/contact", "/api/contact.html", "Contact").await;
}

#[wasm_bindgen]
pub async fn coming_soon() {
    // Set active tab
    active_tab("");

    // Go to the page.
    goto_page("/coming_soon", "/api/coming_soon.html", "Coming Soon!").await;
}

#[wasm_bindgen]
pub async fn error_404() {
    // Set active tab
    active_tab("");

    let window = web_sys::window().expect("No global `window` exists");
    let document = window.document().expect("Should have a document on window");
    let history = window.history().expect("Could not get history");

    let mut req = RequestInit::new();
    req.method("GET");
    let request = Request::new_with_str_and_init("/api/404.html", &req)
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
    close_dropdowns();

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
        "/coming_soon" => spawn_local(coming_soon()),
        "/projects/this_website" => spawn_local(this_website()),
        "/projects/quadcopter" => spawn_local(quadcopter()),
        "/projects/amplifier_optimizer" => spawn_local(amplifier_optimizer()),
        "/projects/industrial_automation" => spawn_local(industrial_automation()),
        "/projects/mnist_tutorial" => spawn_local(mnist_tutorial()),
        "/projects/archviz" => spawn_local(archviz()),
        _ => spawn_local(error_404()),
    };
}

#[wasm_bindgen]
pub fn init_panic_hook() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();
}
