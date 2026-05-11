use std::borrow::Cow;

use orco::types::FunctionSignature;

use crate::types::FmtType;

/// Reference to a declaration
#[derive(Debug)]
pub enum SymbolKind {
    /// Function, see [FunctionSignature]
    Function(FunctionSignature),
    /// Type alias, aka `type` or `interface`
    Type(orco::Type),
}

/// Formats function signature
pub struct FmtFunction<'a, 'b> {
    /// Function name
    pub name: &'a str,
    /// Function signature
    pub signature: &'a FunctionSignature,
    /// Reference to the backend, required for symbol resolutions
    pub backend: &'a crate::Backend<'b>,
}

impl std::fmt::Display for FmtFunction<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let FmtFunction {
            name,
            signature,
            backend,
        } = *self;

        use orco::attrs as oa;
        match signature.attrs.inlining {
            oa::Inlining::Auto | oa::Inlining::Hint => (),
            // NOTE: if there was a way to emit a warning, it would be done here
            oa::Inlining::Never | oa::Inlining::Always => (),
        }

        // TODO HERE: cgen uses this for both function defn and decl but JS has different syntax for
        // these things
        write!(f, "declare function {}(", name)?;
        for (idx, (name, ty)) in signature.params.iter().enumerate() {
            if idx > 0 {
                write!(f, ", ")?;
            }
            write!(
                f,
                "{name}: {ty}",
                name = name
                    .as_ref()
                    .map_or_else(|| Cow::Owned(format!("_{idx}")), Cow::Borrowed),
                ty = FmtType(ty, backend),
            )?;
        }
        write!(f, "): ")?;

        if let Some(return_ty) = &signature.return_type {
            write!(f, "{}", FmtType(return_ty, backend))?;
        } else {
            write!(f, "void")?;
        }
        write!(f, ";\n")
    }
}
