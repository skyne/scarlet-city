# Scarlet Protocol Definition Language (SPDL)

SPDL is the markup language for Scarlet City's client/server protocol. One `.spdl` file generates:

- Rust `serde` types for `/server`
- C# `System.Text.Json` types for `/client`

```bash
python3 shared/codegen/generate.py
```

## Language

```
protocol <name> version <int>
json tagged "<discriminator>"

struct Name { field: type }
enum Name { VariantA VariantB }
union Name {
  Variant { field: type }
}
```

Types: `f32` `f64` `u8` `u16` `u32` `u64` `i32` `i64` `bool` `string` `uuid`, arrays `T[]`, optionals `T?`.

Unions encode as internally tagged JSON:

```json
{ "type": "Join", "name": "Rook" }
```
