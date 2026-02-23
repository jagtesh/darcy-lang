pub fn resolve_type_alias(name: &str) -> &str {
    match name {
        "int" => "i64",
        "uint" => "u64",
        "float" => "f64",
        "double" => "f64",
        "long" => "i64",
        "ulong" => "u64",
        "short" => "i16",
        "ushort" => "u16",
        "byte" => "u8",
        "sbyte" => "i8",
        "str" => "string",
        _ => name,
    }
}
