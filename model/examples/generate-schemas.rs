use std::{fs::File, path::PathBuf, str::FromStr};

use authust_model::{
    user::PartialUser, Flow, FlowData, FlowDesignation, Policy, Prompt, Reference, Stage,
};
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
    gen_schema!("partial-user": PartialUser);
    gen_schema!("flow-designation": FlowDesignation);
    gen_schema!("reference": Reference<Flow>);
}