use wasm_bindgen::prelude::*;

use crate::{active_tab, goto_page};

#[wasm_bindgen]
pub async fn archviz() {
    // Set active tab.
    active_tab("");

    // Go to the page.
    goto_page(
        "/projects/archviz",
        "/api/projects/archviz/archviz.html",
        "Archviz",
    )
    .await;

    let window = web_sys::window().expect("No global `window` exists");
    let document = window.document().expect("Should have a document on window");

    // Load Unity's JavaScript stuff.
    let loader = document
        .create_element("script")
        .expect("Could not create Unity Load script element.");
    loader
        .set_attribute(
            "src",
            "/api/projects/archviz/Build/UnityLoader.js?ver=d8tjExZ0_k8",
        )
        .expect("Could not set unity loader 'src' attribute.");
    loader
        .set_attribute("onload", "unityInitializer()")
        .expect("Could not set unity loader 'onload' attribute.");

    let progress = document
        .create_element("script")
        .expect("Could not create Unity Progress script element.");
    progress
        .set_attribute(
            "src",
            "/api/projects/archviz/TemplateData/UnityProgress.js?ver=ac6T--xi1Fs",
        )
        .expect("Could not set unity progress bar 'src' attribute.");

    let instance = document
        .create_element("script")
        .expect("Could not create Unity instance script element.");
    instance.set_inner_html(
        "var unityInstance;
                function unityInitializer() {
                    unityInstance = UnityLoader.instantiate(
                    \"unityContainer\", 
                    \"/api/projects/archviz/Build/WebGLBuild.json?ver=h0kqziNXTOM\", 
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
}
