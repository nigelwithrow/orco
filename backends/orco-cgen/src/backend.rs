use super::*;

/// Root backend struct
#[derive(Debug, Default)]
pub struct Backend {
    /// A map from symbol to it's declaration
    pub symbols: scc::HashMap<orco::Symbol, SymbolKind>,
    /// Interned types
    interned: scc::HashSet<orco::Symbol>,
    /// Definitions
    definitions: scc::Stack<String>,
}

impl Backend {
    #[allow(missing_docs)]
    pub fn new() -> Backend {
        Self::default()
    }

    /// Retursn a previously declared symbol
    pub fn get_symbol(
        &self,
        name: orco::Symbol,
    ) -> scc::hash_map::OccupiedEntry<'_, orco::Symbol, SymbolKind> {
        self.symbols
            .get_sync(&name)
            .unwrap_or_else(|| panic!("undeclared symbol {}", name))
    }

    /// If ty is a type alias (but not a struct), inlines it.
    /// Does not inline inner types
    pub fn inline_type_aliases(&self, ty: orco::Type, inline_struct: bool) -> orco::Type {
        match ty {
            orco::Type::Symbol(symbol) => {
                let symbol = self.get_symbol(symbol);
                match symbol.get() {
                    SymbolKind::Type(ty)
                        if inline_struct || !matches!(ty, orco::Type::Struct { .. }) =>
                    {
                        self.inline_type_aliases(ty.clone(), inline_struct)
                    }
                    _ => ty,
                }
            }
            ty => ty,
        }
    }
}

impl BackendContext for Backend {
    fn backend(&self) -> &Backend {
        self
    }

    fn macro_context(&self) -> bool {
        false
    }

    fn symbol(&self, name: orco::Symbol, kind: SymbolKind) {
        self.symbols
            .entry_sync(name)
            .and_modify(|_| panic!("symbol {name:?} is already declared"))
            .or_insert(kind);
    }

    fn define(&self, code: String) {
        self.definitions.push(code);
    }

    fn intern_type(&self, ty: &mut orco::Type, named: bool) {
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

impl orco::DeclarationBackend for Backend {
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
            SymbolKind::Function(symbols::FunctionSignature {
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

    fn generic(&self, params: Vec<String>) -> impl orco::DeclarationBackend {
        generics::Wrapper {
            backend: self,
            params,
        }
    }
}

impl orco::CodegenBackend for crate::Backend {
    fn function(&self, name: orco::Symbol) -> impl orco::codegen::BodyCodegen {
        codegen::Codegen::new(self, name)
    }
}

impl std::fmt::Display for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "#include <stdint.h>")?;
        writeln!(f, "#include <stddef.h>")?;
        writeln!(f, "#include <stdbool.h>")?;
        writeln!(f)?;

        let mut result = Ok(());
        self.symbols.iter_sync(|name, sym| {
            let sym = format!(
                "{}",
                symbols::FmtSymbol {
                    backend: self,
                    name: &symname(*name, false),
                    kind: sym,
                    macro_context: false,
                }
            );
            result = writeln!(
                f,
                "{}{}",
                sym,
                if sym.lines().count() > 1 { "\n" } else { "" }
            );
            result.is_ok()
        });
        result?;

        for def in self.definitions.iter(&scc::Guard::new()) {
            writeln!(f, "{def}\n")?;
        }

        Ok(())
    }
}
