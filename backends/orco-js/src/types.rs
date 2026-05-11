use crate::SymbolKind;

/// A thin wrapper around [`orco::Type`] for formatting it as a Typescript type.
#[allow(missing_docs)]
pub struct FmtType<'a, 'b>(pub &'a orco::Type, pub &'a crate::Backend<'b>);

impl std::fmt::Display for FmtType<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(ty, backend @ crate::Backend { config, .. }) = *self;

        use orco::Type as OT;
        match ty {
            OT::Integer(_) | OT::Unsigned(_) | OT::Float(_) => write!(f, "number"),

            OT::Bool => write!(f, "boolean"),

            OT::Symbol(sym) => write!(f, "{}", crate::symname(*sym)),

            OT::Array(ty, sz) => match *sz {
                0 => write!(f, "[]"),
                1 => write!(f, "[{}]", FmtType(ty, backend)),
                n if n <= config.max_tuple_type_size => {
                    let inner_ty = FmtType(ty, backend).to_string();
                    write!(f, "[{inner_ty}")?;
                    for _ in 0..(n - 1) {
                        write!(f, ", {inner_ty}")?;
                    }
                    write!(f, "]")
                }
                _ => write!(f, "({}[])", FmtType(ty, backend)),
            },

            OT::Struct { fields } if fields.is_empty() => {
                write!(f, "{{}}")
            }
            OT::Struct { fields } => {
                writeln!(f, "{{")?;
                for (idx, (name, ty)) in fields.iter().enumerate() {
                    writeln!(
                        f,
                        " {name}:  {ty},",
                        name = name
                            .as_deref()
                            .map(std::borrow::Cow::Borrowed)
                            .unwrap_or_else(|| format!("_{idx}").into()),
                        ty = FmtType(ty, backend),
                    )?;
                }
                write!(f, "}}")
            }

            OT::Ptr(ty, mutable) => {
                let mut ptr_of_type = |ty: &orco::Type| {
                    match ty {
                        // pointer to a copy type `T` is `{ get: () => T, set: (v: T) => void }`
                        ty @ (OT::Integer(_)
                        | OT::Unsigned(_)
                        | OT::Float(_)
                        | OT::Bool
                        | OT::Error) => make_ptr(f, *mutable, FmtType(ty, backend)),

                        // pointer to a reference type `T` is `T`
                        ty @ (OT::Array(_, _)
                        | OT::Struct { .. }
                        | OT::Ptr(_, _)
                        | OT::FnPtr { .. }) => {
                            write!(f, "{}", FmtType(ty, backend))
                        }

                        OT::Symbol(_) => unreachable!("symbol is resolved at this point"),
                    }
                };

                match &**ty {
                    OT::Symbol(sym) => {
                        let sym_kind = backend.get_symbol(sym);
                        let SymbolKind::Type(t) = sym_kind.get() else {
                            unreachable!("function symbols cannot appear in types");
                        };
                        ptr_of_type(t)
                    }
                    t => ptr_of_type(t),
                }
            }

            OT::FnPtr {
                params,
                return_type,
            } => {
                fn id() -> usize {
                    unsafe {
                        static mut I: usize = 0;
                        let out = I;
                        I += 1;
                        out
                    }
                }

                // print params list
                write!(f, "(");
                let mut params = params.iter();
                if let Some(fp) = params.next() {
                    write!(f, "_{}: {}", id(), FmtType(fp, backend))?;
                    params.try_for_each(|ty| write!(f, ", _{}: {}", id(), FmtType(ty, backend)))?;
                }
                write!(f, ")");

                // print return type
                write!(f, " => ");
                if let Some(return_ty) = return_type {
                    write!(f, "{}", FmtType(return_ty, backend))
                } else {
                    write!(f, "void")
                }
            }

            OT::Error => write!(f, "<error-type>"),
        }
    }
}

// given an `inner_ty` "T", writes:
// - "{ get: () => T, set: (v: T) => void }" if `mutable`
// - "{ get: () => T }" if not `mutable`
fn make_ptr(f: &mut std::fmt::Formatter, mutable: bool, inner_ty: FmtType) -> std::fmt::Result {
    write!(f, "{{get: () => {}", inner_ty)?;
    if mutable {
        write!(f, ", set: (v: {}) => void", inner_ty)?;
    }
    write!(f, "}}")?;
    Ok(())
}
