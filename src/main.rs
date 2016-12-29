extern crate syntex_syntax;
extern crate syntex_errors;
extern crate git2;

use git2::Repository;
use std::env;
use std::rc::Rc;
use std::path::{Path};
use std::str::{from_utf8};
use std::collections::{HashMap};
use std::fmt::{self};
use syntex_syntax::codemap::{CodeMap, Span};
use syntex_syntax::parse::{self, ParseSess};
use syntex_syntax::ast::{Crate, NodeId, Block, FnDecl, Mac, Unsafety, BlockCheckMode,
                         TraitItem, ImplItemKind, ImplItem, TraitItemKind};
use syntex_syntax::visit::{self, Visitor, FnKind};
use syntex_errors::{Handler};
use syntex_errors::emitter::{ColorConfig};


// The codemap is necessary to go from a `Span` to actual line & column
// numbers for closures.
struct UnsafeLocations<'a> {
    locations: HashMap<String, String>,
    codemap: &'a CodeMap,
}

// Unsafe code can be introduced in functions, blocks, traits, and implementations
impl<'a> Visitor for UnsafeLocations<'a> {
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
                        self.locations.insert(
                            String::from("kind"),
                            String::from("unsafe_function"),
                        );
                        self.locations.insert(
                            String::from("location"),
                            self.codemap.span_to_expanded_string(span),
                        );
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
                self.locations.insert(
                    String::from("kind"),
                    String::from("unsafe_block"),
                );
                self.locations.insert(
                    String::from("location"),
                    self.codemap.span_to_expanded_string(block.span),
                );
            }
        }
        visit::walk_block(self, block);
    }

    // Capture any unsafe traits
    fn visit_trait_item(&mut self, ti: &TraitItem) {
        match ti.node {
            TraitItemKind::Const(_, _) => (),
            TraitItemKind::Type(_, _) => (),
            TraitItemKind::Macro(_) => (),
            TraitItemKind::Method(ref sig, _) => match sig.unsafety {
                Unsafety::Normal => (),
                Unsafety::Unsafe => {
                    self.locations.insert(
                        String::from("kind"),
                        String::from("unsafe_trait"),
                    );
                    self.locations.insert(
                        String::from("location"),
                        self.codemap.span_to_expanded_string(ti.span),
                    );
                }
            },
        };
    }

    // Capture any unsafe implementations
    fn visit_impl_item(&mut self, ii: &ImplItem) {
        match ii.node {
            ImplItemKind::Const(_, _) => (),
            ImplItemKind::Type(_) => (),
            ImplItemKind::Macro(_) => (),
            ImplItemKind::Method(ref sig, _) => match sig.unsafety {
                Unsafety::Normal => (),
                Unsafety::Unsafe => {
                    self.locations.insert(
                        String::from("kind"),
                        String::from("unsafe_impl"),
                    );
                    self.locations.insert(
                        String::from("location"),
                        self.codemap.span_to_expanded_string(ii.span),
                    );
                }
            },
        };
    }
}

fn find_unsafe_blocks<'a>(krate: &Crate, codemap: &'a Rc<CodeMap>)
                          -> UnsafeLocations<'a> {
    let mut visitor = UnsafeLocations {
        locations: HashMap::new(),
        codemap: codemap,
    };
    visitor.visit_mod(&krate.module, krate.span, NodeId::new(0));
    visitor
}


pub fn git_url_to_name(git_url: &String) -> String {
    git_url.split("/").collect::<Vec<&str>>().last().unwrap().to_lowercase()
}

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

// Exclude
pub fn is_in_valid_dir(file_path: &str) -> bool {
    !(file_path.contains("examples") ||
      file_path.contains("target") ||
      file_path.contains("tests"))
}

pub fn is_valid_file(file_path: &str) -> bool {
    is_rust_file(file_path) && is_in_valid_dir(file_path)
}

// Read the main file and load the AST then search the AST
// for any `unsafe` keywords
pub fn safety_infractions<'a>(prefix: String, repo: Repository)
                              -> Vec<HashMap<String, String>> {
    let codemap = Rc::new(CodeMap::new());
    let tty_handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(codemap.clone()));
    let parse_session = ParseSess::with_span_handler(tty_handler, codemap.clone());
    let mut accum: Vec<HashMap<String, String>> = Vec::new();

    match repo.index() {
        Ok(index) => {
            // TODO parallize this
            for i in index.iter().filter(|x| is_valid_file(from_utf8(&x.path).unwrap())) {
                let file_path = from_utf8(&i.path).unwrap();
                // println!("Checking file: {}", file_path);
                let path_buf = Path::new(&prefix).join(file_path);
                let ast = parse::parse_crate_from_file(path_buf.as_path(), &parse_session);
                let blocks = find_unsafe_blocks(&ast.unwrap(), &codemap);
                if blocks.locations.len() > 0 {
                    // println!("Found unsafe: {:?}", blocks.locations);
                    accum.push(blocks.locations);
                };
            }
        },
        Err(e) => panic!("Failed to parse: {}", e),
    }

    accum
}

#[derive(Debug)]
struct SafetyReport {
    repo_url: String,
    status: bool,
    offenses: Vec<HashMap<String, String>>,
}

impl SafetyReport {
    pub fn new(repo_url: String,
               status: bool,
               offenses: Vec<HashMap<String, String>>) -> SafetyReport{
        SafetyReport {repo_url: repo_url, status: status, offenses: offenses}
    }
}

impl fmt::Display for SafetyReport {
    // Displays in serialized json
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{\"repo_url\":\"{}\",\"status\":{},\"offenses\":{:?}}}",
               self.repo_url,
               self.status.to_string(),
               self.offenses)
    }
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
            println!("{}", report);
        }
        None => {
            return println!("Please provide a git repo url.");
        }
    };
}
