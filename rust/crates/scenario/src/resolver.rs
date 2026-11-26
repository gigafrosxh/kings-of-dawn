//! ScenarioResolver – turns a (year, mode) choice into an initialisable start
//! state against a fixed atlas snapshot. Independent of the YearPicker.

use crate::atlas::{AtlasSnapshot, CoverageGap, DataQuality, DatasetVersion, PolityRecord};

/// Game mode chosen at setup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScenarioMode {
    /// Historical campaign: only polities of the atlas, realism rules.
    Historical = 0,
    /// Alternative history: atlas polities, divergence allowed later.
    AlternateHistory = 1,
    /// Freely configurable sandbox: custom polities allowed.
    Sandbox = 2,
}

impl ScenarioMode {
    /// Decodes the wire value.
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Historical,
            1 => Self::AlternateHistory,
            2 => Self::Sandbox,
            _ => return None,
        })
    }

    /// Whether players may create polities that are not in the atlas.
    pub fn allows_custom_polities(self) -> bool {
        self == Self::Sandbox
    }
}

/// Data availability of the resolved year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataStatus {
    /// Atlas covers the year.
    Complete,
    /// Year lies in a known gap; polities may be incomplete.
    Partial(CoverageGap),
    /// No polity at all for the year.
    Missing,
}

/// Resolved, initialisable start state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedScenario {
    /// Astronomical start year.
    pub year: i32,
    /// Mode.
    pub mode: ScenarioMode,
    /// Dataset used.
    pub dataset: DatasetVersion,
    /// Polities valid at the year, sorted by id.
    pub polities: Vec<PolityRecord>,
    /// Lowest data quality among those polities (None when empty).
    pub min_quality: Option<DataQuality>,
    /// Availability.
    pub status: DataStatus,
}

/// Resolves a start year. Every year in the picker range yields a state;
/// missing data is reported via [`DataStatus`], never hidden.
pub fn resolve(atlas: &AtlasSnapshot, year: i32, mode: ScenarioMode) -> ResolvedScenario {
    let polities: Vec<PolityRecord> = atlas.polities_at(year).cloned().collect();
    let status = match (polities.is_empty(), atlas.gap_at(year)) {
        (true, _) => DataStatus::Missing,
        (false, Some(gap)) => DataStatus::Partial(gap.clone()),
        (false, None) => DataStatus::Complete,
    };
    ResolvedScenario {
        year,
        mode,
        dataset: atlas.dataset.clone(),
        min_quality: polities.iter().map(|p| p.quality).min(),
        polities,
        status,
    }
}
