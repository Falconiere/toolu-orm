fn main() {
  // Expose active backend features as DEP_TOOLU_ORM_CORE_* metadata so that
  // downstream crates (e.g. orm-cli) can detect which `FromRow` trait shape
  // orm-core actually compiled, even when Cargo feature unification activates
  // features that the downstream crate did not explicitly request.
  if cfg!(feature = "rusqlite") {
    println!("cargo::metadata=has_rusqlite=1");
  }
  if cfg!(feature = "libsql") {
    println!("cargo::metadata=has_libsql=1");
  }
  if cfg!(feature = "postgres") {
    println!("cargo::metadata=has_postgres=1");
  }
}
