use specta_typescript::Typescript;
use tauri_specta::{collect_commands, Builder};

mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(err) = try_run() {
        eprintln!("Hugo Studio failed to start: {err}");
        std::process::exit(1);
    }
}

fn try_run() -> Result<(), Box<dyn std::error::Error>> {
    let builder = Builder::<tauri::Wry>::new()
        .commands(collect_commands![commands::health_check::health_check]);

    // In debug builds export the typed bindings to the frontend so the TS
    // client stays in sync as commands evolve. Release builds skip this.
    #[cfg(debug_assertions)]
    builder.export(Typescript::default(), "../src/lib/tauri/bindings.ts")?;

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())?;

    Ok(())
}
