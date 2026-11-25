#!/usr/bin/env python3
"""Compile Scarlet Protocol Definition Language (SPDL) to Rust and C#."""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SOURCE = ROOT / "shared" / "protocol" / "scarlet.spdl"
RUST_OUT = ROOT / "server" / "src" / "protocol.rs"
CSHARP_DIR = ROOT / "client" / "Protocol"

PRIMITIVES = {
    "f32",
    "f64",
    "u8",
    "u16",
    "u32",
    "u64",
    "i32",
    "i64",
    "bool",
    "string",
    "uuid",
}

RUST_PRIM = {
    "f32": "f32",
    "f64": "f64",
    "u8": "u8",
    "u16": "u16",
    "u32": "u32",
    "u64": "u64",
    "i32": "i32",
    "i64": "i64",
    "bool": "bool",
    "string": "String",
    "uuid": "uuid::Uuid",
}

CS_PRIM = {
    "f32": "float",
    "f64": "double",
    "u8": "byte",
    "u16": "ushort",
    "u32": "uint",
    "u64": "ulong",
    "i32": "int",
    "i64": "long",
    "bool": "bool",
    "string": "string",
    "uuid": "Guid",
}


@dataclass
class Field:
    name: str
    base: str
    array: bool = False
    optional: bool = False


@dataclass
class Struct:
    name: str
    fields: list[Field]


@dataclass
class Enum:
    name: str
    variants: list[str]


@dataclass
class UnionVariant:
    name: str
    fields: list[Field]


@dataclass
class Union:
    name: str
    variants: list[UnionVariant]


@dataclass
class Protocol:
    name: str
    version: int
    tag: str
    structs: list[Struct] = field(default_factory=list)
    enums: list[Enum] = field(default_factory=list)
    unions: list[Union] = field(default_factory=list)


class ParseError(Exception):
    pass


class Parser:
    def __init__(self, text: str, path: Path) -> None:
        self.path = path
        stripped = []
        for line in text.splitlines():
            if "#" in line:
                line = line[: line.index("#")]
            stripped.append(line)
        self.src = "\n".join(stripped)
        self.i = 0

    def peek(self) -> str:
        self._ws()
        return self.src[self.i :][:1]

    def _ws(self) -> None:
        while self.i < len(self.src) and self.src[self.i].isspace():
            self.i += 1

    def eof(self) -> bool:
        self._ws()
        return self.i >= len(self.src)

    def ident(self) -> str:
        self._ws()
        m = re.match(r"[A-Za-z_][A-Za-z0-9_.]*", self.src[self.i :])
        if not m:
            raise ParseError(f"{self.path}:{self._loc()} expected identifier")
        self.i += m.end()
        return m.group(0)

    def string(self) -> str:
        self._ws()
        if self.i >= len(self.src) or self.src[self.i] != '"':
            raise ParseError(f"{self.path}:{self._loc()} expected string")
        self.i += 1
        start = self.i
        while self.i < len(self.src) and self.src[self.i] != '"':
            self.i += 1
        val = self.src[start : self.i]
        self.i += 1
        return val

    def number(self) -> int:
        self._ws()
        m = re.match(r"\d+", self.src[self.i :])
        if not m:
            raise ParseError(f"{self.path}:{self._loc()} expected integer")
        self.i += m.end()
        return int(m.group(0))

    def eat(self, tok: str) -> None:
        self._ws()
        if not self.src.startswith(tok, self.i):
            raise ParseError(f"{self.path}:{self._loc()} expected {tok!r}")
        self.i += len(tok)

    def _loc(self) -> str:
        line = self.src.count("\n", 0, self.i) + 1
        return f"{line}"

    def parse(self) -> Protocol:
        self.eat("protocol")
        name = self.ident()
        self.eat("version")
        version = self.number()
        tag = "type"
        self._ws()
        if self.src.startswith("json", self.i):
            self.eat("json")
            self.eat("tagged")
            tag = self.string()
        proto = Protocol(name=name, version=version, tag=tag)
        while not self.eof():
            kind = self.ident()
            if kind == "struct":
                proto.structs.append(self._struct())
            elif kind == "enum":
                proto.enums.append(self._enum())
            elif kind == "union":
                proto.unions.append(self._union())
            else:
                raise ParseError(f"{self.path}:{self._loc()} unknown declaration {kind}")
        return proto

    def _fields_block(self) -> list[Field]:
        self.eat("{")
        fields: list[Field] = []
        while self.peek() != "}":
            fname = self.ident()
            self.eat(":")
            fields.append(self._type(fname))
        self.eat("}")
        return fields

    def _type(self, name: str) -> Field:
        base = self.ident()
        array = False
        optional = False
        self._ws()
        if self.src.startswith("[]", self.i):
            array = True
            self.i += 2
        self._ws()
        if self.src.startswith("?", self.i):
            optional = True
            self.i += 1
        if base not in PRIMITIVES and not re.match(r"[A-Z][A-Za-z0-9]*", base):
            raise ParseError(f"{self.path}:{self._loc()} invalid type {base}")
        return Field(name=name, base=base, array=array, optional=optional)

    def _struct(self) -> Struct:
        name = self.ident()
        return Struct(name=name, fields=self._fields_block())

    def _enum(self) -> Enum:
        name = self.ident()
        self.eat("{")
        variants = []
        while self.peek() != "}":
            variants.append(self.ident())
        self.eat("}")
        return Enum(name=name, variants=variants)

    def _union(self) -> Union:
        name = self.ident()
        self.eat("{")
        variants = []
        while self.peek() != "}":
            vname = self.ident()
            variants.append(UnionVariant(name=vname, fields=self._fields_block()))
        self.eat("}")
        return Union(name=name, variants=variants)


def rust_type(f: Field) -> str:
    t = RUST_PRIM.get(f.base, f.base)
    if f.array:
        t = f"Vec<{t}>"
    if f.optional:
        t = f"Option<{t}>"
    return t


def cs_type(f: Field) -> str:
    t = CS_PRIM.get(f.base, f.base)
    if f.array:
        t = f"List<{t}>"
    if f.optional:
        t = f"{t}?"
    return t


def pascal(name: str) -> str:
    return "".join(p.title() if p else "" for p in name.split("_"))


def rust_header(proto: Protocol) -> str:
    return f"""// AUTO-GENERATED from shared/protocol/scarlet.spdl — do not edit.
// Protocol {proto.name} v{proto.version}

#![allow(dead_code)]

use serde::{{Deserialize, Serialize}};

"""


def emit_rust(proto: Protocol) -> str:
    lines = [rust_header(proto)]
    for en in proto.enums:
        lines.append("#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]")
        lines.append("pub enum %s {" % en.name)
        for v in en.variants:
            lines.append(f"    {v},")
        lines.append("}")
        lines.append("")
    for st in proto.structs:
        lines.append("#[derive(Debug, Clone, Serialize, Deserialize)]")
        lines.append(f"pub struct {st.name} {{")
        for f in st.fields:
            lines.append(f"    pub {f.name}: {rust_type(f)},")
        lines.append("}")
        lines.append("")
    for un in proto.unions:
        lines.append("#[derive(Debug, Clone, Serialize, Deserialize)]")
        lines.append(f'#[serde(tag = "{proto.tag}")]')
        lines.append(f"pub enum {un.name} {{")
        for v in un.variants:
            if v.fields:
                lines.append(f"    {v.name} {{")
                for f in v.fields:
                    lines.append(f"        {f.name}: {rust_type(f)},")
                lines.append("    },")
            else:
                lines.append(f"    {v.name},")
        lines.append("}")
        lines.append("")
    return "\n".join(lines)


def emit_csharp_file(kind: str, name: str, body: str, proto: Protocol, usings: list[str]) -> str:
    extra = "\n".join(f"using {u};" for u in usings)
    return f"""// AUTO-GENERATED from shared/protocol/scarlet.spdl — do not edit.
// Protocol {proto.name} v{proto.version}

using System;
using System.Collections.Generic;
using System.Text.Json.Serialization;
{extra}

namespace ScarletCity.Protocol;

{body}
"""


def emit_csharp(proto: Protocol) -> dict[str, str]:
    files: dict[str, str] = {}
    enum_names = {e.name for e in proto.enums}
    for en in proto.enums:
        body = f"[JsonConverter(typeof(JsonStringEnumConverter))]\npublic enum {en.name}\n{{\n"
        for v in en.variants:
            body += f"    {v},\n"
        body += "}\n"
        files[f"{en.name}.cs"] = emit_csharp_file("enum", en.name, body, proto, [])

    for st in proto.structs:
        body = f"public sealed class {st.name}\n{{\n"
        for f in st.fields:
            body += f'    [JsonPropertyName("{f.name}")]\n'
            default = ""
            if f.array:
                default = f" = new {cs_type(f)}();"
            elif f.base == "string" and not f.optional:
                default = ' = string.Empty;'
            elif f.base not in PRIMITIVES and f.base not in enum_names and not f.optional:
                default = " = null!;"
            body += f"    public {cs_type(f)} {pascal(f.name)} {{ get; set; }}{default}\n\n"
        body += "}\n"
        files[f"{st.name}.cs"] = emit_csharp_file("class", st.name, body, proto, [])

    for un in proto.unions:
        attrs = [f'[JsonPolymorphic(TypeDiscriminatorPropertyName = "{proto.tag}")]']
        for v in un.variants:
            attrs.append(f'[JsonDerivedType(typeof({un.name}.{v.name}), "{v.name}")]')
        body = "\n".join(attrs) + "\n"
        body += f"public abstract class {un.name}\n{{\n"
        body += f"    public sealed class NestedTypesMarker {{ }}\n"
        body += "}\n\n"
        # C# nested types: better as sibling classes with names Join, Input
        # JsonDerivedType typeof needs public types. Use nested classes:
        body = "\n".join(attrs) + "\n"
        body += f"public abstract class {un.name}\n{{\n"
        for v in un.variants:
            body += f"    public sealed class {v.name} : {un.name}\n    {{\n"
            for f in v.fields:
                body += f'        [JsonPropertyName("{f.name}")]\n'
                default = ""
                if f.array:
                    default = f" = new {cs_type(f)}();"
                elif f.base == "string" and not f.optional:
                    default = " = string.Empty;"
                elif f.base not in PRIMITIVES and f.base not in enum_names and not f.optional:
                    default = " = null!;"
                body += f"        public {cs_type(f)} {pascal(f.name)} {{ get; set; }}{default}\n\n"
            body += "    }\n\n"
        body += "}\n"
        files[f"{un.name}.cs"] = emit_csharp_file("union", un.name, body, proto, [])

    return files


def main() -> int:
    ap = argparse.ArgumentParser(description="Generate Rust/C# from SPDL")
    ap.add_argument("source", nargs="?", default=str(DEFAULT_SOURCE))
    args = ap.parse_args()
    path = Path(args.source)
    proto = Parser(path.read_text(), path).parse()
    RUST_OUT.parent.mkdir(parents=True, exist_ok=True)
    RUST_OUT.write_text(emit_rust(proto))
    CSHARP_DIR.mkdir(parents=True, exist_ok=True)
    files = emit_csharp(proto)
    for name, content in files.items():
        (CSHARP_DIR / name).write_text(content)
    print(f"wrote {RUST_OUT.relative_to(ROOT)}")
    for name in files:
        print(f"wrote {(CSHARP_DIR / name).relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
