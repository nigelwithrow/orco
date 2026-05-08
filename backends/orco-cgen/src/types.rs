/// A thin wrapper around [`orco::Type`] for formatting it as a C type.
/// Because C loves types to influence postfixes (aka arrays and function pointers),
/// also wraps optional name (variable name, parameter name, type name in typedef)
#[allow(missing_docs)]
pub struct FmtType<'a> {
    pub ty: &'a orco::Type,
    pub name: Option<&'a str>,
}

impl std::fmt::Display for FmtType<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let FmtType { ty, name } = *self;

        use orco::Type as OT;
        use orco::types::IntegerSize as IS;
        match ty {
            OT::Integer(size) => match size {
                IS::Bits(bits) => {
                    assert!(
                        [8, 16, 32, 64].contains(bits),
                        "invalid or unsupported integer bit width {bits}"
                    );

                    // TODO: __int128_t
                    write!(f, "int{bits}_t")
                }
                IS::Size => write!(f, "ssize_t"),
            },
            OT::Unsigned(size) => match size {
                IS::Bits(bits) => {
                    assert!(
                        [8, 16, 32, 64].contains(bits),
                        "invalid or unsupported integer bit width {bits}"
                    );

                    // TODO: unsigned __int128_t
                    write!(f, "uint{bits}_t")
                }
                IS::Size => write!(f, "size_t"),
            },
            OT::Float(size) => match size {
                32 => write!(f, "float"),
                64 => write!(f, "double"),
                size => {
                    // TODO: f16 and f128
                    panic!("invalid or unsupported floating point type size {size} bits")
                }
            },
            OT::Bool => write!(f, "bool"),
            OT::Symbol(sym) => write!(f, "{}", crate::symname(*sym)),

            OT::Array(ty, sz) => {
                return write!(f, "{}[{sz}]", FmtType { ty, name });
            }
            OT::Struct { fields } if fields.is_empty() => {
                write!(f, "struct {{}}")
            }
            OT::Struct { fields } => {
                writeln!(f, "struct {{")?;
                for (idx, (name, ty)) in fields.iter().enumerate() {
                    writeln!(
                        f,
                        "  {};",
                        FmtType {
                            ty,
                            name: Some(
                                name.as_deref()
                                    .map(std::borrow::Cow::Borrowed)
                                    .unwrap_or_else(|| format!("_{idx}").into())
                                    .as_ref()
                            )
                        }
                    )?;
                }
                write!(f, "}}")
            }
            OT::Ptr(ty, mutable) => {
                return write!(
                    f,
                    "{}",
                    FmtType {
                        ty,
                        name: Some(&format!(
                            "*{}{}",
                            match mutable {
                                true => "",
                                false => "const",
                            },
                            name.unwrap_or_default()
                        ))
                    }
                );
            }
            OT::FnPtr {
                params,
                return_type,
            } => {
                return write!(
                    f,
                    "{}",
                    FmtType {
                        ty: return_type
                            .as_deref()
                            .unwrap_or(&orco::Type::Symbol("void".into())),
                        name: Some(&format!(
                            "{}({})",
                            name.unwrap_or_default(),
                            params
                                .iter()
                                .map(|ty| FmtType { ty, name }.to_string())
                                .collect::<Vec<_>>()
                                .join(", ")
                        )),
                    }
                );
            }
            OT::Error => write!(f, "<error-type>"),
        }?;
        if let Some(name) = name {
            write!(f, " {name}")?;
        }
        Ok(())
    }
}
