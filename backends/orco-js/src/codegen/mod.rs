use crate::{Backend, SymbolKind};
use std::collections::HashMap;

mod value;
use value::ValueInfo;

/// Implementation of [`orco::BodyCodegen`]
pub struct Codegen<'a, 'b: 'a> {
    /// Backend context that will recieve the symbol once codegen is done
    pub backend: &'a Backend<'b>,
    /// Symbol name
    pub name: orco::Symbol,

    /// Currently generated function body as a string
    body: String,
    /// Current indentation level
    indent: usize,

    /// A variable info list. Variables never get removed,
    /// this can be indexed using [`Variable::0`] directly
    variables: Vec<VariableInfo>,
    /// Number of arguments this function has
    arg_count: usize,

    /// Map of [`Value::0`] to value info. Entries get
    /// removed whenever values get used
    values: HashMap<usize, ValueInfo>,
    next_value_index: usize,
}

struct VariableInfo {
    name: String,
    ty: orco::Type,
}

impl<'a, 'b: 'a> Codegen<'a, 'b> {
    #[allow(missing_docs)]
    pub fn new(ctx: &'a Backend<'b>, name: orco::Symbol) -> Self {
        let mut this = Self {
            backend: ctx,
            name,

            body: "{\n".to_owned(),
            indent: 1,

            variables: Vec::new(),
            arg_count: 0,

            values: HashMap::new(),
            next_value_index: 0,
        };

        let symbol = ctx.get_symbol(&this.name);
        let symbol = symbol.get();
        if let SymbolKind::Function(signature) = symbol {
            this.body = format!(
                "{} {{\n",
                crate::symbols::FmtFunction {
                    name: &crate::symname(name), // FIXME: Generics
                    signature: &signature,
                    backend: ctx,
                }
            );

            for (idx, (name, ty)) in signature.params.iter().enumerate() {
                let name = name.clone().unwrap_or_else(|| format!("arg{idx}"));
                this.variables.push(VariableInfo {
                    ty: ty.clone(),
                    name,
                });
            }
            this.arg_count = signature.params.len();
        } else {
            panic!("Trying to define a non-function symbol {symbol:#?}")
        }

        this
    }
}
