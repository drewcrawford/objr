extern crate cc;

fn main() {
    cc::Build::new()
        .file("src/hard-exception.m")
        .compile("libexception.a");
}