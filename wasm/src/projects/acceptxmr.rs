use wasm_bindgen::prelude::*;

use crate::{active_tab, goto_page};

#[wasm_bindgen]
pub async fn acceptxmr() {
    // Set active tab.
    active_tab("");

    // Go to the page.
    goto_page(
        "/projects/acceptxmr",
        "/api/projects/acceptxmr/acceptxmr.html?ver=_Y2SYCtt4ss",
        "AcceptXMR",
    )
    .await;

    let window = web_sys::window().expect("No global `window` exists");
    let document = window.document().expect("Should have a document on window");

    // Load acceptxmr's js.
    let acceptxmr_js = document
        .create_element("script")
        .expect("Could not create Unity Load script element.");
    acceptxmr_js
        .set_attribute(
            "src",
            "/api/projects/acceptxmr/acceptxmr.js?ver=gFLt824zDMI",
        )
        .expect("Could not set 'src' attribute for acceptxmr.js.");
    
    // Load qrcode's js.
    let qrcode_js = document
        .create_element("script")
        .expect("Could not create Unity Load script element.");
    qrcode_js
        .set_attribute(
            "src",
            "/api/projects/acceptxmr/vendor/qrcode.js?ver=ZWNnb_r_P3s",
        )
        .expect("Could not set 'src' attribute for qrcode.js.");

    if let Some(head) = document.get_elements_by_tag_name("head").item(0) {
        head.append_with_node_1(&acceptxmr_js)
            .expect("Could not append acceptxmr js script to document");
    }

    if let Some(head) = document.get_elements_by_tag_name("head").item(0) {
        head.append_with_node_1(&qrcode_js)
            .expect("Could not append qrcode js script to document");
    }
}
