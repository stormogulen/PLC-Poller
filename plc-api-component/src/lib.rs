
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Request, RequestInit, RequestMode, Response, WebSocket, Blob, FileReader, Event, MessageEvent};
use js_sys::{Uint8Array, JsString};
use std::rc::Rc;
use std::cell::RefCell;
use serde::{Deserialize, Serialize};

// JavaScript functions that we'll call from Rust
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_name = updateUiPlc)]
    fn update_ui_plc(value: &str);

    #[wasm_bindgen(js_name = updateUiApi)]
    fn update_ui_api(result: &str);

    #[wasm_bindgen(js_name = handleError)]
    fn handle_error(error: &str);
}

#[derive(Serialize, Deserialize)]
struct ApiResponse {
    id: u32,
    title: String,
    completed: bool,
}

#[wasm_bindgen]
pub struct PlcApiComponent {
    socket: Rc<RefCell<WebSocket>>,
}

#[wasm_bindgen]
impl PlcApiComponent {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<PlcApiComponent, JsValue> {
        log("Creating new WebSocket...");
        let socket = WebSocket::new("ws://127.0.0.1:5502")?;
        Ok(PlcApiComponent { socket: Rc::new(RefCell::new(socket)) })
    }

    pub fn start_polling(&self) -> Result<(), JsValue> {
        log("Start polling...");
        let socket = self.socket.clone();

        let onopen_callback = Closure::wrap(Box::new(move |_| {
            log("WebSocket opened");
            if let Err(e) = send_read_request(&socket.borrow()) {
                handle_error(&format!("Failed to send read request: {:?}", e));
            }
        }) as Box<dyn FnMut(JsValue)>);

        let socket_clone_message = self.socket.clone();
        let onmessage_callback = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
            log("onmessage callback");
            match process_message(e, socket_clone_message.clone()) {
                Ok(_) => log("Message processed successfully"),
                Err(err) => handle_error(&format!("Failed to process message: {:?}", err)),
            }
        }) as Box<dyn FnMut(web_sys::MessageEvent)>);

        self.socket.borrow().set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        self.socket.borrow().set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));

        onopen_callback.forget();
        onmessage_callback.forget();

        Ok(())
    }
}

// Process incoming messages from the PLC
fn process_message(e: MessageEvent, socket_clone_message: Rc<RefCell<WebSocket>>) -> Result<(), JsValue> {
    log("Processing message event");

    // Convert the message data to a Blob
    let blob = e.data().dyn_into::<Blob>().map_err(|err| {
        log(&format!("Failed to convert to Blob: {:?}", err));
        err
    })?;

    log(&format!("Blob size: {} bytes", blob.size()));

    // Convert the Blob to an ArrayBuffer
    let promise = blob.array_buffer();
    let future = JsFuture::from(promise);

    let socket_clone = socket_clone_message.clone();
   
    spawn_local(async move {
        match future.await {
            Ok(array_buffer) => {
                let array = js_sys::Uint8Array::new(&array_buffer);
                let mut bytes = vec![0u8; array.length() as usize];
                array.copy_to(&mut bytes);
                log(&format!("Received bytes: {:?}", bytes));
               
                if bytes.len() >= 11 {
                    let value = u16::from_be_bytes([bytes[9], bytes[10]]);
                    log(&format!("Extracted value: {}", value));
                    update_ui_plc(&value.to_string());

                    match poll_and_request(value).await {
                        Ok(result) => {
                            log(&format!("API response: {}", result));
                            update_ui_api(&result);
                        },
                        Err(e) => {
                            let error_msg = format!("API Error: {:?}", e);
                            log(&error_msg);
                            update_ui_api(&error_msg);
                        },
                    }

                    // Schedule the next read request
                    schedule_next_read(&socket_clone);
                } else {
                    handle_error("Received message is too short");
                }
            },
            Err(e) => handle_error(&format!("Failed to get ArrayBuffer from Blob: {:?}", e)),
        }
    });

    Ok(())
}

// Schedule the next read request after a short delay
fn schedule_next_read(socket: &Rc<RefCell<WebSocket>>) {
    let socket_clone = socket.clone();
    spawn_local(async move {
        // Wait for a short interval before sending the next read request
        wasm_bindgen_futures::JsFuture::from(js_sys::Promise::new(&mut |resolve, _| {
            web_sys::window()
                .unwrap()
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    &resolve,
                    100, // 100ms delay
                )
                .unwrap();
        }))
        .await
        .unwrap();

        if let Err(e) = send_read_request(&socket_clone.borrow()) {
            handle_error(&format!("Failed to send read request: {:?}", e));
        }
    });
}


// Send a read request to the PLC
fn send_read_request(socket: &WebSocket) -> Result<(), JsValue> {
    log("Sending read request");
    let request = Uint8Array::new_with_length(12);
    request.set_index(0, 0); // Transaction ID High Byte
    request.set_index(1, 1); // Transaction ID Low Byte
    request.set_index(2, 0); // Protocol ID High Byte
    request.set_index(3, 0); // Protocol ID Low Byte
    request.set_index(4, 0); // Message Length High Byte
    request.set_index(5, 6); // Message Length Low Byte
    request.set_index(6, 1); // Unit ID
    request.set_index(7, 3); // Function Code (Read Holding Registers)
    request.set_index(8, 0); // Starting Address High Byte
    request.set_index(9, 0); // Starting Address Low Byte
    request.set_index(10, 0); // Quantity High Byte
    request.set_index(11, 1); // Quantity Low Byte

    socket.send_with_u8_array(&request.to_vec())
}

// Make an API request with the PLC value
async fn poll_and_request(id: u16) -> Result<String, JsValue> {
    log(&format!("Polling API with id: {}", id));
    let url = format!("https://jsonplaceholder.typicode.com/todos/{}", id);
    log(&format!("URL: {}", url));

    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(&url, &opts)?;
    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into()?;

    if !resp.ok() {
        // Handle non-OK responses (like 404)
        return Ok(format!("API Error: {} {}", resp.status(), resp.status_text()));
    }

    let json = JsFuture::from(resp.json()?).await?;
    let data: ApiResponse = serde_wasm_bindgen::from_value(json)?;
    Ok(serde_json::to_string(&data).unwrap())
}


// Entry point
#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
    log("Running PlcApiComponent...");
    let plc_api = PlcApiComponent::new()?;
    plc_api.start_polling()?;
    Ok(())
}

