//! Herramienta de desarrollo: regenera `src/bindings.ts` sin abrir la app.
//!
//! Uso: `cargo run --bin export_bindings` desde `src-tauri`.
fn main() {
    isalab_lib::specta_builder()
        .export(
            specta_typescript::Typescript::default(),
            "../src/bindings.ts",
        )
        .expect("no se pudieron exportar los bindings TypeScript");
    println!("bindings exportados a ../src/bindings.ts");
}
