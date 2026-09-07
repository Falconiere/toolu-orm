//! trybuild compile-fail cases for the macro error paths.

#[test]
fn compile_fail() {
  let t = trybuild::TestCases::new();
  t.compile_fail("tests/ui/*.rs");
}
