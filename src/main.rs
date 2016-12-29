#![feature(proc_macro)]

extern crate syntex_syntax;
extern crate syntex_errors;
extern crate git2;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate harbor;

use git2::{Repository, Oid, ResetType, ObjectType, BranchType};
use std::env;
use std::rc::Rc;
use std::path::{Path};
use std::str::{from_utf8};
use std::collections::{HashSet};
use syntex_syntax::codemap::{CodeMap};
use syntex_syntax::parse::{self, ParseSess};
use syntex_syntax::ast::{NodeId};
use syntex_syntax::visit::{Visitor};
use syntex_errors::{Handler};
use syntex_errors::emitter::{ColorConfig};
use harbor::checks::{UnsafeCrate, UnsafeCode};
use harbor::reports::{SafetyReport};


// Returns the project name by extracting it from the git url
pub fn git_url_to_name(git_url: &String) -> String {
    git_url.split("/").collect::<Vec<&str>>().last().unwrap().to_lowercase()
}

#[test]
fn test_git_url_to_name() {
    assert!("harbor" == git_url_to_name(&String::from("https://github.com/alexkehayias/harbor")));
}

// Returns a repository by fetching it from disk or cloning it
// If the repo already exists it will fetch the latest from origin
pub fn get_or_clone(git_url: &String, path: &String) -> Repository {
    match Repository::open(path) {
        // If we already have it on disk, fetch the latest
        Ok(repo) => {
            repo.find_remote(&"origin")
                .unwrap()
                .fetch(&[], None, None)
                .unwrap();
            repo.set_head(
                // TODO: this is gross, why can't I let bind this without
                // the borrow checker throwing up?
                &repo.find_branch("origin/master", BranchType::Remote)
                    .unwrap()
                    .get()
                    .name()
                    .unwrap()
            ).unwrap();
            repo.checkout_head(None).unwrap();
            repo
        },
        // Otherwise clone it
        Err(_) => {
            match Repository::clone(git_url, path) {
                Ok(repo) => repo,
                Err(e) => panic!("Failed to clone: {}", e),
            }
        }
    }
}

pub fn is_rust_file(file_path: &str) -> bool {
    file_path.contains(".rs")
}

#[test]
fn test_is_rust_file() {
    assert!(is_rust_file(&"src/main.rs"));
    assert!(!is_rust_file(&"src/main.js"));
}

// Returns false for any directories that should be excluded based on
// cargo conventions
pub fn is_in_valid_dir(file_path: &str) -> bool {
    !(file_path.contains("examples") ||
      file_path.contains("target") ||
      file_path.contains("tests") ||
      file_path.contains("benches"))
}

#[test]
fn test_is_in_valid_dir() {
    assert!(is_in_valid_dir(&"src/main.rs"));
    assert!(!is_in_valid_dir(&"benches/main.rs"));
    assert!(!is_in_valid_dir(&"examples/main.rs"));
    assert!(!is_in_valid_dir(&"tests/test.rs"));
    assert!(!is_in_valid_dir(&"target/test.rs"));
}

pub fn is_valid_file(file_path: &str) -> bool {
    is_rust_file(file_path) && is_in_valid_dir(file_path)
}

// Iterate through all files in the repo and return all safety infractions
pub fn safety_infractions<'a>(prefix: String, repo: Repository)
                              -> HashSet<UnsafeCode> {
    let codemap = Rc::new(CodeMap::new());
    let tty_handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(codemap.clone()));
    let parse_session = ParseSess::with_span_handler(tty_handler, codemap.clone());
    let index = repo.index().unwrap();

    index.iter()
        .filter(|x| is_valid_file(from_utf8(&x.path).unwrap()))
        .fold(HashSet::<UnsafeCode>::new(), |accum, i| {
            let file_path = from_utf8(&i.path).unwrap();
            let path_buf = Path::new(&prefix).join(file_path);
            let krate = parse::parse_crate_from_file(
                path_buf.as_path(),
                &parse_session
            ).unwrap();
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

#[test]
// Tests the integration between a git repo and finding unsafe code
// This test uses the harbor repo (since we are in it).
fn test_safety_infractions() {
    let repo = Repository::open("../harbor");
    let actual = safety_infractions(String::from("../harbor"), repo.unwrap());
    assert!(actual.len() == 0);
}

// Args:
// - git url of the project to check
// - optional commit sha to checkout
//
// Environment variables:
// - HARBOR_HOME: Path to a directory that the process has write access to
fn main() {
    let git_url = env::args().nth(1);
    let git_commit = env::args().nth(2);
    let home_dir = match env::var("HARBOR_HOME") {
        Ok(val) => val,
        Err(_) => String::from(".harbor"),
    };
    match git_url {
        Some(url) => {
            let name = git_url_to_name(&url);
            let path = format!("{root}/{path}", root=home_dir, path=name);
            let repo = get_or_clone(&url, &path);

            // If we got a commit reset hard to that otherwise use the default
            if let Some(commit) = git_commit {
                let oid = Oid::from_str(&commit).unwrap();
                let target = repo.find_object(oid, Some(ObjectType::Commit)).unwrap();
                repo.reset(&target, ResetType::Hard, None).unwrap();
            }

            let infractions = safety_infractions(path, repo);
            let passed = infractions.len() == 0;
            let report = SafetyReport::new(url, passed, infractions);
            print!("{}", serde_json::to_string(&report).unwrap());
        }
        None => {
            panic!("Please provide a git repo url.");
        }
    };
}
