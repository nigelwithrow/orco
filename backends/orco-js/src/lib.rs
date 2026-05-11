//! Javascript/Typesript backend for orco.
//! Also used to generate Typescript definition files (.d.ts) [WIP]
//! See [Backend]

/// Type formatting & other things
pub mod types;
use types::FmtType;

/// Symbol container types
pub mod symbols;
pub use symbols::SymbolKind;

/// Code generation, used to generate function bodies.
pub mod codegen;
pub use codegen::Codegen;

/// Root backend struct
#[derive(Debug, Default)]
pub struct Backend<'a> {
    /// A map from symbol to their declarations
    pub symbols: scc::HashMap<orco::Symbol, SymbolKind>,
    /// Interned types
    interned: scc::HashSet<orco::Symbol>,
    /// Definitions
    definitions: scc::Stack<String>,
    /// The default macro handler
    pub macros: orco::impls::MacroServer<'a>,
    /// Backend Configuration
    pub config: Config,
}

#[derive(Debug)]
pub struct Config {
    max_tuple_type_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_tuple_type_size: 64,
        }
    }
}

impl Backend<'_> {
    /// Declares a symbol
    pub fn symbol(&self, name: orco::Symbol, kind: SymbolKind) {
        self.symbols
            .entry_sync(name)
            .and_modify(|_| panic!("symbol {name:?} is already declared"))
            .or_insert(kind);
    }

    /// Returns a previously declared symbol
    pub fn get_symbol(
        &self,
        name: &orco::Symbol,
    ) -> scc::hash_map::OccupiedEntry<'_, orco::Symbol, SymbolKind> {
        self.symbols
            .get_sync(name)
            .unwrap_or_else(|| panic!("undeclared symbol {name}"))
    }

    /// Intern the following type and it's insides.
    pub fn intern_type(&self, ty: &mut orco::Type, named: bool) {
        match ty {
            orco::Type::Array(ty, _) => {
                self.intern_type(ty.as_mut(), false) // TODO: More work on arrays
            }
            orco::Type::Struct { fields } if named => {
                for (_, ty) in fields {
                    self.intern_type(ty, false);
                }
            }
            orco::Type::Struct { fields } if !named => {
                let sym = orco::Symbol::new(&format!("s {}", ty.hashable_name()));
                let ty = std::mem::replace(ty, orco::Type::Symbol(sym));
                if self.interned.insert_sync(sym).is_ok() {
                    use orco::DeclarationBackend as _;
                    self.type_(sym, ty);
                }
            }
            _ => (),
        }
    }
}

impl<'a> orco::DeclarationBackend<'a> for Backend<'a> {
    fn function(
        &self,
        name: orco::Symbol,
        mut params: Vec<(Option<String>, orco::Type)>,
        mut return_type: Option<orco::Type>,
        attrs: orco::attrs::FunctionAttributes,
    ) {
        for (_, ty) in &mut params {
            self.intern_type(ty, false);
        }
        if let Some(rt) = &mut return_type {
            self.intern_type(rt, false);
        }
        self.symbol(
            name,
            SymbolKind::Function(orco::types::FunctionSignature {
                attrs,
                params,
                return_type,
            }),
        );
    }

    fn type_(&self, name: orco::Symbol, mut ty: orco::Type) {
        self.intern_type(&mut ty, true);
        self.symbol(name, SymbolKind::Type(ty));
    }

    fn macro_(
        &self,
        name: orco::Symbol,
        callback: impl Fn(&[orco::Type]) + Send + Sync + 'a,
        call_once: bool,
    ) {
        self.macros.macro_(name, callback, call_once)
    }

    fn invoke_macro(&self, name: orco::Symbol, args: &[orco::Type]) {
        self.macros.invoke_macro(name, args);
    }
}

/* impl orco::CodegenBackend for crate::Backend<'_> {
    fn function(&self, name: orco::Symbol) -> impl orco::codegen::BodyCodegen {
        codegen::Codegen::new(self, name)
    }
} */

/// Get the name of the symbol used in generated code
fn symname(symbol: orco::Symbol) -> String {
    // TODO: Needs work

    let mut symbol = symbol
        .replace(':', ".") // Replace all `:`s with `.`s
        .replace(|c: char| !c.is_ascii_alphanumeric(), "_"); // Replace forbidden characters with _
    // Add leading _ in case symbol starts with a digit
    if symbol.chars().next().is_none_or(|c| c.is_ascii_digit()) {
        symbol.insert(0, '_');
    }

    symbol
}
