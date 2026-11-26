//! Year selection (YearPicker) – independent of scenario resolution.
//!
//! Years are astronomical internally (1 BC = 0, 100 BC = -99) and are only
//! displayed as era + positive number, so stepping forward from "1 v. Chr."
//! lands on "1 n. Chr." and year 0 is never shown or accepted as input.

use kod_core::time::Era;

/// Earliest selectable start year: 100 BC (astronomical -99).
pub const MIN_START_YEAR: i32 = -99;

/// A year as shown to players.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayYear {
    /// Era.
    pub era: Era,
    /// Positive year number within the era.
    pub number: u32,
}

/// Converts an astronomical year for display; never yields 0.
pub fn to_display(year: i32) -> DisplayYear {
    if year > 0 {
        DisplayYear {
            era: Era::Ad,
            number: year.unsigned_abs(),
        }
    } else {
        DisplayYear {
            era: Era::Bc,
            number: (1 - i64::from(year)) as u32,
        }
    }
}

/// German display label, e.g. "100 v. Chr." or "1492".
pub fn label_de(year: i32) -> String {
    let d = to_display(year);
    match d.era {
        Era::Bc => format!("{} v. Chr.", d.number),
        Era::Ad if year < 1000 => format!("{} n. Chr.", d.number),
        Era::Ad => d.number.to_string(),
    }
}

/// Converts a displayed year into astronomical numbering. Year 0 does not exist.
pub fn from_display(era: Era, number: u32) -> Option<i32> {
    if number == 0 {
        return None;
    }
    let n = i32::try_from(number).ok()?;
    Some(match era {
        Era::Ad => n,
        Era::Bc => 1 - n,
    })
}

/// Why a year cannot be selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YearError {
    /// Before 100 BC.
    TooEarly,
    /// After the current calendar year.
    TooLate,
    /// Displayed year 0.
    NoYearZero,
}

/// Bounded year selection from 100 BC to the current calendar year.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YearPicker {
    min: i32,
    max: i32,
    year: i32,
}

impl YearPicker {
    /// Picker up to `current_year` (supplied by the host; the simulation never
    /// reads the wall clock). Starts at the earliest year.
    pub fn new(current_year: i32) -> Self {
        let max = current_year.max(MIN_START_YEAR);
        Self {
            min: MIN_START_YEAR,
            max,
            year: MIN_START_YEAR,
        }
    }

    /// Selected astronomical year.
    pub fn year(&self) -> i32 {
        self.year
    }

    /// Selectable range (inclusive).
    pub fn range(&self) -> (i32, i32) {
        (self.min, self.max)
    }

    /// Validates an astronomical year.
    pub fn check(&self, year: i32) -> Result<i32, YearError> {
        if year < self.min {
            Err(YearError::TooEarly)
        } else if year > self.max {
            Err(YearError::TooLate)
        } else {
            Ok(year)
        }
    }

    /// Selects an astronomical year.
    pub fn select(&mut self, year: i32) -> Result<(), YearError> {
        self.year = self.check(year)?;
        Ok(())
    }

    /// Selects a displayed year.
    pub fn select_display(&mut self, era: Era, number: u32) -> Result<(), YearError> {
        self.select(from_display(era, number).ok_or(YearError::NoYearZero)?)
    }

    /// Moves by `delta` years, clamped to the range.
    pub fn step(&mut self, delta: i32) {
        self.year = self.year.saturating_add(delta).clamp(self.min, self.max);
    }

    /// Quick-select years of the F-01 mockup: 100 BC, 1 AD, 476, 1066, 1492,
    /// 1815, 1914, 1936, current year.
    pub fn presets(&self) -> [i32; 9] {
        [-99, 1, 476, 1066, 1492, 1815, 1914, 1936, self.max]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_bc_steps_to_one_ad() {
        let mut p = YearPicker::new(2026);
        p.select_display(Era::Bc, 1).unwrap();
        assert_eq!(p.year(), 0);
        p.step(1);
        assert_eq!(
            to_display(p.year()),
            DisplayYear {
                era: Era::Ad,
                number: 1
            }
        );
        p.step(-1);
        assert_eq!(label_de(p.year()), "1 v. Chr.");
    }

    #[test]
    fn bounds_and_year_zero() {
        let mut p = YearPicker::new(2026);
        assert_eq!(p.select_display(Era::Bc, 101), Err(YearError::TooEarly));
        assert_eq!(p.select(2027), Err(YearError::TooLate));
        assert_eq!(p.select_display(Era::Ad, 0), Err(YearError::NoYearZero));
        p.select(2026).unwrap();
        p.step(10);
        assert_eq!(p.year(), 2026);
        p.step(-100_000);
        assert_eq!(p.year(), -99);
    }

    #[test]
    fn labels() {
        assert_eq!(label_de(-99), "100 v. Chr.");
        assert_eq!(label_de(1), "1 n. Chr.");
        assert_eq!(label_de(476), "476 n. Chr.");
        assert_eq!(label_de(1492), "1492");
        assert_eq!(YearPicker::new(2026).presets()[8], 2026);
    }
}
