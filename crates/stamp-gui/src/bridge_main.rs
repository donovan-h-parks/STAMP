//! `stamp-mcp`: the program Claude Desktop starts to reach STAMP. See
//! `stamp_gui::assistant::bridge`.

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    stamp_gui::assistant::bridge::run();
}

// Claude Desktop cannot start programs in a browser; nothing to do there.
#[cfg(target_arch = "wasm32")]
fn main() {}
