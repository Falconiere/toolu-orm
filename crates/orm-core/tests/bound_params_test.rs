//! The statement-wide parameter buffer: what it counts, what `nested` re-bases,
//! and that a handle can cross a thread boundary inside an expression.

use toolu_orm_core::expr::{BoundParams, Expr, Scalar, SharedBind, SharedBindList};
use toolu_orm_core::value::Value;

fn text(value: &str) -> Value {
  Value::Text(value.to_owned())
}

/// The types an `Expr` may carry must stay `Send + Sync`, or a builder holding
/// one stops crossing an `await`.
const fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn an_empty_buffer_numbers_the_first_placeholder_one() {
  let params = BoundParams::new();

  assert_eq!(params.len(), 0);
  assert!(params.is_empty());
  assert_eq!(params.next_index(), 1);
}

#[test]
fn push_returns_the_index_it_took_and_advances_the_count() {
  let mut params = BoundParams::new();

  assert_eq!(params.push(text("a")), 1);
  assert_eq!(params.push(Value::Integer(2)), 2);
  assert_eq!(params.next_index(), 3);
  assert!(!params.is_empty());
  assert_eq!(params.into_values(), vec![text("a"), Value::Integer(2)]);
}

#[test]
fn extend_appends_a_rendered_fragments_values_in_order() {
  let mut params = BoundParams::new();
  params.push(text("a"));
  params.extend(vec![text("b"), text("c")]);

  assert_eq!(params.next_index(), 4);
  assert_eq!(params.into_values(), vec![text("a"), text("b"), text("c")]);
}

#[test]
fn a_default_buffer_is_an_empty_one() {
  assert_eq!(BoundParams::default().into_values(), Vec::<Value>::new());
}

#[test]
fn nested_continues_a_live_buffer_from_its_next_index() {
  let mut params = BoundParams::new();
  params.push(text("outer"));

  let start = params.next_index();
  let indices = params.nested(start, |nested| {
    let first = nested.push(text("inner-a"));
    let second = nested.push(text("inner-b"));
    (first, second)
  });

  // `start` is where the frame's values will land, so the indices it numbered
  // are the positions those values actually occupy.
  assert_eq!(indices, (2, 3));
  let values = params.into_values();
  assert_eq!(
    values,
    vec![text("outer"), text("inner-a"), text("inner-b")]
  );
  assert_eq!(values.get(1), Some(&text("inner-a")));
  assert_eq!(values.get(2), Some(&text("inner-b")));
}

#[test]
fn nested_on_an_empty_buffer_is_the_standalone_fragment_frame() {
  // The shape `Expr::to_sql_fragment_for(4, …)` uses: an empty buffer plus the
  // absolute index the caller will splice the fragment at.
  let mut params = BoundParams::new();

  let indices = params.nested(4, |nested| {
    let first = nested.push(text("inner-a"));
    let second = nested.push(text("inner-b"));
    (first, second)
  });

  assert_eq!(indices, (4, 5));
  // Only the frame's own values come back; the stand-ins never escape.
  assert_eq!(params.into_values(), vec![text("inner-a"), text("inner-b")]);
}

#[test]
fn nested_at_one_is_the_identity_frame() {
  let mut params = BoundParams::new();
  let index = params.nested(1, |nested| nested.push(text("only")));

  assert_eq!(index, 1);
  assert_eq!(params.into_values(), vec![text("only")]);
}

#[test]
fn nested_carries_the_binding_ledger_out_of_the_frame() {
  let shared = SharedBind::new("candidate");
  let mut params = BoundParams::new();

  // First use is inside the frame, at the absolute index the frame was given.
  let inner = params.nested(1, |nested| {
    Scalar::shared(&shared).render_into(nested, toolu_orm_core::dialect::Dialect::Sqlite)
  });
  // Second use is outside it, and must land on the same placeholder.
  let outer =
    Scalar::shared(&shared).render_into(&mut params, toolu_orm_core::dialect::Dialect::Sqlite);

  assert_eq!(inner, "?1");
  assert_eq!(outer, "?1");
  assert_eq!(params.into_values(), vec![text("candidate")]);
}

#[test]
fn handles_and_expressions_stay_send_and_sync() {
  assert_send_sync::<SharedBind>();
  assert_send_sync::<SharedBindList>();
  assert_send_sync::<Expr>();
  assert_send_sync::<Scalar>();
}

#[test]
fn a_handle_moves_across_a_thread_and_still_renders_one_placeholder() {
  let shared = SharedBind::new("moved");
  let handle = std::thread::spawn(move || {
    let dialect = toolu_orm_core::dialect::Dialect::Sqlite;
    let mut params = BoundParams::new();
    let first = Scalar::shared(&shared).render_into(&mut params, dialect);
    let second = Scalar::shared(&shared).render_into(&mut params, dialect);
    (first, second, params.into_values())
  });

  let (first, second, values) = handle.join().expect("render thread panicked");
  assert_eq!((first.as_str(), second.as_str()), ("?1", "?1"));
  assert_eq!(values, vec![text("moved")]);
}

/// Handles built concurrently must stay distinct.
///
/// Identity is a `fetch_add` on one atomic counter: `fetch_add` is a
/// read-modify-write, so every caller observes a different value whatever the
/// ordering — `Relaxed` constrains only the visibility of *other* memory, and
/// nothing else is published through the counter. This asserts the property
/// rather than the ordering: 64 handles built on 64 threads, all used in one
/// statement, must bind 64 separate parameters.
#[test]
fn handles_built_on_many_threads_all_stay_independent() {
  let threads: Vec<_> = (0..64)
    .map(|_| std::thread::spawn(|| SharedBind::new("same value everywhere")))
    .collect();
  let handles: Vec<SharedBind> = threads
    .into_iter()
    .map(|thread| thread.join().expect("handle thread panicked"))
    .collect();

  let dialect = toolu_orm_core::dialect::Dialect::Sqlite;
  let mut params = BoundParams::new();
  let rendered: Vec<String> = handles
    .iter()
    .map(|handle| Scalar::shared(handle).render_into(&mut params, dialect))
    .collect();

  assert_eq!(params.len(), 64);
  let expected: Vec<String> = (1..=64).map(|index| format!("?{index}")).collect();
  assert_eq!(rendered, expected);
}

/// The same handle across threads is still one binding — the twin of the test
/// above, so "distinct" cannot be passing by accident of construction.
#[test]
fn one_handle_shared_across_threads_is_still_one_binding() {
  let handle = SharedBind::new("shared");
  let threads: Vec<_> = (0..16).map(|_| handle.clone()).collect();

  let dialect = toolu_orm_core::dialect::Dialect::Sqlite;
  let mut params = BoundParams::new();
  let rendered: Vec<String> = threads
    .iter()
    .map(|clone| Scalar::shared(clone).render_into(&mut params, dialect))
    .collect();

  assert_eq!(params.len(), 1);
  assert!(rendered.iter().all(|sql| sql == "?1"));
}
