use crate::{BackendContext, SymbolKind};

// FIXME: Proof of concept
/// [`crate::Backend`] wrapper that wraps all symbols in generic macros.
/// For more info, see [`orco::Backend::generic`]
pub(super) struct Wrapper<'a> {
    /// Original backend
    pub backend: &'a crate::Backend,
    /// Macro params
    pub params: Vec<String>,
}

impl BackendContext for Wrapper<'_> {
    fn backend(&self) -> &crate::Backend {
        self.backend
    }

    fn macro_context(&self) -> bool {
        true
    }

    /// Adds a symbol, wrapping it with generics
    fn symbol(&self, name: orco::Symbol, kind: SymbolKind) {
        self.backend.symbol(
            name,
            SymbolKind::Generic {
                params: self.params.clone(),
                symbol: Box::new(kind),
            },
        );
    }

    fn define(&self, _code: String) {
        todo!()
    }

    fn intern_type(&self, ty: &mut orco::Type, named: bool) {
        // TODO: Interned types in generics
        self.backend.intern_type(ty, named)
    }
}

impl orco::DeclarationBackend for Wrapper<'_> {
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
            SymbolKind::Function(crate::symbols::FunctionSignature {
                attrs,
                params,
                return_type,
            }),
        );
    }

    fn type_(&self, name: orco::Symbol, mut ty: orco::Type) {
        self.backend.intern_type(&mut ty, true);
        self.symbol(name, SymbolKind::Type(ty));
    }

    fn generic(&self, params: Vec<String>) -> impl orco::DeclarationBackend {
        Self {
            backend: self.backend,
            params: self.params.iter().cloned().chain(params).collect(),
        }
    }
}
