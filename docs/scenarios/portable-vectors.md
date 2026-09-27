# Checked portable f32 vectors

`toolu_orm::core::vector::Vector<N>` validates a positive dimension, exact length and finite f32 elements before creating a parameter. Its fields are private and its slice is read-only. `VectorValue::new(elements, dimension)` provides the same invariants for a runtime declaration. No unchecked deserialization is provided.

```rust
use toolu_orm::core::{error::DbCoreError, value::Value, vector::Vector};

fn embedding_parameter() -> Result<Value, DbCoreError> {
    let embedding = Vector::<3>::new(&[1.25, -2.5, 0.0])?;
    Ok(embedding.into())
}
// Pass the result to execute_sql or a builder's values; retain the vector tag.
```

The same parameter binds through each backend's existing conversion path. Declare matching native columns explicitly for this codec slice:

| Backend | Column | Bound representation | Result decoding |
|---|---|---|---|
| PostgreSQL + pgvector | `vector(3)` | Native vector parameter using pgvector text input | `row.try_get::<_, Vector<3>>(...)` validates binary vector results |
| rusqlite + sqlite-vec | `vec0(embedding float[3])` | Little-endian f32 BLOB | `row.get::<_, Vector<3>>(...)` validates bytes |
| DuckDB + Lance | `embedding FLOAT[3]` | Bound bracketed decimal VARCHAR converted by column assignment to FLOAT[3] | Manual `FromRow::from_lance_row` uses `row.get_typed::<Vector<3>>(...)` |

The pinned DuckDB Rust binding refuses native Array/List parameters. The bound text is a conversion workaround; stored Lance values remain native FLOAT[N], never SQLite BLOBs. When SQL cannot infer a destination array type, explicitly cast the placeholder, for example `CAST(?1 AS FLOAT[3])`. Values are bound, never interpolated into SQL.

NaN, either infinity, zero dimensions and mismatched lengths fail before a vector can become a `Value`. Decoding checks dimensions and finiteness again. SQLite rejects incomplete bytes; Lance only accepts finite non-NULL FLOAT elements in a fixed array, refusing lists, integer/double arrays and NULL elements. `Option<Vector<N>>` accepts a NULL cell. Required NULLs and typed Lance row failures retain column context. Database-specific dimension limits and a declaration inconsistent with database DDL are still database errors.

A zero vector is valid for L2. L2 is the common metric in epic #145; this codec does not introduce a portable top-k API, equalize distance outputs, normalize vectors, or promise identical filtered/ANN rankings. Cosine and inner product remain backend-specific and outside the epic's portable contract. Backend-specific search functions and index limits still apply.

`Value::vector` and `Value::vector_with_dim` retain their legacy SQLite BLOB behavior. They do not become portable vector values and do not gain finiteness checks. Use `Vector<N>` for checked portable input. libsql retains SQLite parameter encoding; the real three-backend SQLite acceptance leg is rusqlite with sqlite-vec. Schema generation and derive expansion for the new type are outside this slice.

## Verification

Tests create real native columns, write [1.25, -2.5, 0.0], verify their native storage type and read the same elements. Each fixture attempts invalid inputs and proves the seeded count remains one. Read tests reject malformed data, mismatched dimensions and unsupported element types. Core checks additionally cover a zero vector, f32::MAX, the smallest positive normal and the smallest positive subnormal.

Run the PostgreSQL fixture with `TEST_DB_PORT=5434 cargo nextest run -p toolu-orm-connection --features postgres --test portable_vector_test`, SQLite with `cargo nextest run -p toolu-orm-connection --features rusqlite,sqlite-vec --test portable_vector_test`, and the pinned real Lance lane with `bash scripts/check-lancedb-smoke.sh`.

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | vector_value_test | checked_values_keep_dimensions_and_finite_elements |
| default | vector_value_test | invalid_inputs_fail_before_value_construction |
| default | vector_value_test | sqlite_decode_rejects_bad_lengths_and_nonfinite_bytes |
| postgres | vector_value_test | postgres_decode_checks_binary_header_and_elements |
| postgres | portable_vector_test | postgres::pgvector_round_trip_and_prewrite_validation |
| rusqlite-only | portable_vector_test | sqlite::sqlite_vec_round_trip_and_prewrite_validation |
| lancedb-smoke | portable_vector_test | lance::lance_array_round_trip_and_prewrite_validation |
