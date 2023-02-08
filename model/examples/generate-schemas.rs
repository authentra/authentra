use std::{fs::File, path::PathBuf, str::FromStr};

use model::{Flow, FlowData, Policy, Prompt, Stage};
use schemars::schema_for;

const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

macro_rules! gen_schema {
    ($name:literal: $ty:ty) => {{
        let path = PathBuf::from_str(&format!("{}/schemas/{}.json", MANIFEST_DIR, $name)).unwrap();
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir(parent).unwrap();
            }
        }
        let file = File::create(path).unwrap();
        let schema = schema_for!($ty);
        serde_json::to_writer_pretty(file, &schema).unwrap();
    }};
}

fn main() {
    gen_schema!("flow": Flow);
    gen_schema!("stage": Stage);
    gen_schema!("policy": Policy);
    gen_schema!("prompt": Prompt);
    gen_schema!("flow-data": FlowData);
}
