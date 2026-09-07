#[test]
fn marker_structs_are_constructible() {
  let _serial = toolu_orm_core::column::Serial;
  let _big_serial = toolu_orm_core::column::BigSerial;
  let _jsonb = toolu_orm_core::column::Jsonb;
  let _numeric = toolu_orm_core::column::Numeric;
  let _char: toolu_orm_core::column::Char<10> = toolu_orm_core::column::Char::<10>;
}
