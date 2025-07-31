use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo::rerun-if-changed=../README.md");
    let readme = fs::read_to_string("../README.md").unwrap();
    let doc = readme.split_once("<!-- start-doc -->").unwrap().1.trim();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(out_dir.join("docs.md"), doc).unwrap();
}
