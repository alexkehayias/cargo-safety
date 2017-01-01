use std::collections::{HashSet};
use syntex_syntax::codemap::{CodeMap, Span};
use syntex_syntax::ast::{NodeId, Block, FnDecl, Mac, Unsafety, BlockCheckMode,
                         TraitItem, ImplItemKind, ImplItem, TraitItemKind,
                         Attribute, MetaItemKind, Item, ItemKind};
use syntex_syntax::visit::{self, Visitor, FnKind};


#[allow(non_camel_case_types)]
#[derive(Hash, Eq, PartialEq, Debug, Clone, Serialize)]
enum UnsafeKind {
    unsafe_function,
    unsafe_impl,
    unsafe_impl_item,
    unsafe_block,
    unsafe_trait,
    unsafe_trait_item,
    unsafe_attr,
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
    pub locations: HashSet<UnsafeCode>,
    // Used to go from a Span to line:column information
    pub codemap: &'a CodeMap,
}

// Unsafe code can be introduced in functions, blocks, traits, and implementations
impl<'a> Visitor for UnsafeCrate<'a> {
    // Implement this otherwise it will panic when it hits a macro
    fn visit_mac(&mut self, _mac: &Mac) {}

    // Capture unsafe items i.e unsafe impl Trait for Foo
    fn visit_item(&mut self, item: &Item) {
        match item.node {
            ItemKind::Impl(unsafety, ..) => {
                match unsafety {
                    Unsafety::Normal => (),
                    Unsafety::Unsafe => {
                        let record = UnsafeCode::new(
                            UnsafeKind::unsafe_impl,
                            self.codemap.span_to_expanded_string(item.span),
                        );
                        self.locations.insert(record);
                    },
                }
            },
            ItemKind::Trait(unsafety, ..) => {
                match unsafety {
                    Unsafety::Normal => (),
                    Unsafety::Unsafe => {
                        let record = UnsafeCode::new(
                            UnsafeKind::unsafe_trait,
                            self.codemap.span_to_expanded_string(item.span),
                        );
                        self.locations.insert(record);
                    },
                }
            },
            _ => (),
        }
    }

    // Recursively capture all occurences of unsafe functions
    fn visit_fn(&mut self,
                fn_kind: FnKind,
                fn_decl: &FnDecl,
                span: Span,
                _id: NodeId) {
        match fn_kind {
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
            },
            _ => (),
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

    // Capture any unsafe traits
    fn visit_trait_item(&mut self, ti: &TraitItem) {
        match ti.node {
            TraitItemKind::Method(ref sig, _) => match sig.unsafety {
                Unsafety::Normal => (),
                Unsafety::Unsafe => {
                    let record = UnsafeCode::new(
                        UnsafeKind::unsafe_trait_item,
                        self.codemap.span_to_expanded_string(ti.span),
                    );
                    self.locations.insert(record);
                },
            },
            _ => (),
        };
    }

    // Capture any unsafe implementations
    fn visit_impl_item(&mut self, ii: &ImplItem) {
        match ii.node {
            ImplItemKind::Method(ref sig, _) => match sig.unsafety {
                Unsafety::Normal => (),
                Unsafety::Unsafe => {
                    let record = UnsafeCode::new(
                        UnsafeKind::unsafe_impl_item,
                        self.codemap.span_to_expanded_string(ii.span),
                    );
                    self.locations.insert(record);
                }
            },
            _ => (),
        };
    }

    // Capture unsafe destructor attribute i.e #["unsafe_destructor_blind_to_params"]
    fn visit_attribute(&mut self, attr: &Attribute) {
        match attr.value.node {
            MetaItemKind::Word =>
                if attr.value.name == "unsafe_destructor_blind_to_params" {
                    let record = UnsafeCode::new(
                        UnsafeKind::unsafe_attr,
                        self.codemap.span_to_expanded_string(attr.span),
                    );
                    self.locations.insert(record);
                },
            _ => (),
        };
    }
}
