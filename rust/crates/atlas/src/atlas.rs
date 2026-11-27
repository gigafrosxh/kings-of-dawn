//! Validated atlas container and year queries.

use std::collections::BTreeSet;
use std::fmt;

use kod_core::hash::StateHash;
use kod_scenario::atlas::{CoverageGap, PolityId};

use crate::geometry::GeometryError;
use crate::model::{BorderId, BorderVersion, Layer, PolityAssertion, SourceManifest, SourceRef};

/// A validated historical atlas.
///
/// Construct with [`Atlas::new`]; all invariants listed in the crate docs hold
/// for every value of this type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atlas {
    pub(crate) dataset_id: String,
    pub(crate) dataset_version: String,
    pub(crate) manifest: SourceManifest,
    pub(crate) polities: Vec<PolityAssertion>,
    pub(crate) borders: Vec<BorderVersion>,
    pub(crate) gaps: Vec<CoverageGap>,
}

/// A violated atlas invariant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtlasError {
    /// Source ids in the manifest are not unique.
    DuplicateSource(String),
    /// An object references a source missing from the manifest.
    UnknownSource(SourceRef),
    /// The source's manifest decision forbids shipping its data.
    SourceNotShippable(SourceRef),
    /// The source's license does not allow commercial use (#57).
    NotCommercial(SourceRef),
    /// The record license is not in the source's shippable list.
    RecordLicenseNotShippable(SourceRef),
    /// Two assertions of the same polity overlap in time.
    OverlappingAssertions(PolityId),
    /// A border version is not covered by one assertion of its polity.
    BorderWithoutPolity(BorderId),
    /// Two border versions of the same polity and layer overlap in time.
    OverlappingBorders(BorderId, BorderId),
    /// Border ids are not unique.
    DuplicateBorder(BorderId),
    /// Invalid geometry.
    Geometry(BorderId, GeometryError),
    /// A coverage gap is empty or has no reason.
    InvalidGap(i32, i32),
}

impl fmt::Display for AtlasError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateSource(id) => write!(f, "duplicate source {id}"),
            Self::UnknownSource(r) => write!(f, "unknown source {} ({})", r.source, r.record),
            Self::SourceNotShippable(r) => {
                write!(f, "source {} may not ship ({})", r.source, r.record)
            }
            Self::NotCommercial(r) => {
                write!(
                    f,
                    "source {} not licensed for commercial use ({})",
                    r.source, r.record
                )
            }
            Self::RecordLicenseNotShippable(r) => {
                write!(
                    f,
                    "license {} not shippable for {} ({})",
                    r.license, r.source, r.record
                )
            }
            Self::OverlappingAssertions(p) => write!(f, "overlapping assertions for {p}"),
            Self::BorderWithoutPolity(b) => write!(f, "{b} not covered by a polity assertion"),
            Self::OverlappingBorders(a, b) => write!(f, "{a} and {b} overlap in time"),
            Self::DuplicateBorder(b) => write!(f, "duplicate border {b}"),
            Self::Geometry(b, e) => write!(f, "{b}: {e}"),
            Self::InvalidGap(from, to) => write!(f, "invalid coverage gap [{from}, {to})"),
        }
    }
}

/// Coverage of one year.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coverage<'a> {
    /// At least one border version is valid and no gap is declared.
    Covered,
    /// A coverage gap is declared for the year.
    Gap(&'a CoverageGap),
    /// No border and no declared gap: missing data that must be reported.
    Undeclared,
}

/// Atlas state of one year. Only objects valid in that exact year are
/// returned; nothing is interpolated between versions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YearState<'a> {
    /// Queried year (astronomical).
    pub year: i32,
    /// Polity assertions valid in the year, sorted by polity id.
    pub polities: Vec<&'a PolityAssertion>,
    /// Border versions valid in the year, sorted by (layer, polity).
    pub borders: Vec<&'a BorderVersion>,
    /// Coverage classification.
    pub coverage: Coverage<'a>,
}

impl Atlas {
    /// Validates and builds an atlas. Returns every violation found.
    pub fn new(
        dataset_id: String,
        dataset_version: String,
        mut manifest: SourceManifest,
        mut polities: Vec<PolityAssertion>,
        mut borders: Vec<BorderVersion>,
        mut gaps: Vec<CoverageGap>,
    ) -> Result<Self, Vec<AtlasError>> {
        manifest.sources.sort_by(|a, b| a.id.cmp(&b.id));
        polities.sort_by_key(|p| (p.polity, p.interval));
        borders.sort_by_key(|b| b.id);
        gaps.sort_by_key(|g| (g.from, g.to));
        let atlas = Self {
            dataset_id,
            dataset_version,
            manifest,
            polities,
            borders,
            gaps,
        };
        let errors = atlas.violations();
        if errors.is_empty() {
            Ok(atlas)
        } else {
            Err(errors)
        }
    }

    pub(crate) fn violations(&self) -> Vec<AtlasError> {
        let mut errors = Vec::new();
        for w in self.manifest.sources.windows(2) {
            if w[0].id == w[1].id {
                errors.push(AtlasError::DuplicateSource(w[0].id.clone()));
            }
        }
        let refs = self
            .polities
            .iter()
            .map(|p| &p.source)
            .chain(self.borders.iter().map(|b| &b.source));
        for r in refs {
            if let Some(e) = self.check_source(r) {
                errors.push(e);
            }
        }
        for w in self.polities.windows(2) {
            if w[0].polity == w[1].polity && w[0].interval.overlaps(w[1].interval) {
                errors.push(AtlasError::OverlappingAssertions(w[0].polity));
            }
        }
        for w in self.borders.windows(2) {
            if w[0].id == w[1].id {
                errors.push(AtlasError::DuplicateBorder(w[0].id));
            }
        }
        for (i, b) in self.borders.iter().enumerate() {
            let covered = self
                .polities
                .iter()
                .any(|p| p.polity == b.polity && p.interval.covers(b.interval));
            if !covered {
                errors.push(AtlasError::BorderWithoutPolity(b.id));
            }
            if let Err(e) = b.geometry.validate() {
                errors.push(AtlasError::Geometry(b.id, e));
            }
            for o in &self.borders[i + 1..] {
                if o.polity == b.polity && o.layer == b.layer && o.interval.overlaps(b.interval) {
                    errors.push(AtlasError::OverlappingBorders(b.id, o.id));
                }
            }
        }
        for g in &self.gaps {
            if g.from >= g.to || g.reason.trim().is_empty() {
                errors.push(AtlasError::InvalidGap(g.from, g.to));
            }
        }
        errors
    }

    fn check_source(&self, r: &SourceRef) -> Option<AtlasError> {
        let Some(src) = self.manifest.get(&r.source) else {
            return Some(AtlasError::UnknownSource(r.clone()));
        };
        if !src.decision.may_ship() {
            return Some(AtlasError::SourceNotShippable(r.clone()));
        }
        if !src.commercial_use {
            return Some(AtlasError::NotCommercial(r.clone()));
        }
        if !src.shippable_licenses.contains(&r.license) {
            return Some(AtlasError::RecordLicenseNotShippable(r.clone()));
        }
        None
    }

    /// Dataset id, e.g. `kod.atlas`.
    pub fn dataset_id(&self) -> &str {
        &self.dataset_id
    }

    /// Dataset version.
    pub fn dataset_version(&self) -> &str {
        &self.dataset_version
    }

    /// Sources this atlas was built from.
    pub fn manifest(&self) -> &SourceManifest {
        &self.manifest
    }

    /// All polity assertions, sorted by (polity, interval).
    pub fn polities(&self) -> &[PolityAssertion] {
        &self.polities
    }

    /// All border versions, sorted by id.
    pub fn borders(&self) -> &[BorderVersion] {
        &self.borders
    }

    /// Declared coverage gaps, sorted by interval.
    pub fn gaps(&self) -> &[CoverageGap] {
        &self.gaps
    }

    /// Looks up a border version by id.
    pub fn border(&self, id: BorderId) -> Option<&BorderVersion> {
        self.borders
            .binary_search_by_key(&id, |b| b.id)
            .ok()
            .map(|i| &self.borders[i])
    }

    /// Provenance of a border version (source record and license).
    pub fn provenance(&self, id: BorderId) -> Option<&SourceRef> {
        self.border(id).map(|b| &b.source)
    }

    /// State of `year`. Only objects whose interval contains the year are
    /// returned; a year between two versions is never filled by interpolation.
    pub fn state_at(&self, year: i32) -> YearState<'_> {
        let polities: Vec<&PolityAssertion> = self
            .polities
            .iter()
            .filter(|p| p.interval.contains(year))
            .collect();
        let mut borders: Vec<&BorderVersion> = self
            .borders
            .iter()
            .filter(|b| b.interval.contains(year))
            .collect();
        borders.sort_by_key(|b| (b.layer, b.polity));
        let coverage = match self.gaps.iter().find(|g| g.from <= year && year < g.to) {
            Some(g) => Coverage::Gap(g),
            None if borders.iter().any(|b| b.layer == Layer::DeFacto) => Coverage::Covered,
            None => Coverage::Undeclared,
        };
        YearState {
            year,
            polities,
            borders,
            coverage,
        }
    }

    /// Content fingerprint of the canonical encoding.
    pub fn checksum(&self) -> StateHash {
        StateHash::of(self)
    }

    /// Every polity id appearing in the atlas.
    pub fn polity_ids(&self) -> BTreeSet<PolityId> {
        self.polities.iter().map(|p| p.polity).collect()
    }
}
