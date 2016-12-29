#![feature(proc_macro)]

extern crate syntex_syntax;
extern crate syntex_errors;
extern crate git2;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use git2::Repository;
use std::env;
use std::rc::Rc;
use std::path::{Path};
use std::str::{from_utf8};
use std::collections::{HashSet};
use syntex_syntax::codemap::{CodeMap, Span};
use syntex_syntax::parse::{self, ParseSess};
use syntex_syntax::ast::{NodeId, Block, FnDecl, Mac, Unsafety, BlockCheckMode,
                         TraitItem, ImplItemKind, ImplItem, TraitItemKind};
use syntex_syntax::visit::{self, Visitor, FnKind};
use syntex_errors::{Handler};
use syntex_errors::emitter::{ColorConfig};


#[allow(non_camel_case_types)]
#[derive(Hash, Eq, PartialEq, Debug, Clone, Serialize)]
enum UnsafeKind {
    unsafe_function,
    unsafe_impl,
    unsafe_block,
    unsafe_trait,
}

#[derive(Hash, Eq, PartialEq, Debug, Clone, Serialize)]
pub struct UnsafeCode {
    kind: UnsafeKind,
    occurences: String,
}

impl UnsafeCode {
    fn new(kind: UnsafeKind, occurences: String) -> UnsafeCode {
        UnsafeCode {kind: kind, occurences: occurences}
    }
}

// The codemap is necessary to go from a `Span` to actual line & column
// numbers for closures.
pub struct UnsafeCrate<'a> {
    // Format {
    //   kind: <unsafe_function | unsafe_impl | unsafe_block | unsafe_trait>,
    //   location: <See CodeMap.span_to_expanded_string for format details>
    // }
    //
    locations: HashSet<UnsafeCode>,
    // Used to go from a Span to line:column information
    codemap: &'a CodeMap,
}

// Unsafe code can be introduced in functions, blocks, traits, and implementations
impl<'a> Visitor for UnsafeCrate<'a> {
    // Implement this otherwise it will panic when it hits a macro
    fn visit_mac(&mut self, _mac: &Mac) {}

    // Recursively capture all occurences of unsafe functions
    fn visit_fn(&mut self,
                fn_kind: FnKind,
                fn_decl: &FnDecl,
                span: Span,
                _id: NodeId) {
        match fn_kind {
            FnKind::Method(_, _, _, _) => (),
            FnKind::Closure(_) => (),
            FnKind::ItemFn(_, _, unsafety, _, _, _, _) => {
                match unsafety {
                    Unsafety::Normal => (),
                    Unsafety::Unsafe => {
                        let record = UnsafeCode::new(
                            UnsafeKind::unsafe_function,
                            self.codemap.span_to_expanded_string(span),
                        );
                        self.locations.insert(record);
                    },
                };
            }
        };
        visit::walk_fn(self, fn_kind, fn_decl, span);
    }

    // Recursively capture all unsafe blocks
    fn visit_block(&mut self, block: &Block) {
        match block.rules {
            BlockCheckMode::Default => (),
            BlockCheckMode::Unsafe(_) => {
                let record = UnsafeCode::new(
                    UnsafeKind::unsafe_block,
                    self.codemap.span_to_expanded_string(block.span),
                );
                self.locations.insert(record);
            },
        };
        visit::walk_block(self, block);
    }

    // // Capture any unsafe traits
    fn visit_trait_item(&mut self, ti: &TraitItem) {
        match ti.node {
            TraitItemKind::Const(_, _) => (),
            TraitItemKind::Type(_, _) => (),
            TraitItemKind::Macro(_) => (),
            TraitItemKind::Method(ref sig, _) => match sig.unsafety {
                Unsafety::Normal => (),
                Unsafety::Unsafe => {
                    let record = UnsafeCode::new(
                        UnsafeKind::unsafe_trait,
                        self.codemap.span_to_expanded_string(ti.span),
                    );
                    self.locations.insert(record);
                },
            },
        };
    }

    // // Capture any unsafe implementations
    fn visit_impl_item(&mut self, ii: &ImplItem) {
        match ii.node {
            ImplItemKind::Const(_, _) => (),
            ImplItemKind::Type(_) => (),
            ImplItemKind::Macro(_) => (),
            ImplItemKind::Method(ref sig, _) => match sig.unsafety {
                Unsafety::Normal => (),
                Unsafety::Unsafe => {
                    let record = UnsafeCode::new(
                        UnsafeKind::unsafe_impl,
                        self.codemap.span_to_expanded_string(ii.span),
                    );
                    self.locations.insert(record);
                }
            },
        };
    }
}

// Returns the project name by extracting it from the git url
pub fn git_url_to_name(git_url: &String) -> String {
    git_url.split("/").collect::<Vec<&str>>().last().unwrap().to_lowercase()
}

#[test]
fn test_git_url_to_name() {
    assert!("harbor" == git_url_to_name(&String::from("https://github.com/alexkehayias/harbor")));
}

// Returns a repository by fetching it from disk or cloning it fresh
pub fn get_or_clone(git_url: &String, path: &String) -> Repository {
    match Repository::open(path) {
        Ok(repo) => repo,
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
      file_path.contains("tests"))
}

#[test]
fn test_is_in_valid_dir() {
    assert!(is_in_valid_dir(&"src/main.rs"));
    assert!(!is_in_valid_dir(&"examples/main.rs"));
    assert!(!is_in_valid_dir(&"tests/test.rs"));
    assert!(!is_in_valid_dir(&"target/test.rs"));
}

pub fn is_valid_file(file_path: &str) -> bool {
    is_rust_file(file_path) && is_in_valid_dir(file_path)
}

#[derive(Debug, Serialize)]
struct SafetyReport {
    repo_url: String,
    status: bool,
    offenses: HashSet<UnsafeCode>,
}

impl SafetyReport {
    pub fn new(repo_url: String,
               status: bool,
               offenses: HashSet<UnsafeCode>) -> SafetyReport {
        SafetyReport {repo_url: repo_url, status: status, offenses: offenses}
    }
}

// Iterate through all files in the repo and return all safety infractions
pub fn safety_infractions<'a>(prefix: String, repo: Repository)
                              -> HashSet<UnsafeCode> {
    let codemap = Rc::new(CodeMap::new());
    let tty_handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(codemap.clone()));
    let parse_session = ParseSess::with_span_handler(tty_handler, codemap.clone());

    match repo.index() {
        Ok(index) => {
            // TODO parallize this
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
        },
        Err(e) => panic!("Failed to parse: {}", e),
    }
}

#[test]
// Tests the integration between a git repo and finding unsafe code
// This test uses the harbor repo (since we are in it).
fn test_safety_infractions() {
    let repo = Repository::open("../harbor");
    let actual = safety_infractions(String::from("../harbor"), repo.unwrap());
    assert!(actual.len() == 0);
}

// Called with an argument for a git url of the project to check
//
// Environment variables:
// - HARBOR_HOME: Path to a directory that the process has write access to
fn main() {
    let git_url = env::args().nth(1);
    let home_dir = match env::var("HARBOR_HOME") {
        Ok(val) => val,
        Err(_) => String::from(".harbor"),
    };
    match git_url {
        Some(url) => {
            let name = git_url_to_name(&url);
            let path = format!("{root}/{path}", root=home_dir, path=name);
            let repo = get_or_clone(&url, &path);
            let infractions = safety_infractions(path, repo);
            let passed = infractions.len() == 0;
            let report = SafetyReport::new(url, passed, infractions);
            println!("{}", serde_json::to_string(&report).unwrap());
        }
        None => {
            panic!("Please provide a git repo url.");
        }
    };
}
