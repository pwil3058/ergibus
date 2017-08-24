extern crate snapshot;
extern crate serde;
extern crate serde_json;

use snapshot::{SnapshotDir};
use std::path::{Path};

fn main() {
    let p = Path::new(".");
    let sd = SnapshotDir::new(p).unwrap_or_else(|err| {
        panic!("bummer: {:?}", err);
    });
    let sd_str = serde_json::to_string(&sd).unwrap_or_else(|err| {
        panic!("double bummer: {:?}", err);
    });
    println!("JSON string:\n{:?}", sd_str);
    let sde: SnapshotDir = serde_json::from_str(&sd_str).unwrap_or_else(|err| {
        panic!("triple bummer: {:?}", err);
    });
    println!("***********************************************");
    println!("***********************************************");
    println!("Extracted:\n{:?}", sde);
}
