// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|a| a == "--askpass") {
        std::process::exit(squadron_lib::askpass::client(
            args.get(2).map_or("", String::as_str),
        ));
    }
    squadron_lib::run();
}
