#!/usr/bin/env python3
"""
Generate kabipay-db-entities Rust sources from Liquibase tenant migrations (0005-0030).
Run from repo root:  python kabipay-svc/scripts/generate_db_entities.py
"""
from __future__ import annotations

import re
import xml.etree.ElementTree as ET
from pathlib import Path

NS = {"db": "http://www.liquibase.org/xml/ns/dbchangelog"}
MIGRATIONS = Path("kabipay-database/changelog/migrations")
OUT = Path("kabipay-svc/crates/kabipay-db-entities/src/tenant")

COMPOSITE_PK: dict[str, list[str]] = {
    "role_permission": ["role_id", "permission_id"],
    "user_role": ["user_id", "role_id"],
}


def sql_type_to_rust(sql_type: str, nullable: bool) -> str:
    st = sql_type.strip()
    if st == "UUID":
        base = "Uuid"
    elif st == "BOOLEAN":
        base = "bool"
    elif st == "INT":
        base = "i32"
    elif st == "BIGINT":
        base = "i64"
    elif re.match(r"^NUMERIC\(\d+,\d+\)$", st):
        base = "Decimal"
    elif st == "TIMESTAMPTZ":
        base = "DateTimeUtc"
    elif st == "DATE":
        base = "NaiveDate"
    elif st == "TIME":
        base = "NaiveTime"
    elif st == "JSONB":
        base = "Json"
    elif st.startswith("VARCHAR"):
        base = "String"
    elif st == "TEXT":
        base = "String"
    else:
        raise ValueError(f"Unknown SQL type: {sql_type}")
    if nullable:
        return f"Option<{base}>"
    return base


def col_nullable(col: ET.Element) -> bool:
    cons = col.find("db:constraints", NS)
    if cons is None:
        return True
    return cons.get("nullable", "true") != "false"


def parse_columns(table_el: ET.Element) -> list[tuple[str, str, bool]]:
    cols = []
    for col in table_el.findall("db:column", NS):
        name = col.attrib["name"]
        ctype = col.attrib["type"]
        nullable = col_nullable(col)
        cols.append((name, ctype, nullable))
    return cols


def sanitize_mod_name(table: str) -> str:
    if table in ("mod", "type", "use", "self", "crate"):
        return f"{table}_"
    return table.replace("-", "_")


def rust_field_name(col: str) -> str:
    """Escape Rust keywords used as SQL column names."""
    if col == "type":
        return "r#type"
    return col


def emit_entity(table: str, cols: list[tuple[str, str, bool]]) -> str:
    mod = sanitize_mod_name(table)
    composite = COMPOSITE_PK.get(table)
    lines = [
        f"pub mod {mod} {{",
        "    use crate::tenant::prelude::*;",
        "",
        "    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]",
        f'    #[sea_orm(table_name = "{table}")]',
        "    pub struct Model {",
    ]
    for name, ctype, nullable in cols:
        if composite and name in composite:
            lines.append(
                "        #[sea_orm(primary_key, auto_increment = false)]"
            )
            lines.append(f"        pub {rust_field_name(name)}: Uuid,")
            continue
        if composite:
            rs_t = sql_type_to_rust(ctype, nullable)
            lines.append(f"        pub {rust_field_name(name)}: {rs_t},")
            continue
        if name == "id" and ctype == "UUID":
            lines.append(
                "        #[sea_orm(primary_key, auto_increment = false)]"
            )
            lines.append("        pub id: Uuid,")
            continue
        rs_t = sql_type_to_rust(ctype, nullable)
        lines.append(f"        pub {rust_field_name(name)}: {rs_t},")
    lines.append("    }")
    lines.append("")
    lines.append("    impl ActiveModelBehavior for ActiveModel {}")
    lines.append("")
    lines.append("    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]")
    lines.append("    pub enum Relation {}")
    lines.append("}")
    lines.append("")
    return "\n".join(lines)


def process_file(path: Path) -> list[str]:
    tree = ET.parse(path)
    root = tree.getroot()
    chunks: list[str] = []
    for ct in root.findall(".//db:createTable", NS):
        schema = ct.attrib.get("schemaName", "")
        if schema != "${schema}":
            continue
        table = ct.attrib["tableName"]
        cols = parse_columns(ct)
        if not cols:
            continue
        chunks.append(emit_entity(table, cols))
    return chunks


def domain_rust_mod(folder_name: str) -> str:
    """0005_auth_rbac -> d0005_auth_rbac (valid Rust identifier)."""
    return f"d{folder_name}"


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    dirs = sorted(
        [p for p in MIGRATIONS.iterdir() if p.is_dir() and re.match(r"^\d{4}_", p.name)],
        key=lambda p: p.name,
    )
    mod_lines: list[str] = []

    for d in dirs:
        if "integration_connector" in d.name and "0005_integration" in d.name:
            continue
        chunks: list[str] = []
        sources: list[str] = []
        for xml in sorted(d.glob("*.xml")):
            part = process_file(xml)
            if part:
                chunks.extend(part)
                sources.append(xml.as_posix())
        if not chunks:
            continue
        fname = f"{domain_rust_mod(d.name)}.rs"
        src_comment = ", ".join(sources)
        body = "\n".join(chunks)
        (OUT / fname).write_text(
            f"//! Auto-generated from `{src_comment}`.\n\n{body}",
            encoding="utf-8",
        )
        mod_lines.append(f"pub mod {domain_rust_mod(d.name)};")

    prelude = """//! Shared imports for generated tenant entities.
pub use sea_orm::entity::prelude::*;
pub use sea_orm::prelude::Json;
pub use sea_orm::{
    ActiveModelBehavior, DeriveEntityModel, DeriveRelation, EnumIter, RelationTrait,
};
pub use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
pub use rust_decimal::Decimal;
pub use uuid::Uuid;

pub type DateTimeUtc = DateTime<Utc>;
"""
    (OUT / "prelude.rs").write_text(prelude, encoding="utf-8")

    # mod.rs
    unique_mods = sorted(set(mod_lines))
    mod_rs = (
        "//! Tenant-schema table models (Liquibase domains 0005–0030).\n"
        "//! Generated — do not hand-edit; re-run `scripts/generate_db_entities.py`.\n\n"
        "pub mod prelude;\n"
        "pub use prelude::*;\n\n"
        + "\n".join(unique_mods)
        + "\n"
    )
    (OUT / "mod.rs").write_text(mod_rs, encoding="utf-8")

    print(f"Wrote {len(unique_mods)} modules under {OUT}")


if __name__ == "__main__":
    main()
