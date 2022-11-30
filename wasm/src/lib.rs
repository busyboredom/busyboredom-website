pub mod contact;
pub mod projects;

use contact::*;
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

    // Scroll to top.
    window.scroll_to_with_x_and_y(0.0, 0.0);

    // Remove base title if it exists.
    if let Some(base_title) = document.get_element_by_id("base-title") {
        base_title.remove();
    }

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
    // Go to the page.
    goto_page("/resume", "/api/resume.html?ver=2lPtGRZU09k", "Résumé").await;
}

#[wasm_bindgen]
pub async fn welcome() {
    // Set active tab.
    active_tab("");

    // Go to the page.
    goto_page("/", "/api/welcome.html?ver=2FcUtZS8rxs", "Welcome!").await;

    // Show warning if safari is detected.
    let window = web_sys::window().expect("No global `window` exists");
    let user_agent = window.navigator().user_agent().unwrap();
    if user_agent.contains("Safari") && !user_agent.contains("Chrome") {
        let document = window.document().expect("Should have a document on window");
        let warning = document
            .get_element_by_id("safari-warning")
            .expect("Could not get element with id 'safari-warning'");
        if warning.class_name() == "warning" {
            warning.set_class_name("warning show");
        }
    }
}

#[wasm_bindgen]
pub async fn coming_soon() {
    // Set active tab
    active_tab("");

    // Go to the page.
    goto_page(
        "/coming-soon",
        "/api/coming_soon.html?ver=YoSytkd9Ke0",
        "Coming Soon!",
    )
    .await;
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
    let request = Request::new_with_str_and_init("/api/404.html?ver=p9Qlk98fUzE", &req)
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
    if history.state().expect("Could not get history state") != "/error-404" {
        history
            .push_state(&JsValue::from_str("/error-404"), "404: Page Not Found")
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
        "/coming-soon" => spawn_local(coming_soon()),
        "/contact-submitted" => spawn_local(contact_submitted()),
        "/projects/acceptxmr" => spawn_local(acceptxmr()),
        "/projects/amplifier-optimizer" => spawn_local(amplifier_optimizer()),
        "/projects/archviz" => spawn_local(archviz()),
        "/projects/mnist-tutorial" => spawn_local(mnist_tutorial()),
        "/projects/quadcopter" => spawn_local(quadcopter()),
        "/projects/this-website" => spawn_local(this_website()),
        _ => spawn_local(error_404()),
    };
}

#[wasm_bindgen]
pub fn init_panic_hook() {
    #[cfg(all(debug_assertions, feature = "console_error_panic_hook"))]
    console_error_panic_hook::set_once();
}
