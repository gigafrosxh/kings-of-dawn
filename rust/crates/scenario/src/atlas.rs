//! Atlas contract consumed by F-01 and implemented productively by F-02 (#12).
//!
//! Until F-02 exists, [`FixtureAtlas`] provides an explicitly synthetic M0
//! dataset: generic test polities, no historical claims, and deliberate
//! coverage gaps so missing data paths are exercised.

use kod_core::codec::{Decode, DecodeError, Decoder, Encode, Encoder};
use kod_core::hash::StateHash;

kod_core::define_id!(
    /// Stable polity identifier shared with map cells (#9) and later domains.
    PolityId,
    "pol"
);

/// Reliability of the data for a polity or year.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DataQuality {
    /// Synthetic test data (M0 fixture).
    Synthetic = 0,
    /// Rough approximation.
    Approximate = 1,
    /// Reconstructed from secondary sources.
    Reconstructed = 2,
    /// Documented by primary sources.
    Documented = 3,
}

impl DataQuality {
    fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Synthetic,
            1 => Self::Approximate,
            2 => Self::Reconstructed,
            3 => Self::Documented,
            _ => return None,
        })
    }
}

/// Exact dataset a scenario was resolved against; fixed in every save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetVersion {
    /// Dataset id, e.g. `kod.fixture.m0` or `cliopatria`.
    pub id: String,
    /// Dataset version, e.g. `v0.2.2`.
    pub version: String,
    /// Fingerprint of the dataset content.
    pub checksum: StateHash,
}

/// A polity valid in `[valid_from, valid_to)` (astronomical years).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolityRecord {
    /// Stable id.
    pub id: PolityId,
    /// Display name.
    pub name: String,
    /// First year (inclusive).
    pub valid_from: i32,
    /// End year (exclusive).
    pub valid_to: i32,
    /// Data quality.
    pub quality: DataQuality,
}

impl PolityRecord {
    /// `true` when the polity exists in `year`.
    pub fn valid_in(&self, year: i32) -> bool {
        self.valid_from <= year && year < self.valid_to
    }
}

/// A period without (complete) coverage in `[from, to)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageGap {
    /// First year (inclusive).
    pub from: i32,
    /// End year (exclusive).
    pub to: i32,
    /// Human-readable reason.
    pub reason: String,
}

/// Read-only atlas contract.
pub trait AtlasSource {
    /// Dataset identity.
    fn dataset(&self) -> DatasetVersion;
    /// All polity records (any order).
    fn polities(&self) -> Vec<PolityRecord>;
    /// Known coverage gaps.
    fn gaps(&self) -> Vec<CoverageGap>;
}

/// Owned snapshot of an atlas, stored inside the setup state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtlasSnapshot {
    /// Dataset identity.
    pub dataset: DatasetVersion,
    /// Polities sorted by id.
    pub polities: Vec<PolityRecord>,
    /// Gaps sorted by start year.
    pub gaps: Vec<CoverageGap>,
}

impl AtlasSnapshot {
    /// Copies an atlas into canonical order and verifies its checksum.
    pub fn capture(source: &dyn AtlasSource) -> Result<Self, String> {
        let mut polities = source.polities();
        polities.sort_by_key(|p| p.id);
        if polities.windows(2).any(|w| w[0].id == w[1].id) {
            return Err("duplicate polity id".into());
        }
        if let Some(p) = polities.iter().find(|p| p.valid_from >= p.valid_to) {
            return Err(format!("{} has an empty validity interval", p.id));
        }
        let mut gaps = source.gaps();
        gaps.sort_by_key(|g| g.from);
        let dataset = source.dataset();
        let actual = content_checksum(&polities, &gaps);
        if actual != dataset.checksum {
            return Err(format!(
                "dataset checksum {actual} does not match declared {}",
                dataset.checksum
            ));
        }
        Ok(Self {
            dataset,
            polities,
            gaps,
        })
    }

    /// Polities valid in `year`, sorted by id.
    pub fn polities_at(&self, year: i32) -> impl Iterator<Item = &PolityRecord> {
        self.polities.iter().filter(move |p| p.valid_in(year))
    }

    /// Gap covering `year`, if any.
    pub fn gap_at(&self, year: i32) -> Option<&CoverageGap> {
        self.gaps.iter().find(|g| g.from <= year && year < g.to)
    }

    /// Polity by id.
    pub fn polity(&self, id: PolityId) -> Option<&PolityRecord> {
        self.polities
            .binary_search_by_key(&id, |p| p.id)
            .ok()
            .map(|i| &self.polities[i])
    }
}

/// Canonical checksum over polity records and gaps.
pub fn content_checksum(polities: &[PolityRecord], gaps: &[CoverageGap]) -> StateHash {
    let mut enc = Encoder::new();
    enc.put(&polities.to_vec());
    enc.put(&gaps.to_vec());
    StateHash::of_bytes(enc.as_bytes())
}

/// Synthetic M0 atlas (no historical claims; replaced by F-02).
#[derive(Debug, Clone, Copy, Default)]
pub struct FixtureAtlas;

impl FixtureAtlas {
    fn records() -> Vec<PolityRecord> {
        let p = |id: u32, name: &str, from: i32, to: i32| PolityRecord {
            id: PolityId::new(id),
            name: name.to_string(),
            valid_from: from,
            valid_to: to,
            quality: DataQuality::Synthetic,
        };
        vec![
            p(1, "Testreich Alpha", -99, 500),
            p(2, "Testbund Beta", -99, 2101),
            p(3, "Testkönigreich Gamma", 400, 1500),
            p(4, "Testrepublik Delta", 1000, 2101),
            p(5, "Teststaat Epsilon", 1800, 2101),
            p(6, "Testimperium Zeta", 1850, 1950),
        ]
    }

    fn gap_records() -> Vec<CoverageGap> {
        vec![CoverageGap {
            from: 600,
            to: 700,
            reason: "M0-Fixture: absichtliche Datenlücke".into(),
        }]
    }
}

impl AtlasSource for FixtureAtlas {
    fn dataset(&self) -> DatasetVersion {
        let mut polities = Self::records();
        polities.sort_by_key(|p| p.id);
        DatasetVersion {
            id: "kod.fixture.m0".into(),
            version: "1".into(),
            checksum: content_checksum(&polities, &Self::gap_records()),
        }
    }

    fn polities(&self) -> Vec<PolityRecord> {
        Self::records()
    }

    fn gaps(&self) -> Vec<CoverageGap> {
        Self::gap_records()
    }
}

impl Encode for DataQuality {
    fn encode(&self, enc: &mut Encoder) {
        enc.put(&(*self as u8));
    }
}

impl Decode for DataQuality {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        let tag: u8 = dec.get()?;
        Self::from_u8(tag).ok_or(DecodeError::InvalidTag {
            ty: "DataQuality",
            tag: tag.into(),
        })
    }
}

impl Encode for DatasetVersion {
    fn encode(&self, enc: &mut Encoder) {
        enc.put(&self.id);
        enc.put(&self.version);
        enc.put(&self.checksum);
    }
}

impl Decode for DatasetVersion {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            id: dec.get()?,
            version: dec.get()?,
            checksum: dec.get()?,
        })
    }
}

impl Encode for PolityRecord {
    fn encode(&self, enc: &mut Encoder) {
        enc.put(&self.id);
        enc.put(&self.name);
        enc.put(&self.valid_from);
        enc.put(&self.valid_to);
        enc.put(&self.quality);
    }
}

impl Decode for PolityRecord {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            id: dec.get()?,
            name: dec.get()?,
            valid_from: dec.get()?,
            valid_to: dec.get()?,
            quality: dec.get()?,
        })
    }
}

impl Encode for CoverageGap {
    fn encode(&self, enc: &mut Encoder) {
        enc.put(&self.from);
        enc.put(&self.to);
        enc.put(&self.reason);
    }
}

impl Decode for CoverageGap {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            from: dec.get()?,
            to: dec.get()?,
            reason: dec.get()?,
        })
    }
}

impl Encode for AtlasSnapshot {
    fn encode(&self, enc: &mut Encoder) {
        enc.put(&self.dataset);
        enc.put(&self.polities);
        enc.put(&self.gaps);
    }
}

impl Decode for AtlasSnapshot {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        let snap = Self {
            dataset: dec.get()?,
            polities: dec.get()?,
            gaps: dec.get()?,
        };
        if content_checksum(&snap.polities, &snap.gaps) != snap.dataset.checksum {
            return Err(DecodeError::Invalid("atlas snapshot checksum mismatch"));
        }
        Ok(snap)
    }
}
