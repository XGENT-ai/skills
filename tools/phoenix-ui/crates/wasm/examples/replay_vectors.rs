use std::io::{self, BufRead};

fn main() {
    for line in io::stdin().lock().lines() {
        let request: serde_json::Value =
            serde_json::from_str(&line.expect("stdin")).expect("request");
        let result = impeccable_wasm::exports_pure::pure_call(
            request["module"].as_str().expect("module"),
            request["name"].as_str().expect("name"),
            request["args"].as_str().expect("args JSON"),
        );
        println!("{}", serde_json::json!({ "result": result }));
    }
}
