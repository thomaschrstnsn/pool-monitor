fn main() {
    println!("cargo:rustc-link-search=lib");
    println!("cargo:rustc-link-arg-bins=-Tlinkall.x");
}
