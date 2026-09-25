//! Marker types used by typed columns and schema macros.

/// SQL text column marker.
pub struct Text;
/// SQL integer column marker.
pub struct Integer;
/// SQL real column marker.
pub struct Real;
/// SQL blob column marker.
pub struct Blob;
/// SQL UUID column marker.
pub struct Uuid;
/// SQL boolean column marker.
pub struct Boolean;
/// SQL timestamp column marker.
pub struct Timestamp;
/// SQL date column marker.
pub struct Date;
/// SQL time column marker.
pub struct Time;
/// SQL JSON column marker.
pub struct Json;
/// SQL big integer column marker.
pub struct BigInt;
/// SQL small integer column marker.
pub struct SmallInt;
/// SQL bounded varchar column marker.
pub struct Varchar<const N: u32>;
/// SQL serial column marker.
pub struct Serial;
/// SQL big serial column marker.
pub struct BigSerial;
/// SQL JSONB column marker.
pub struct Jsonb;
/// SQL numeric column marker.
pub struct Numeric;
/// SQL fixed-width character column marker.
pub struct Char<const N: u32>;
/// `#[vec0_table]`'s vector marker; the dimension comes from `#[column(dim = N)]`.
pub struct Vector;
