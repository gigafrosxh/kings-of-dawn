//! Atlas data model: source manifest, polity assertions and border versions.

use kod_scenario::atlas::{DataQuality, PolityId};

use crate::geometry::MultiPolygon;
use crate::interval::ValidityInterval;

kod_core::define_id!(
    /// Stable id of one border version.
    BorderId,
    "brd"
);

/// Manifest decision for a source (`data/sources/manifest.json`, ADR-0004).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceDecision {
    /// Shipped in builds.
    Bundle = 0,
    /// Shipped after per-object filtering.
    BundleFiltered = 1,
    /// Shipped only for records whose license is listed as shippable.
    Conditional = 2,
    /// Used by tools only; never in shipped data.
    ToolingOnly = 3,
    /// Not used at all.
    Exclude = 4,
    /// Methodology reference, no data.
    Reference = 5,
}

impl SourceDecision {
    /// Parses the manifest spelling (`bundle`, `bundle-filtered`, ...).
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "bundle" => Self::Bundle,
            "bundle-filtered" => Self::BundleFiltered,
            "conditional" => Self::Conditional,
            "tooling-only" => Self::ToolingOnly,
            "exclude" => Self::Exclude,
            "reference" => Self::Reference,
            _ => return None,
        })
    }

    /// `true` when data from this source may appear in shipped atlas data.
    pub const fn may_ship(self) -> bool {
        matches!(
            self,
            Self::Bundle | Self::BundleFiltered | Self::Conditional
        )
    }

    pub(crate) fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Bundle,
            1 => Self::BundleFiltered,
            2 => Self::Conditional,
            3 => Self::ToolingOnly,
            4 => Self::Exclude,
            5 => Self::Reference,
            _ => return None,
        })
    }
}

/// One source of the manifest as needed at runtime (credits, provenance).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceEntry {
    /// Manifest id, e.g. `A`.
    pub id: String,
    /// Display name, e.g. `Cliopatria`.
    pub name: String,
    /// Exact version or snapshot used for this atlas.
    pub version: String,
    /// Manifest decision.
    pub decision: SourceDecision,
    /// License allows commercial use (owner rule, #57: required for shipping).
    pub commercial_use: bool,
    /// Record licenses that may be shipped from this source.
    pub shippable_licenses: Vec<String>,
    /// Attribution text shown in the credits screen.
    pub attribution: String,
}

/// The sources an atlas was built from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceManifest {
    /// Sources sorted by id.
    pub sources: Vec<SourceEntry>,
}

impl SourceManifest {
    /// Looks up a source by manifest id.
    pub fn get(&self, id: &str) -> Option<&SourceEntry> {
        self.sources.iter().find(|s| s.id == id)
    }
}

/// Provenance of one atlas object: which source record it came from.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceRef {
    /// Manifest id of the source.
    pub source: String,
    /// Record id inside the source (e.g. Cliopatria feature id).
    pub record: String,
    /// License of this record.
    pub license: String,
}

/// Map layer: actual control or claimed territory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Layer {
    /// Herrschaft – de facto control.
    DeFacto = 0,
    /// Anspruch – de jure claim.
    DeJure = 1,
}

/// Whether a border is established or uncertain (rendered hatched).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Certainty {
    /// Established by the source.
    Certain = 0,
    /// Disputed or poorly documented; rendered hatched.
    Uncertain = 1,
}

/// Temporal precision of the source for this object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Precision {
    /// Dated to the year.
    Year = 0,
    /// Dated to the century only.
    Century = 1,
}

/// A polity exists with a name during an interval, according to a source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolityAssertion {
    /// Stable polity id shared with scenarios and map cells.
    pub polity: PolityId,
    /// Display name during this interval.
    pub name: String,
    /// Validity.
    pub interval: ValidityInterval,
    /// Data quality.
    pub quality: DataQuality,
    /// Provenance.
    pub source: SourceRef,
}

/// The territory of a polity on one layer during an interval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BorderVersion {
    /// Stable id.
    pub id: BorderId,
    /// Owning polity.
    pub polity: PolityId,
    /// Layer.
    pub layer: Layer,
    /// Validity; the geometry is exact for every year inside it.
    pub interval: ValidityInterval,
    /// Certainty.
    pub certainty: Certainty,
    /// Temporal precision of the source.
    pub precision: Precision,
    /// Territory.
    pub geometry: MultiPolygon,
    /// Provenance.
    pub source: SourceRef,
}
