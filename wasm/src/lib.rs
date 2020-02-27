use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use wasm_bindgen::JsCast;
use web_sys::{Request, RequestInit, Response};

// Called when the wasm module is instantiated
#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    init_panic_hook();

    // Use `web_sys`'s global `window` function to get a handle on the global
    // window object.
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let body = document.body().expect("document should have a body");

    // Get current URL and load the resulting page.
    match &window.location().pathname().unwrap()[..] {
        "/" => spawn_local(welcome(window.clone(), document.clone())),
        _ => spawn_local(error_404(window.clone(), document.clone())),
    }

    async fn welcome(window: web_sys::Window, document: web_sys::Document) {
        let mut req = RequestInit::new();
        req.method("GET");
        let request =
            Request::new_with_str_and_init(
                "/api/welcome", 
                &req).expect("Request could not be created");
        request.headers().set("Accept", "text/html").expect("Headers could not be set");

        let response = JsFuture::from(window.fetch_with_request(&request))
            .await
            .expect("Could not unwrap response");

        // `response` is a `Response` object.
        assert!(response.is_instance_of::<Response>());
        let resp: Response = response.dyn_into().unwrap();

        // Convert this other `Promise` into a rust `Future`.
        let page = JsFuture::from(resp.text().unwrap()).await.unwrap().as_string().unwrap();

        // Show the new content.
        document
            .get_element_by_id("page")
            .unwrap()
            .set_inner_html(&page);
        document.set_title("Welcome!");
    }

    async fn error_404(window: web_sys::Window, document: web_sys::Document) {
        let mut req = RequestInit::new();
        req.method("GET");
        let request =
            Request::new_with_str_and_init(
                "/api/error-404", 
                &req).expect("Request could not be created");
        request.headers().set("Accept", "text/html").expect("Headers could not be set");

        let response = JsFuture::from(window.fetch_with_request(&request))
            .await
            .expect("Could not unwrap response");

        // `response` is a `Response` object.
        assert!(response.is_instance_of::<Response>());
        let resp: Response = response.dyn_into().unwrap();

        // Convert this other `Promise` into a rust `Future`.
        let page = JsFuture::from(resp.text().unwrap()).await.unwrap().as_string().unwrap();

        // Show the new content.
        document
            .get_element_by_id("page")
            .unwrap()
            .set_inner_html(&page);
        document.set_title("404: Page Not Found");
    }
    
    // Manufacture the element we're gonna append
    let val = document.create_element("p")?;
    val.set_inner_html("");

    body.append_child(&val)?;

    Ok(())
}

#[wasm_bindgen]
pub fn init_panic_hook() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();
}
