// Headless TypeScript binding generator. Invoked via `make gen-bindings`
// (or `cargo run --bin gen-bindings`) so CI / dev shells without a display
// can keep `src/lib/tauri/bindings.ts` in sync with the Rust commands.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "../src/lib/tauri/bindings.ts".to_string());
    hugo_studio_lib::export_typescript_bindings(&path)?;
    println!("wrote {path}");
    Ok(())
}
