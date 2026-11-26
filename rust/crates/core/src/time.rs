//! Simulation time.
//!
//! One [`Tick`] is one simulated day. Time only advances through the
//! scheduler, never from wall-clock time or render frames.
//!
//! Dates use **astronomical year numbering** in the proleptic Gregorian
//! calendar (docs/SIMULATION.md): 1 BC = year 0, 100 BC = year -99. Year 0 is
//! valid internally but must never be shown; use [`SimDate::era_year`] for
//! display.

use std::fmt;

use crate::codec::{Decode, DecodeError, Decoder, Encode, Encoder};

/// Number of simulated days since the start of a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Tick(pub u64);

impl Tick {
    /// Following tick (saturating).
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

impl fmt::Display for Tick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "T{}", self.0)
    }
}

impl Encode for Tick {
    fn encode(&self, enc: &mut Encoder) {
        enc.put(&self.0);
    }
}

impl Decode for Tick {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(Self(dec.get()?))
    }
}

/// Era for display purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Era {
    /// Before Christ ("v. Chr.").
    Bc,
    /// Anno Domini ("n. Chr.").
    Ad,
}

/// Calendar date (astronomical year, proleptic Gregorian).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SimDate {
    year: i32,
    month: u8,
    day: u8,
}

/// Supported year range; generous but keeps day arithmetic far from overflow.
pub const MIN_YEAR: i32 = -100_000;
/// Supported year range upper bound.
pub const MAX_YEAR: i32 = 100_000;

impl SimDate {
    /// Validated constructor.
    pub fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        if !(MIN_YEAR..=MAX_YEAR).contains(&year) || !(1..=12).contains(&month) {
            return None;
        }
        if day == 0 || day > days_in_month(year, month) {
            return None;
        }
        Some(Self { year, month, day })
    }

    /// 1 January of an astronomical year (the documented default start day).
    pub fn new_year(year: i32) -> Option<Self> {
        Self::new(year, 1, 1)
    }

    /// Astronomical year (may be 0 or negative).
    pub const fn year(self) -> i32 {
        self.year
    }

    /// Month 1..=12.
    pub const fn month(self) -> u8 {
        self.month
    }

    /// Day of month 1..=31.
    pub const fn day(self) -> u8 {
        self.day
    }

    /// Display era and year; never yields year 0.
    pub fn era_year(self) -> (Era, u32) {
        if self.year > 0 {
            (Era::Ad, self.year.unsigned_abs())
        } else {
            (Era::Bc, (1 - i64::from(self.year)) as u32)
        }
    }

    /// Days since 1970-01-01 (negative before).
    pub fn to_day_number(self) -> i64 {
        days_from_civil(i64::from(self.year), self.month.into(), self.day.into())
    }

    /// Inverse of [`to_day_number`](Self::to_day_number).
    pub fn from_day_number(days: i64) -> Option<Self> {
        let (y, m, d) = civil_from_days(days);
        let year = i32::try_from(y).ok()?;
        Self::new(year, m as u8, d as u8)
    }

    /// Date `days` days later.
    pub fn add_days(self, days: u64) -> Option<Self> {
        let days = i64::try_from(days).ok()?;
        Self::from_day_number(self.to_day_number().checked_add(days)?)
    }

    /// ISO weekday, Monday = 1 .. Sunday = 7.
    pub fn iso_weekday(self) -> u8 {
        // 1970-01-01 was a Thursday (4).
        ((self.to_day_number() + 3).rem_euclid(7) + 1) as u8
    }
}

impl fmt::Display for SimDate {
    /// Technical representation `YYYY-MM-DD` with astronomical year.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl Encode for SimDate {
    fn encode(&self, enc: &mut Encoder) {
        enc.put(&self.year);
        enc.put(&self.month);
        enc.put(&self.day);
    }
}

impl Decode for SimDate {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        let (y, m, d) = (dec.get()?, dec.get()?, dec.get()?);
        Self::new(y, m, d).ok_or(DecodeError::Invalid("calendar date"))
    }
}

/// Proleptic Gregorian leap year rule (astronomical numbering).
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Days of `month` in `year`.
pub fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

// Howard Hinnant's civil calendar algorithms (public domain), integer only.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_and_known_dates() {
        assert_eq!(SimDate::new(1970, 1, 1).unwrap().to_day_number(), 0);
        assert_eq!(SimDate::new(2000, 3, 1).unwrap().to_day_number(), 11_017);
        assert_eq!(SimDate::new(1970, 1, 1).unwrap().iso_weekday(), 4);
        assert_eq!(SimDate::new(1, 1, 1).unwrap().iso_weekday(), 1); // proleptic Gregorian Monday
    }

    #[test]
    fn roundtrip_across_year_zero() {
        let start = SimDate::new(-1, 12, 31).unwrap();
        let next = start.add_days(1).unwrap();
        assert_eq!(next, SimDate::new(0, 1, 1).unwrap());
        assert_eq!(next.add_days(366).unwrap(), SimDate::new(1, 1, 1).unwrap()); // year 0 is leap
        for days in (-800_000i64..800_000).step_by(997) {
            assert_eq!(
                SimDate::from_day_number(days).unwrap().to_day_number(),
                days
            );
        }
    }

    #[test]
    fn era_display_never_shows_year_zero() {
        assert_eq!(SimDate::new_year(0).unwrap().era_year(), (Era::Bc, 1));
        assert_eq!(SimDate::new_year(-99).unwrap().era_year(), (Era::Bc, 100));
        assert_eq!(SimDate::new_year(1).unwrap().era_year(), (Era::Ad, 1));
    }

    #[test]
    fn validation() {
        assert!(SimDate::new(1900, 2, 29).is_none());
        assert!(SimDate::new(2000, 2, 29).is_some());
        assert!(SimDate::new(2000, 13, 1).is_none());
        assert!(SimDate::new(MAX_YEAR + 1, 1, 1).is_none());
        assert_eq!(
            SimDate::decode(&mut Decoder::new(&[0, 0, 0, 0, 2, 30])),
            Err(DecodeError::Invalid("calendar date"))
        );
    }
}
