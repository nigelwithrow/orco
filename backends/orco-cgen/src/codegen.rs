use crate::{BackendContext, SymbolKind};
use orco::codegen as oc;
use std::collections::HashMap;

/// Implementation of [`orco::BodyCodegen`]
pub struct Codegen<'a, B: BackendContext> {
    /// Backend context that will recieve the symbol once codegen is done
    pub ctx: &'a B,
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

struct ValueInfo {
    /// Expression for this value, will either be placed whenever
    /// the value is used or placed in the code whenever the value is flushed.
    expression: String,
    ty: orco::Type,
}

impl ValueInfo {
    fn new(expression: String, ty: orco::Type) -> Self {
        Self { expression, ty }
    }
}

impl<'a, B: BackendContext> Codegen<'a, B> {
    #[allow(missing_docs)]
    pub fn new(ctx: &'a B, name: orco::Symbol) -> Self {
        let mut this = Self {
            ctx,
            name,

            body: "{\n".to_owned(),
            indent: 1,

            variables: Vec::new(),
            arg_count: 0,

            values: HashMap::new(),
            next_value_index: 0,
        };

        let symbol = ctx.backend().get_symbol(this.name);
        let symbol = symbol.get().skip_generics();
        if let SymbolKind::Function(signature) = symbol {
            this.body = format!(
                "{} {{\n",
                crate::symbols::FmtFunction {
                    macro_context: ctx.macro_context(),
                    name: &ctx.symname(name), // FIXME: Generics
                    signature,
                    name_all_args: true
                }
            );

            for (idx, (name, ty)) in signature.params.iter().enumerate() {
                let name = name.clone().unwrap_or_else(|| format!("_{idx}"));
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

    /// Adds indent to the body
    fn indent(&mut self) {
        for _ in 0..self.indent {
            self.body.push_str("  ");
        }
    }

    /// Add a statement
    pub fn stmt(&mut self, statement: &str) {
        for line in statement.split('\n') {
            self.indent();
            self.body.push_str(line);
            self.body.push('\n');
        }
    }

    /// Make a value and put it in [`Self::values`]
    fn mk_value(&mut self, value: ValueInfo) -> oc::Value {
        let id = oc::Value(self.next_value_index);
        self.next_value_index += 1;
        self.values.insert(id.0, value);
        id
    }

    /// Use a value, returns it's expression, must be placed in the code to avoid
    /// missing side effects.
    fn use_value(&mut self, value: oc::Value) -> ValueInfo {
        self.values.remove(&value.0).unwrap_or_else(|| {
            panic!(
                "value #{} either doesn't exist or was used already",
                value.0
            )
        })
    }

    /// Convert [`oc::Place`] to a value (C code string + type)
    fn place(&mut self, place: oc::Place) -> ValueInfo {
        match place {
            oc::Place::Variable(variable) => {
                let variable = &self.variables[variable.0];
                ValueInfo::new(variable.name.clone(), variable.ty.clone())
            }
            oc::Place::Global(name) => {
                let symbol = self.ctx.backend().get_symbol(name);
                ValueInfo::new(
                    self.ctx.symname(name),
                    match symbol.get() {
                        SymbolKind::Function(signature) => signature.ptr_type(),
                        _ => panic!("trying to access {name} as a value, but it is {symbol:?}"),
                    },
                )
            }
            oc::Place::Deref(value) => {
                let value = self.use_value(value);
                ValueInfo::new(
                    format!("*({})", value.expression),
                    match self.ctx.backend().inline_type_aliases(value.ty) {
                        orco::Type::Ptr(ty, _) => *ty,
                        ty => panic!("trying to dereference a non-pointer type {ty:#?}"),
                    },
                )
            }
            oc::Place::Field(_, _) => todo!("field access"),
        }
    }
}

impl<B: BackendContext> oc::BodyCodegen for Codegen<'_, B> {
    fn type_of(&self, id: usize) -> orco::Type {
        self.values[&id].ty.clone()
    }

    fn declare_var(&mut self, mut ty: orco::Type) -> oc::Variable {
        self.ctx.intern_type(&mut ty, false, false);
        let id = self.variables.len();
        let name = format!("v{}", id);

        if !matches!(&ty, orco::Type::Struct { fields } if fields.is_empty()) {
            self.stmt(&format!(
                "{};",
                crate::types::FmtType {
                    macro_context: false,
                    ty: &ty,
                    name: Some(&name),
                }
            ));
        }

        self.variables.push(VariableInfo { name, ty });
        oc::Variable(id)
    }

    fn arg_var(&self, idx: usize) -> oc::Variable {
        assert!(
            idx < self.arg_count,
            "trying to access argument #{idx}, but there are only {} arguments.",
            self.arg_count
        );
        oc::Variable(idx)
    }

    fn assign(&mut self, target: oc::Place, value: oc::Value) {
        let target = self.place(target).expression;
        let value = self.use_value(value).expression;
        self.stmt(&format!("{target} = {value}"));
    }

    fn iconst(&mut self, value: i128, size: orco::types::IntegerSize) -> oc::Value {
        self.mk_value(ValueInfo::new(value.to_string(), orco::Type::Integer(size))) // TODO: Literal sizes
    }

    fn uconst(&mut self, value: u128, size: orco::types::IntegerSize) -> oc::Value {
        self.mk_value(ValueInfo::new(
            value.to_string(),
            orco::Type::Unsigned(size),
        )) // TODO: Literal sizes
    }

    fn fconst(&mut self, value: f64, size: u16) -> oc::Value {
        self.mk_value(ValueInfo::new(value.to_string(), orco::Type::Float(size))) // TODO: Literal sizes
    }

    fn read(&mut self, place: oc::Place) -> oc::Value {
        let place = self.place(place);
        self.mk_value(place)
    }

    fn return_(&mut self, value: Option<oc::Value>) {
        if let Some(value) = value {
            let value = self.use_value(value).expression;
            self.stmt(&format!("return {value};"));
        } else {
            self.stmt("return;");
        }
    }
}

impl<B: BackendContext> std::ops::Drop for Codegen<'_, B> {
    fn drop(&mut self) {
        self.body.push('}');
        self.ctx.define(std::mem::take(&mut self.body));
    }
}
