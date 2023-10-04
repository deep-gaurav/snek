#[cfg(not(target_family = "wasm"))]
pub fn get_width() -> f32 {
    1024_f32
}

#[cfg(not(target_family = "wasm"))]
pub fn get_height() -> f32 {
    800_f32
}

#[cfg(target_family = "wasm")]
pub fn get_width() -> f32 {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector("#main_canvas")
        .unwrap()
        .unwrap()
        .client_width() as f32
}

#[cfg(target_family = "wasm")]
pub fn get_height() -> f32 {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector("#main_canvas")
        .unwrap()
        .unwrap()
        .client_height() as f32
}
