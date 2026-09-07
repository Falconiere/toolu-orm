use toolu_orm_query::tuple_append::TupleAppend;

#[test]
fn single_element_appends_to_pair() {
  let base: (String,) = ("hello".to_owned(),);
  let result: (String, i32) = base.append(42);
  assert_eq!(result.0, "hello");
  assert_eq!(result.1, 42);
}

#[test]
fn pair_appends_to_triple() {
  let base = ("a".to_owned(), 1_i32);
  let result: (String, i32, bool) = base.append(true);
  assert_eq!(result.0, "a");
  assert_eq!(result.1, 1);
  assert!(result.2);
}

#[test]
fn chained_appends_build_up_tuple() {
  let t1: (i32,) = (1,);
  let t2: (i32, i32) = t1.append(2);
  let t3: (i32, i32, i32) = t2.append(3);
  let t4: (i32, i32, i32, i32) = t3.append(4);
  assert_eq!(t4, (1, 2, 3, 4));
}

#[test]
fn append_up_to_eight_elements() {
  let t = (1_i32,)
    .append(2_i32)
    .append(3_i32)
    .append(4_i32)
    .append(5_i32)
    .append(6_i32)
    .append(7_i32)
    .append(8_i32);
  assert_eq!(t, (1, 2, 3, 4, 5, 6, 7, 8));
}
