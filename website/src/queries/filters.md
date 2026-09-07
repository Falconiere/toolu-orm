# Filters and expressions

`.filter()` takes an `Expr`. Expressions are built from the generated
`Column<T>` constants through three traits in
`toolu_orm_core::query_column`:

```rust
use toolu_orm_core::query_column::{CommonOps, NumericOps, TextOps};
```

| Trait | Methods | Available on |
|---|---|---|
| `CommonOps` | `eq`, `ne`, `in_list`, `not_in`, `is_null`, `is_not_null` | every column |
| `TextOps` | `like` | text-shaped columns |
| `NumericOps` | `gt`, `lt`, `gte`, `lte`, `between` | numeric columns |

```rust
use toolu_orm_core::value::Value;

UsersTable::select()
  .filter(users::org_id.eq("org123"))
  .filter(users::email.like("%@example.com"))
  .filter(users::attempts.between(1i32, 5i32))
  .filter(users::deleted_at.is_null())
  .filter(users::status.in_list(&[Value::Text("active".into()), Value::Text("trial".into())]));
```

Repeated `.filter()` calls are combined with `AND`. `in_list` / `not_in` take
`&[Value]` because the list is data, not a typed column expression.

## Combining

`Expr::and` and `Expr::or` nest predicates explicitly, and the rendered SQL is
parenthesised so precedence survives:

```rust
let expr = users::org_id
  .eq("org123")
  .and(users::status.eq("active").or(users::status.eq("trial")));

UsersTable::select().filter(expr);
// WHERE "users"."org_id" = ?1 AND ("users"."status" = ?2 OR "users"."status" = ?3)
```

## Empty lists

An empty `in_list` renders `1 = 0` and an empty `not_in` renders `1 = 1` — the
same choice drizzle makes. `IN ()` is a syntax error on Postgres, and silently
dropping the predicate would return rows the caller did not ask for.

## Raw fragments

`Expr::raw` carries its own parameters and is renumbered with the rest of the
query, so it composes with the typed predicates:

```rust
use toolu_orm_core::expr::Expr;

UsersTable::select()
  .filter(users::org_id.eq("org123"))
  .filter(Expr::raw("length(\"users\".\"email\") > ?", vec![Value::Integer(5)]));
```

JSON access has a small helper on SQLite-flavoured storage:

```rust
Expr::json_extract(&users::settings, "$.theme").eq("dark");
Expr::json_extract(&users::settings, "$.locale").like("pt%");
```

## Parameter numbering

An `Expr` renders through `to_sql_fragment_for(start, dialect)`, which returns
the fragment plus its parameters and starts numbering at `start`. The builders
pass the current parameter count, which is how several filters, a `limit` and an
`offset` end up with one continuous `?1 … ?N` (or `$1 … $N`) sequence:

```rust
let (fragment, params) = users::email.eq("a@b.c").to_sql_fragment_for(3, Dialect::Postgres);
// ("\"users\".\"email\" = $3", [Text("a@b.c")])
```

## Ordering and joins

`Column::asc()` / `Column::desc()` build an `OrderBy`; `Column::equals(&other)`
builds the `JoinCondition` that `join` / `left_join` take:

```rust
UsersTable::select()
  .left_join("pipelines", users::id.equals(&pipelines::user_id))
  .order_by(users::created_at.desc());
```
