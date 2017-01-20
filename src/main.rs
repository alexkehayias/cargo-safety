#![feature(proc_macro)]

extern crate glob;
extern crate syntex_syntax;
extern crate syntex_errors;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate harbor;

use std::rc::Rc;
use std::path::{Path};
use std::collections::{HashSet, HashMap};
use std::process::Command;
use std::result::Result;
use glob::glob;

use syntex_syntax::codemap::{CodeMap};
use syntex_syntax::parse::{self, ParseSess};
use syntex_syntax::ast::{NodeId};
use syntex_syntax::visit::{Visitor};
use syntex_errors::{Handler};
use syntex_errors::emitter::{ColorConfig};

use harbor::checks::{UnsafeCrate, UnsafeCode};
use harbor::reports::{SafetyReport, Status};


// Returns false for any directories that should be excluded based on
// cargo conventions
pub fn is_valid_dir(file_path: &str) -> bool {
    !(file_path.contains("examples") ||
      file_path.contains("target") ||
      file_path.contains("tests") ||
      file_path.contains("benches"))
}

#[test]
fn test_is_valid_dir() {
    assert!(is_valid_dir(&"src/main.rs"));
    assert!(!is_valid_dir(&"benches/main.rs"));
    assert!(!is_valid_dir(&"examples/main.rs"));
    assert!(!is_valid_dir(&"tests/test.rs"));
    assert!(!is_valid_dir(&"target/test.rs"));
}

// Iterate through all files in the repo and return all safety infractions
pub fn safety_infractions<'a>(root: &Path) -> HashSet<UnsafeCode> {
    let codemap = Rc::new(CodeMap::new());
    let tty_handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(codemap.clone()));
    let parse_session = ParseSess::with_span_handler(tty_handler, codemap.clone());

    glob(root.join("*.rs").to_str().unwrap()).expect("Failed to glob")
        .filter_map(Result::ok)
        .filter(|x| is_valid_dir(x.to_str().expect("Failed to coerce to string")))
        .fold(HashSet::<UnsafeCode>::new(), |accum, path_buf| {
            let file_path = path_buf.as_path();

            let krate = parse::parse_crate_from_file(file_path, &parse_session).unwrap();
            let mut unsafe_code = UnsafeCrate {
                locations: HashSet::<UnsafeCode>::new(),
                codemap: &codemap,
            };

            // Warning this has side-effects!
            unsafe_code.visit_mod(&krate.module, krate.span, NodeId::new(0));

            if unsafe_code.locations.len() > 0 {
                accum.union(&unsafe_code.locations)
                    .cloned()
                    .collect::<HashSet<UnsafeCode>>()
            } else {
                accum
            }
        })
}

#[derive(Debug, Deserialize, Serialize)]
struct Dependency {
    features: Vec<String>,
    kind: String,
    name: String,
    optional: bool,
    req: String,
    source: String,
    target: Option<String>,
    uses_default_features: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct Target {
    kind: Option<Vec<String>>,
    name: String,
    src_path: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Node {
    id: String,
    dependencies: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ResolveMap {
    nodes: Vec<Node>,
    root: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Package {
    id: String,
    name: String,
    version: String,
    manifest_path: String,
    features: Option<HashMap<String, Vec<String>>>,
    targets: Vec<Target>,
    dependencies: Vec<Dependency>,
    source: String,
    license_file: Option<String>,
    license: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Metadata {
    version: u32,
    resolve: ResolveMap,
    workspace_members: Vec<String>,
    packages: Vec<Package>,
}

pub const USAGE: &'static str = "
Compile a local package and all of its dependencies
Usage:
    cargo safety
Options:
    -h, --help                   Print this message
";

pub fn main() {
    let output = Command::new("cargo").arg("metadata").output();
    let stdout = output.unwrap().stdout;
    let json = String::from_utf8(stdout).expect("Failed reading cargo output");
    let data: Metadata = serde_json::from_str(&json).expect("Failed to parse json");

    let mut result: Vec<SafetyReport> = vec![];
    for package in data.packages {
        for target in package.targets {
            let path = Path::new(&target.src_path).parent().unwrap();
            let infractions = safety_infractions(path);
            let status = Status::from_bool(infractions.len() == 0);
            let report = SafetyReport::new(target.name, status, infractions);
            result.push(report);
        };
    };
    println!("{}", serde_json::to_string(&result).unwrap());
}
