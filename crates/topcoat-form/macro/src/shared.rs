use syn::Type;

pub fn option_inner(ty: &Type) -> Option<Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    if segment.ident != "Option" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    args.args.iter().find_map(|arg| match arg {
        syn::GenericArgument::Type(inner) => Some(inner.clone()),
        _ => None,
    })
}

pub fn is_string(ty: &Type) -> bool {
    type_ident(ty).map_or(false, |ident| ident == "String")
}

pub fn is_bool(ty: &Type) -> bool {
    type_ident(ty).map_or(false, |ident| ident == "bool")
}

pub fn is_numeric(ty: &Type) -> bool {
    let Some(ident) = type_ident(ty) else {
        return false;
    };
    const NUMERIC: &[&str] = &[
        "f64", "f32", "i128", "i64", "i32", "i16", "i8", "u128", "u64", "u32", "u16", "u8",
        "isize", "usize",
    ];
    NUMERIC.iter().any(|n| ident == *n)
}

pub fn type_ident(ty: &Type) -> Option<String> {
    let Type::Path(type_path) = ty else {
        return None;
    };
    type_path
        .path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
}
