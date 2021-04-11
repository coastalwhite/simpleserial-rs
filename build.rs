use std::process::Command;

fn main() {
    // makefile is using a special env variable
    Command::new("make")
        .env("PLATFORM", "CWLITEARM")
        .status()
        .expect("failed to make!");
}