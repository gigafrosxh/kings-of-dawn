//! Canonical binary encoding of the atlas (shipped data format).
//!
//! Decoding re-validates every invariant, so a corrupted or hand-edited data
//! file can never produce an [`Atlas`] that violates them.

use kod_core::codec::{Decode, DecodeError, Decoder, Encode, Encoder};

use crate::atlas::Atlas;
use crate::geometry::{MultiPolygon, Point, Polygon, Ring};
use crate::interval::ValidityInterval;
use crate::model::{
    BorderVersion, Certainty, Layer, PolityAssertion, Precision, SourceDecision, SourceEntry,
    SourceManifest, SourceRef,
};

/// Magic + format version of the atlas file.
const MAGIC: [u8; 4] = *b"KODA";
const FORMAT_VERSION: u16 = 1;

impl Encode for ValidityInterval {
    fn encode(&self, enc: &mut Encoder) {
        enc.put(&self.from());
        enc.put(&self.to());
    }
}

impl Decode for ValidityInterval {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        let from = dec.get()?;
        let to = dec.get()?;
        Self::new(from, to).ok_or(DecodeError::Invalid("empty validity interval"))
    }
}

impl Encode for Point {
    fn encode(&self, enc: &mut Encoder) {
        enc.put(&self.lon);
        enc.put(&self.lat);
    }
}

impl Decode for Point {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            lon: dec.get()?,
            lat: dec.get()?,
        })
    }
}

impl Encode for Ring {
    fn encode(&self, enc: &mut Encoder) {
        enc.put(&self.0);
    }
}

impl Decode for Ring {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(Self(dec.get()?))
    }
}

impl Encode for Polygon {
    fn encode(&self, enc: &mut Encoder) {
        enc.put(&self.exterior);
        enc.put(&self.holes);
    }
}

impl Decode for Polygon {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            exterior: dec.get()?,
            holes: dec.get()?,
        })
    }
}

impl Encode for MultiPolygon {
    fn encode(&self, enc: &mut Encoder) {
        enc.put(&self.0);
    }
}

impl Decode for MultiPolygon {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(Self(dec.get()?))
    }
}

macro_rules! enum_codec {
    ($t:ty, $name:literal, $($v:literal => $variant:expr),+) => {
        impl Encode for $t {
            fn encode(&self, enc: &mut Encoder) {
                enc.put(&(*self as u8));
            }
        }
        impl Decode for $t {
            fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
                let tag: u8 = dec.get()?;
                match tag {
                    $($v => Ok($variant),)+
                    _ => Err(DecodeError::InvalidTag { ty: $name, tag: u32::from(tag) }),
                }
            }
        }
    };
}

enum_codec!(Layer, "Layer", 0 => Layer::DeFacto, 1 => Layer::DeJure);
enum_codec!(Certainty, "Certainty", 0 => Certainty::Certain, 1 => Certainty::Uncertain);
enum_codec!(Precision, "Precision", 0 => Precision::Year, 1 => Precision::Century);

impl Encode for SourceDecision {
    fn encode(&self, enc: &mut Encoder) {
        enc.put(&(*self as u8));
    }
}

impl Decode for SourceDecision {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        let tag: u8 = dec.get()?;
        Self::from_u8(tag).ok_or(DecodeError::InvalidTag {
            ty: "SourceDecision",
            tag: u32::from(tag),
        })
    }
}

impl Encode for SourceEntry {
    fn encode(&self, enc: &mut Encoder) {
        enc.put(&self.id);
        enc.put(&self.name);
        enc.put(&self.version);
        enc.put(&self.decision);
        enc.put(&self.commercial_use);
        enc.put(&self.shippable_licenses);
        enc.put(&self.attribution);
    }
}

impl Decode for SourceEntry {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            id: dec.get()?,
            name: dec.get()?,
            version: dec.get()?,
            decision: dec.get()?,
            commercial_use: dec.get()?,
            shippable_licenses: dec.get()?,
            attribution: dec.get()?,
        })
    }
}

impl Encode for SourceRef {
    fn encode(&self, enc: &mut Encoder) {
        enc.put(&self.source);
        enc.put(&self.record);
        enc.put(&self.license);
    }
}

impl Decode for SourceRef {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            source: dec.get()?,
            record: dec.get()?,
            license: dec.get()?,
        })
    }
}

impl Encode for PolityAssertion {
    fn encode(&self, enc: &mut Encoder) {
        enc.put(&self.polity);
        enc.put(&self.name);
        enc.put(&self.interval);
        enc.put(&self.quality);
        enc.put(&self.source);
    }
}

impl Decode for PolityAssertion {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            polity: dec.get()?,
            name: dec.get()?,
            interval: dec.get()?,
            quality: dec.get()?,
            source: dec.get()?,
        })
    }
}

impl Encode for BorderVersion {
    fn encode(&self, enc: &mut Encoder) {
        enc.put(&self.id);
        enc.put(&self.polity);
        enc.put(&self.layer);
        enc.put(&self.interval);
        enc.put(&self.certainty);
        enc.put(&self.precision);
        enc.put(&self.geometry);
        enc.put(&self.source);
    }
}

impl Decode for BorderVersion {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            id: dec.get()?,
            polity: dec.get()?,
            layer: dec.get()?,
            interval: dec.get()?,
            certainty: dec.get()?,
            precision: dec.get()?,
            geometry: dec.get()?,
            source: dec.get()?,
        })
    }
}

impl Encode for Atlas {
    fn encode(&self, enc: &mut Encoder) {
        enc.put_raw(&MAGIC);
        enc.put(&FORMAT_VERSION);
        enc.put(&self.dataset_id);
        enc.put(&self.dataset_version);
        enc.put(&self.manifest.sources);
        enc.put(&self.polities);
        enc.put(&self.borders);
        enc.put(&self.gaps);
    }
}

impl Decode for Atlas {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        if dec.take_array::<4>()? != MAGIC {
            return Err(DecodeError::Invalid("not an atlas file"));
        }
        let version: u16 = dec.get()?;
        if version != FORMAT_VERSION {
            return Err(DecodeError::Invalid("unsupported atlas format version"));
        }
        let dataset_id = dec.get()?;
        let dataset_version = dec.get()?;
        let manifest = SourceManifest {
            sources: dec.get()?,
        };
        let polities = dec.get()?;
        let borders = dec.get()?;
        let gaps = dec.get()?;
        let atlas = Atlas::new(
            dataset_id,
            dataset_version,
            manifest,
            polities,
            borders,
            gaps,
        )
        .map_err(|_| DecodeError::Invalid("atlas invariants violated"))?;
        Ok(atlas)
    }
}
