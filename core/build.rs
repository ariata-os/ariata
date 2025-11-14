fn main() {
    // This tells cargo to recompile if migrations change
    println!("cargo:rerun-if-changed=migrations");
}
