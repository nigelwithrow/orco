//! JavaScript/TypeScript transpilation backend for orco
use std::collections::HashMap;

use orco::{codegen as oc, *};

struct FunctionSignature {
    pub name: String,
    pub params: Vec<Symbol>,
    pub ret: Symbol,
}

#[derive(Default)]
pub struct Backend {
    sigs: HashMap<orco::Symbol, FunctionSignature>,
    defs: std::sync::Mutex<Vec<String>>,
    types: bool,
}

impl Backend {}

enum Types {
    Undefined,
    Boolean,
    Number,
}

impl std::fmt::Display for Types {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Types::Undefined => write!(f, "undefined"),
            Types::Boolean => write!(f, "boolean"),
            Types::Number => write!(f, "number"),
        }
    }
}

// impl std::str::FromStr for Types {
//     type Err = ();

//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         Ok(match s {
//             "undefined" => Types::Undefined,
//             "boolean" => Types::Boolean,
//             "number" => Types::Number,
//             _ => return Err(())
//         })
//     }
// }

impl PartialEq<str> for Types {
    fn eq(&self, other: &str) -> bool {
        match (self, other) {
            (Types::Undefined, "undefined") => true,
            (Types::Boolean, "boolean") => true,
            (Types::Number, "number") => true,
            _ => false,
        }
    }
}

impl PrimitiveTypeSource for Backend {
    fn unit(&self) -> Type {
        Type::Symbol(Types::Undefined.to_string().into())
    }

    fn bool(&self) -> Type {
        Type::Symbol(Types::Boolean.to_string().into())
    }

    fn int(&self, _size: u16, _signedness: bool) -> Type {
        Type::Symbol(Types::Number.to_string().into())
    }

    fn size_type(&self, _signedness: bool) -> Type {
        Type::Symbol(Types::Number.to_string().into())
    }

    fn float(&self, _size: u16) -> Type {
        Type::Symbol(Types::Number.to_string().into())
    }
}

impl DeclarationBackend for Backend {
    fn declare_function(
        &mut self,
        name: Symbol,
        params: &[(Option<Symbol>, Type)],
        return_type: &Type,
    ) {
        assert!(
            !self.sigs.contains_key(&name),
            "function {name:?} is already declared!"
        );

        let Type::Symbol(ret) = return_type else {
            panic!("error type");
        };
        self.sigs.insert(
            name,
            FunctionSignature {
                name: crate::escape(name),
                params: params
                    .into_iter()
                    .map(|(_, ty)| match ty {
                        Type::Symbol(sym) => *sym,
                        Type::Error => panic!("error type"),
                    })
                    .collect(),
                ret: *ret,
            },
        );
    }
}

pub struct FooFunction<'a> {
    backend: &'a Backend,
    code: String,
    indent: usize,
    variables: Vec<Symbol>,
}

impl<'a> FooFunction<'a> {
    fn line(&mut self, line: &str) {
        self.code.reserve(self.indent + line.len() + 1);
        self.code.extend(std::iter::repeat_n(' ', self.indent));
        self.code.push_str(line);
        self.code.push('\n');
    }
}

struct Var<'a>(&'a oc::Variable);

impl<'a> std::fmt::Display for Var<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "_{}", self.0.0)
    }
}

struct Op<'a>(&'a oc::Operand);

impl<'a> std::fmt::Display for Op<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            oc::Operand::Variable(variable) => write!(f, "{}", Var(variable)),
            oc::Operand::IConst(val) => write!(f, "{val}"),
            oc::Operand::UConst(val) => write!(f, "{val}"),
            oc::Operand::Unit => write!(f, "{}", Types::Undefined),
        }
    }
}

impl<'a> oc::Codegen<'a> for FooFunction<'a> {
    fn comment(&mut self, comment: &str) {
        for line in comment.split('\n') {
            self.line(&format!("{line}"));
        }
    }

    fn declare_var(&mut self, ty: &Type) -> oc::Variable {
        let Type::Symbol(ty) = ty else {
            panic!("error type");
        };
        let id = self.variables.len();
        let var = oc::Variable(id);
        self.variables.push(ty.to_owned());

        if &Types::Undefined == &**ty {
            return var;
        }

        if self.backend.types {
            self.line(&format!("let {name}: {ty}", name = Var(&var)));
        } else {
            self.line(&format!("let {name}", name = Var(&var)));
        }
        var
    }

    fn arg_var(&self, idx: usize) -> oc::Variable {
        oc::Variable(idx)
    }

    fn cast(&mut self, value: oc::Operand, result: oc::Variable) {
        use oc::Operand::*;
        let is_unit = match value {
            Variable(oc::Variable(id)) if &Types::Undefined == &*self.variables[id] => true,
            Unit => true,
            _ => false,
        };
        match (is_unit, self.backend.types) {
            (true, true) => self.comment(&format!(
                "{name} = ((({op}) as any) as {ty})",
                name = Var(&result),
                op = Op(&value),
                ty = self.variables[result.0]
            )),
            (true, false) => self.comment(&format!(
                "{name} = {op}",
                name = Var(&result),
                op = Op(&value)
            )),
            (false, true) => self.line(&format!(
                "{name} = ((({op}) as any) as {ty})",
                name = Var(&result),
                op = Op(&value),
                ty = self.variables[result.0]
            )),
            (false, false) => self.line(&format!(
                "{name} = {op}",
                name = Var(&result),
                op = Op(&value)
            )),
        }
    }

    fn return_(&mut self, value: oc::Operand) {
        self.line(&format!("return {op}", op = Op(&value)));
    }
}

impl<'a> Drop for FooFunction<'a> {
    fn drop(&mut self) {
        self.code.push_str("}");
        self.backend
            .defs
            .lock()
            .unwrap()
            .push(std::mem::take(&mut self.code));
    }
}

impl DefinitionBackend for Backend {
    fn define_function(&self, name: Symbol) -> impl oc::Codegen<'_> {
        let sig = self
            .sigs
            .get(&name)
            .unwrap_or_else(|| panic!("tried to define undeclared function '{name}'"));
        let mut codegen = FooFunction {
            backend: self,
            code: format!("function {name}(", name = sig.name),
            indent: 4,
            variables: Vec::new(),
        };
        use std::fmt::Write as _;
        for (idx, ty) in sig.params.iter().enumerate() {
            if idx > 0 {
                codegen.code.push_str(", ");
            }
            if self.types {
                write!(codegen.code, "{name}: {ty}", name = Var(&oc::Variable(idx))).unwrap();
            } else {
                write!(codegen.code, "{name}", name = Var(&oc::Variable(idx))).unwrap();
            }
            codegen.variables.push(*ty);
        }
        if self.types {
            _ = write!(&mut codegen.code, "): {ty} {{\n", ty = sig.ret);
        } else {
            codegen.code.push_str(") {\n");
        }
        codegen
    }
}

pub fn escape(symbol: orco::Symbol) -> String {
    symbol
        .as_str()
        .replace("::", "_")
        .replace(['.', ':', '/', '-'], "_")
}
