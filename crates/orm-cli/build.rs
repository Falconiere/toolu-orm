fn main() {
  // Register custom cfgs so `cargo clippy` / `rustc` doesn't warn about them.
  println!("cargo::rustc-check-cfg=cfg(orm_core_has_rusqlite)");
  println!("cargo::rustc-check-cfg=cfg(orm_core_has_libsql)");
  println!("cargo::rustc-check-cfg=cfg(orm_core_has_postgres)");

  // Detect orm-core's actual feature state (which Cargo may unify independently
  // of orm-cli's own features) via DEP_TOOLU_ORM_CORE_* metadata emitted by
  // orm-core's build script.
  if std::env::var("DEP_TOOLU_ORM_CORE_HAS_RUSQLITE").is_ok() {
    println!("cargo::rustc-cfg=orm_core_has_rusqlite");
  }
  if std::env::var("DEP_TOOLU_ORM_CORE_HAS_LIBSQL").is_ok() {
    println!("cargo::rustc-cfg=orm_core_has_libsql");
  }
  if std::env::var("DEP_TOOLU_ORM_CORE_HAS_POSTGRES").is_ok() {
    println!("cargo::rustc-cfg=orm_core_has_postgres");
  }
}
