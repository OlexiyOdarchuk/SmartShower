fn main() {
    if !std::path::Path::new("src/secrets.rs").exists() {
        panic!("Немає src/secrets.rs: скопіюй src/secrets.example.rs у src/secrets.rs і заповни");
    }
    println!("cargo:rerun-if-changed=src/secrets.rs");
    embuild::espidf::sysenv::output();
}
