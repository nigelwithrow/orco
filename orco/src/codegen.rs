//! Code generation APIs, used to actually define functions and generate code.
use crate::types::IntegerSize;
use crate::{Symbol, Type};

/// Implementations of codegen features
pub mod impls;

/// A variable ID. Variable is a mutable storage, either in RAM or CPU registers
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Variable(pub usize);

/// A variable or symbol with projection (aka field access, dereferences, etc.)
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Place<Value> {
    /// Just variable access
    Variable(Variable),
    /// Global symbol access
    Global(Symbol),
    /// Pointer dereference
    Deref(Value),
    /// Field access, using 0-based field index
    Field(Value, usize),
}

impl<V> From<Variable> for Place<V> {
    fn from(value: Variable) -> Self {
        Self::Variable(value)
    }
}

/// Trait for generating code within a function
pub trait BodyCodegen {
    /// Values are immutable results of operations.
    type Value;
    /// Get type of the value
    fn type_of(&self, value: &Self::Value) -> Type;

    /// Declare a variable, see [Variable]
    fn declare_var(&mut self, ty: Type) -> Variable;
    /// Get the variable representing an argument
    fn arg_var(&self, idx: usize) -> Variable;

    /// Assign a value into a place, which makes it reusable
    fn assign(&mut self, target: Place<Self::Value>, value: Self::Value);
    /// Makes a temproary variable and assigns the value to it. Utility function
    fn mk_tmp(&mut self, value: Self::Value) -> Variable {
        let tmp = self.declare_var(self.type_of(&value));
        self.assign(tmp.into(), value);
        tmp
    }

    /// Signed integer constant
    fn iconst(&mut self, value: i128, size: IntegerSize) -> Self::Value;
    /// Unsigned integer constant
    fn uconst(&mut self, value: u128, size: IntegerSize) -> Self::Value;
    /// Float constant
    fn fconst(&mut self, value: f64, size: u16) -> Self::Value;

    /// Read value from a [Place]
    fn read(&mut self, place: Place<Self::Value>) -> Self::Value;

    /// Return a value from the current function.
    fn return_(&mut self, value: Option<Self::Value>);
    /// Get arbitrary control flow instructions, see [ACFCodegen]
    fn acf(&mut self) -> &mut impl ACFCodegen<Self::Value> {
        Box::leak(Box::new(impls::Unsupported))
    }
}

/// A label ID. See [`ACFCodegen::label`]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Label(pub usize);

/// Arbitrary control flow instructions, such as jumps.
/// Warning: Not all codegens implement arbitrary control flow
pub trait ACFCodegen<Value> {
    /// Puts a said label in the current position.
    /// Note: Labels can be used before placing. Frontend decides on IDs
    fn label(&mut self, label: Label);

    /// Jump to a label.
    /// See [`ACFCodegen::label`]
    fn jump(&mut self, label: Label);

    /// Jumps if condition is true.
    /// See [`ACFCodegen::label`]
    fn cjump(&mut self, condition: Value, label: Label);
}

/// Interface for generating actual code.
/// All the items defined must be declared using [crate::DeclarationBackend] first.
pub trait CodegenBackend: Sync {
    /// Define a function
    fn function(&self, name: Symbol) -> impl BodyCodegen;
}
