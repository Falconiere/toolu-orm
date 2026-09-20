# Filters and expressions

`.filter()` takes an `Expr`. Expressions are built from the generated
`Column<T>` constants through traits in
`toolu_orm_core::query_column`:

```rust
use toolu_orm_core::query_column::{CommonOps, NumericOps, TextOps};
```

| Trait | Methods | Available on |
|---|---|---|
| `CommonOps` | `eq`, `ne`, `in_list`, `not_in`, `is_null`, `is_not_null` | every column |
| `TextOps` | `like`, `like_escape` | `Text`, `Uuid`, `Date`, `Time`, `Varchar<N>` |
| `NumericOps` | `gt`, `lt`, `gte`, `lte`, `between` | `Integer`, `Real`, `BigInt`, `SmallInt`, `Timestamp`, `Date`, `Time` |
| `SharedOps` | `eq_shared`, `ne_shared`, `in_shared`, `not_in_shared` | every column, including aliased columns |

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
// WHERE ("users"."org_id" = ?1 AND ("users"."status" = ?2 OR "users"."status" = ?3))
```

## Empty lists

An empty `in_list` renders `1 = 0` and an empty `not_in` renders `1 = 1` on both
dialects. `IN ()` is a syntax error on Postgres, and silently
dropping the predicate would return rows the caller did not ask for. Nonempty
lists retain SQL's NULL semantics; for example, a `NOT IN` list containing
`NULL` does not match rows.

## Scalar expressions

`Scalar` builds computed values for projections, comparisons, ordering and
assignments. `Scalar::col(&column)` references a column, `Scalar::bind(value)`
binds data, and arithmetic operators compose them:

```rust
use toolu_orm_core::expr::Scalar;

UsersTable::select()
  .columns_raw(&["id"])
  .column_scalar(Scalar::col(&users::attempts) + Scalar::bind(1_i64), "next_attempt")
  .filter(Scalar::col(&users::attempts).gte(Scalar::bind(2_i64)));
```

`Scalar::func(name, args)` validates the function name and returns a `Result`;
the database determines whether the function exists. `Scalar::case_when` builds
conditional values. `Scalar::sql(text)` carries trusted SQL with no parameters,
and `Scalar::raw(text, params)` carries a fragment with its bound values.

For a literal text search, escape SQL's `%` and `_` wildcards explicitly:

```rust
use toolu_orm_core::expr::like_pattern_literal;

let pattern = format!("%{}%", like_pattern_literal("100%", '\\'));
let predicate = users::email.like_escape(pattern, '\\');
```

See the [scalar-expression scenarios](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/scalar-expressions.md)
for functions, CASE expressions, NULLs and timestamps.

## Reusing bound values

Use one `SharedBind` handle when several clauses should share a placeholder.
Clones preserve its identity; two separately created handles bind separately,
even when they hold equal values.

```rust
use toolu_orm_core::expr::SharedBind;
use toolu_orm_core::query_column::SharedOps;

let status = SharedBind::new("active");
let predicate = users::status.eq_shared(&status)
  .or(users::previous_status.eq_shared(&status));
// Both comparisons use ?1 (or $1) when this is the first binding.
```

`SharedBindList` provides the same behavior for `in_shared` / `not_in_shared`.
`Scalar::shared(&handle)` uses a shared value in a computed expression. A handle
can span nested queries and write clauses within one statement; each standalone
render starts a fresh binding scope. See
[reusable bindings](https://github.com/Falconiere/toolu-orm/blob/main/docs/scenarios/reusable-bound-parameters.md).

## Raw fragments

`Expr::raw` carries its own parameters and is renumbered with the rest of the
query, so it composes with the typed predicates:

```rust
use toolu_orm_core::expr::Expr;

UsersTable::select()
  .filter(users::org_id.eq("org123"))
  .filter(Expr::raw("length(\"users\".\"email\") > ?", vec![Value::Integer(5)]));
```

A bare `?` takes the next local index; `?N` addresses this fragment's Nth value,
starting at 1. Both are renumbered for the fragment's position and target dialect.
For example, `Expr::raw("a = ?1 OR b = ?1", vec![value])` binds one value and
references it twice. Use `SharedBind` to share across separate fragments.
The renderer does not parse SQL quoting or comments: keep literal question marks
in bound data, and supply exactly the values the placeholders reference.

JSON access has a small helper on SQLite-flavoured storage. Its path is emitted
as SQL text, so use a fixed path:

```rust
Expr::json_extract(&users::settings, "$.theme").eq("dark");
Expr::json_extract(&users::settings, "$.locale").like("pt%");
```

## Parameter numbering

An `Expr` renders through `to_sql_fragment_for(start, dialect)`, which returns
the fragment plus its parameters and starts numbering at the 1-based `start`.
Builders render into a shared `BoundParams` buffer in SQL clause order, reusing
indices for shared handles. Projections, joins, filters and pagination therefore
bind in one `?1 … ?N` (or `$1 … $N`) sequence:

```rust
use toolu_orm_core::dialect::Dialect;

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

A `JoinCondition` is an expression tree, not a single equality. Alongside
`equals` there are `not_equals`, `less_than`, `less_or_equal`, `greater_than`
and `greater_or_equal` — all column-to-column, binding nothing — and `and` / `or`
combine them with ordinary value predicates, which are plain `Expr`s:

```rust
UsersTable::select()
  .left_join(
    "pipelines",
    users::id.equals(&pipelines::user_id).and(pipelines::active.eq(1)),
  );
// ... LEFT JOIN "pipelines" ON ("users"."id" = "pipelines"."user_id" AND "pipelines"."active" = ?1)
```

`Expr` and `JoinCondition` convert into each other. A column-to-column comparison
needs `.into()` when passed to `filter`, whose argument is an `Expr`:
`.filter(users::id.equals(&pipelines::user_id).into())`. A value predicate works
directly in an `ON` clause. Render a condition with
`to_sql_fragment_for(start, dialect)`, which
returns the fragment together with the values it binds. Table aliases are in
[Select](select.md).
