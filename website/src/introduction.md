<div class="hero">
  <div class="eyebrow">Rust ORM · libsql · rusqlite · Postgres</div>
  <h1>One struct. Three databases.<br>Migrations you can read.</h1>
  <p>
    toolu-orm keeps a Rust struct as the single source of truth. <code>#[table]</code>
    turns it into schema metadata, typed columns and query builders; the schema is
    diffed against the last snapshot into plain SQL migrations you review like any
    other file. No runtime reflection, no generated SQL you cannot read.
  </p>
  <div class="hero-actions">
    <a class="primary" href="getting-started/quickstart.html">Quickstart</a>
    <a href="getting-started/installation.html">Installation</a>
    <a href="https://github.com/Falconiere/toolu-orm">GitHub</a>
    <a href="https://docs.rs/toolu-orm-core">API docs</a>
  </div>
  <div class="pills">
    <span class="pill">schema-first</span>
    <span class="pill">typed columns</span>
    <span class="pill">diff-driven migrations</span>
    <span class="pill">SHA-256 journal</span>
    <span class="pill">no unwrap in src/</span>
  </div>
</div>

## Why

Most Rust database layers make you pick a side. Query builders give you type-safe
SQL but leave schema evolution to you — the migration folder drifts from the
structs and nobody notices until production. Full ORMs own the schema but hide the
SQL behind a runtime, a DSL, or a generator you re-run and re-learn.

toolu-orm generates everything from the struct and leaves the SQL in the open:

<div class="grid">
  <div class="card">
    <h3>Schema as code</h3>
    <p><code>#[table]</code> produces a <code>TableDef</code> with primary keys, defaults, foreign keys, <code>STRICT</code> tables and indexes.</p>
  </div>
  <div class="card">
    <h3>Diff-driven migrations</h3>
    <p><code>run_generate</code> diffs the registry against the last snapshot and writes numbered SQL. Nothing is applied behind your back.</p>
  </div>
  <div class="card">
    <h3>Tamper-evident journal</h3>
    <p><code>_journal.json</code> stores a SHA-256 per migration. A file edited after it shipped stops the run instead of diverging silently.</p>
  </div>
  <div class="card">
    <h3>Typed columns</h3>
    <p>Generated <code>Column&lt;T&gt;</code> constants build <code>Expr</code> trees. Column references are always table-qualified and quoted.</p>
  </div>
  <div class="card">
    <h3>Dialect-aware SQL</h3>
    <p>SQLite renders <code>?N</code>; Postgres renders <code>$N</code>, <code>ON CONFLICT … DO UPDATE</code> and <code>LEFT JOIN LATERAL</code>.</p>
  </div>
  <div class="card">
    <h3>Relations without N+1</h3>
    <p><code>with_many</code> / <code>with_one</code> fetch parent and children as JSON arrays in a single statement per dialect.</p>
  </div>
</div>

## How it works

```text
structs ──#[table]──▶ TableDef ──SchemaRegistry──▶ diff vs last snapshot
        ──▶ NNNN_name.sql + NNNN_name.snapshot.json + _journal.json
```

Six crates. `toolu-orm-core` is the foundation and every other crate depends on
it; only `toolu-orm-cli` depends on `toolu-orm-connection`.

| Crate | What it holds |
|---|---|
| [`toolu-orm`](https://docs.rs/toolu-orm) | The facade: re-exports the four library crates and the macros behind one version and one feature list. |
| [`toolu-orm-core`](https://docs.rs/toolu-orm-core) | `TableDef`, `ColumnType`, `Value`, `Expr`, `Column<T>`, snapshots, journal, diff, `Dialect`. |
| [`toolu-orm-macros`](https://docs.rs/toolu-orm-macros) | `#[table]`, `#[derive(FromRow)]`, `#[derive(Relational)]`, `#[derive(ColumnEnum)]`, `#[view]`. |
| [`toolu-orm-query`](https://docs.rs/toolu-orm-query) | `SelectBuilder`, `InsertBuilder`, `UpdateBuilder`, `DeleteBuilder`, `RelationalQuery`, executor and transactions. |
| [`toolu-orm-connection`](https://docs.rs/toolu-orm-connection) | `DbConnection` over libsql, rusqlite and Postgres. |
| [`toolu-orm-cli`](https://docs.rs/toolu-orm-cli) | `run_generate`, `run_migrate`, `get_status`. |

## Where to go next

- [Installation](getting-started/installation.md) — the facade, macro paths, driver features.
- [Quickstart](getting-started/quickstart.md) — table, migration, insert and read against in-memory libsql.
- [Defining tables](schema/tables.md) — every `#[table]` and `#[column]` attribute.
- [Select](queries/select.md) — builders, joins, paging and the fetch methods.
- [The migration loop](migrations/overview.md) — generate, migrate, status.

Extracted from a production backend where it drives Turso embedded replicas in
the field and Postgres in the cloud, from one set of table structs.
