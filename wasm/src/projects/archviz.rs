use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response};

use crate::active_tab;

#[wasm_bindgen]
pub async fn archviz() {
    active_tab("");

    let window = web_sys::window().expect("No global `window` exists");
    let document = window.document().expect("Should have a document on window");
    let history = window.history().expect("Could not get history");

    let mut req = RequestInit::new();
    req.method("GET");
    let request = Request::new_with_str_and_init("/api/projects/archviz/archviz.html", &req)
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

    // Load Unity's JavaScript stuff.
    let loader = document
        .create_element("script")
        .expect("Could not create Unity Load script element.");
    loader
        .set_attribute("src", "/api/projects/archviz/Build/UnityLoader.js")
        .expect("Could not set unity loader 'src' attribute.");
    loader
        .set_attribute("onload", "unityInitializer()")
        .expect("Could not set unity loader 'onload' attribute.");

    let progress = document
        .create_element("script")
        .expect("Could not create Unity Progress script element.");
    progress
        .set_attribute("src", "/api/projects/archviz/TemplateData/UnityProgress.js")
        .expect("Could not set unity progress bar 'src' attribute.");

    let instance = document
        .create_element("script")
        .expect("Could not create Unity instance script element.");
    instance.set_inner_html(
        "var unityInstance;
                function unityInitializer() {
                    unityInstance = UnityLoader.instantiate(
                    \"unityContainer\", 
                    \"/api/projects/archviz/Build/WebGLBuild.json\", 
                    {onProgress: UnityProgress});
                }",
    );

    if let Some(head) = document.get_elements_by_tag_name("head").item(0) {
        head.append_with_node_1(&progress)
            .expect("Could not append Unity Progress script to document");
    }

    if let Some(head) = document.get_elements_by_tag_name("head").item(0) {
        head.append_with_node_1(&loader)
            .expect("Could not append Unity Load script to document");
    }

    if let Some(head) = document.get_elements_by_tag_name("head").item(0) {
        head.append_with_node_1(&instance)
            .expect("Could not append Unity Instance script to document");
    }

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
    if history.state().expect("Could not get history state") != "/projects/archviz" {
        history
            .push_state_with_url(
                &JsValue::from_str("/projects/archviz"),
                "Archviz",
                Some("/projects/archviz"),
            )
            .expect("Could not push state to history");
    }

    document.set_title("Archviz");
}
