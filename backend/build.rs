// Recompile when migrations change: `sqlx::migrate!` embeds them at compile time but does not
// register the directory with Cargo's change tracking on stable Rust.
fn main() {
    println!("cargo:rerun-if-changed=../migrations");
}
