//! RFC3339 timestamps as Postgres `timestamptz` microseconds.
//!
//! Postgres stores `timestamptz` as microseconds since 2000-01-01 UTC. Callers
//! pass Unix epoch seconds or an RFC3339 string; both become that scale here,
//! before the driver sees the parameter.

use crate::error::DbCoreError;

const PG_EPOCH_UNIX_MICROS: i64 = 946_684_800_000_000;

pub(super) fn epoch_secs_to_pg_micros(secs: i64) -> Result<i64, DbCoreError> {
  let unix_us = secs
    .checked_mul(1_000_000)
    .ok_or_else(|| invalid("timestamp overflows unix microseconds"))?;
  to_pg(unix_us)
}

pub(super) fn rfc3339_to_pg_micros(text: &str) -> Result<i64, DbCoreError> {
  let unix_us = parse_rfc3339(text.as_bytes())?;
  to_pg(unix_us)
}

fn to_pg(unix_us: i64) -> Result<i64, DbCoreError> {
  unix_us
    .checked_sub(PG_EPOCH_UNIX_MICROS)
    .ok_or_else(|| invalid("timestamp overflows timestamptz"))
}

fn invalid(reason: &str) -> DbCoreError {
  DbCoreError::InvalidParameter {
    kind: "timestamp",
    reason: reason.to_owned(),
  }
}

fn narrowed<T, E>(result: Result<T, E>, reason: &'static str) -> Result<T, DbCoreError> {
  result.map_err(|_error| invalid(reason))
}

fn parse_rfc3339(input: &[u8]) -> Result<i64, DbCoreError> {
  let mut scan = Scan { rest: input };
  let (year, month, day) = scan.date()?;
  scan.separator()?;
  let (hour, minute, second, micros) = scan.time()?;
  let offset_secs = scan.offset()?;
  if !scan.rest.is_empty() {
    return Err(invalid("trailing data after an RFC3339 timestamp"));
  }
  let days = days_from_civil(year, month, day)?;
  let local = days
    .checked_mul(86_400)
    .and_then(|secs| secs.checked_add(i64::from(hour) * 3600))
    .and_then(|secs| secs.checked_add(i64::from(minute) * 60))
    .and_then(|secs| secs.checked_add(i64::from(second)))
    .ok_or_else(|| invalid("timestamp overflows unix seconds"))?;
  let utc = local
    .checked_sub(offset_secs)
    .ok_or_else(|| invalid("timestamp overflows unix seconds"))?;
  utc
    .checked_mul(1_000_000)
    .and_then(|us| us.checked_add(micros))
    .ok_or_else(|| invalid("timestamp overflows unix microseconds"))
}

struct Scan<'a> {
  rest: &'a [u8],
}

impl Scan<'_> {
  fn date(&mut self) -> Result<(i32, u32, u32), DbCoreError> {
    let year = self.number(4)?;
    self.byte(b'-')?;
    let month = self.number(2)?;
    self.byte(b'-')?;
    let day = self.number(2)?;
    let year = narrowed(i32::try_from(year), "year does not fit")?;
    let month = narrowed(u32::try_from(month), "month does not fit")?;
    let day = narrowed(u32::try_from(day), "day does not fit")?;
    let max_day = days_in_month(year, month)?;
    if day == 0 || day > max_day {
      return Err(invalid("day is out of range"));
    }
    Ok((year, month, day))
  }

  fn separator(&mut self) -> Result<(), DbCoreError> {
    match self.bump() {
      Some(b'T' | b't' | b' ') => Ok(()),
      _ => Err(invalid("expected 'T' between the date and the time")),
    }
  }

  fn time(&mut self) -> Result<(u32, u32, u32, i64), DbCoreError> {
    let hour = self.small(2, 23, "hour")?;
    self.byte(b':')?;
    let minute = self.small(2, 59, "minute")?;
    self.byte(b':')?;
    let second = self.small(2, 59, "second")?;
    let micros = self.fraction()?;
    Ok((hour, minute, second, micros))
  }

  fn fraction(&mut self) -> Result<i64, DbCoreError> {
    if self.rest.first() != Some(&b'.') {
      return Ok(0);
    }
    let _ = self.bump();
    let mut digits = 0_u32;
    let mut value = 0_i64;
    while self.rest.first().is_some_and(u8::is_ascii_digit) {
      let Some(byte) = self.bump() else {
        break;
      };
      digits += 1;
      if digits <= 6 {
        let d = i64::from(byte - b'0');
        value = value
          .checked_mul(10)
          .and_then(|n| n.checked_add(d))
          .ok_or_else(|| invalid("fraction overflows"))?;
      }
    }
    if digits == 0 {
      return Err(invalid("expected digits after the decimal point"));
    }
    while digits < 6 {
      value = value
        .checked_mul(10)
        .ok_or_else(|| invalid("fraction overflows"))?;
      digits += 1;
    }
    Ok(value)
  }

  fn offset(&mut self) -> Result<i64, DbCoreError> {
    match self.bump() {
      Some(b'Z' | b'z') => Ok(0),
      Some(sign @ (b'+' | b'-')) => self.numeric_offset(sign),
      _ => Err(invalid("expected a timezone offset")),
    }
  }

  fn numeric_offset(&mut self, sign: u8) -> Result<i64, DbCoreError> {
    let hour = self.small(2, 23, "offset hour")?;
    if self.rest.first() == Some(&b':') {
      let _ = self.bump();
    }
    let minute = if self.rest.first().is_some_and(u8::is_ascii_digit) {
      self.small(2, 59, "offset minute")?
    } else {
      0
    };
    let secs = i64::from(hour) * 3600 + i64::from(minute) * 60;
    if sign == b'-' {
      Ok(-secs)
    } else {
      Ok(secs)
    }
  }

  fn small(&mut self, width: usize, max: u32, what: &str) -> Result<u32, DbCoreError> {
    let n = self.number(width)?;
    let n = narrowed(u32::try_from(n), "component does not fit")?;
    if n > max {
      return Err(invalid(&format!("{what} is out of range")));
    }
    Ok(n)
  }

  fn number(&mut self, width: usize) -> Result<i64, DbCoreError> {
    let head = self.take(width)?;
    let mut n = 0_i64;
    for byte in head {
      if !byte.is_ascii_digit() {
        return Err(invalid("expected a digit"));
      }
      n = n
        .checked_mul(10)
        .and_then(|v| v.checked_add(i64::from(byte - b'0')))
        .ok_or_else(|| invalid("number overflows"))?;
    }
    Ok(n)
  }

  fn byte(&mut self, expected: u8) -> Result<(), DbCoreError> {
    if self.bump() == Some(expected) {
      Ok(())
    } else {
      Err(invalid("unexpected character in timestamp"))
    }
  }

  fn bump(&mut self) -> Option<u8> {
    let (byte, rest) = self.rest.split_first()?;
    self.rest = rest;
    Some(*byte)
  }

  fn take(&mut self, n: usize) -> Result<&[u8], DbCoreError> {
    if self.rest.len() < n {
      return Err(invalid("timestamp is truncated"));
    }
    let (head, tail) = self.rest.split_at(n);
    self.rest = tail;
    Ok(head)
  }
}

fn days_in_month(year: i32, month: u32) -> Result<u32, DbCoreError> {
  let days = match month {
    1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
    4 | 6 | 9 | 11 => 30,
    2 if is_leap(year) => 29,
    2 => 28,
    _ => return Err(invalid("month is out of range")),
  };
  Ok(days)
}

fn is_leap(year: i32) -> bool {
  let divisible = |n: i32| year.rem_euclid(n) == 0;
  divisible(4) && !divisible(100) || divisible(400)
}

/// Days since 1970-01-01. Howard Hinnant's civil-from-days inverse.
fn days_from_civil(year: i32, month: u32, day: u32) -> Result<i64, DbCoreError> {
  let mut y = i64::from(year);
  let m = i64::from(month);
  let d = i64::from(day);
  if m <= 2 {
    y -= 1;
  }
  let era = if y >= 0 { y } else { y - 399 } / 400;
  let yoe = narrowed(u64::try_from(y - era * 400), "year does not fit")?;
  let month_shift = if m > 2 { m - 3 } else { m + 9 };
  let doy = (153 * month_shift + 2) / 5 + d - 1;
  let doe = yoe * 365 + yoe / 4 - yoe / 100 + narrowed(u64::try_from(doy), "day does not fit")?;
  let days = era * 146_097 + narrowed(i64::try_from(doe), "day does not fit")? - 719_468;
  Ok(days)
}
