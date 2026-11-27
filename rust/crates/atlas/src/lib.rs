//! F-02 – Historical atlas and map import (issue #12).
//!
//! Runtime model of the historical atlas. Data is prepared offline from the
//! sources in `data/sources/manifest.json` (ADR-0004) and shipped in the
//! canonical binary encoding of [`Atlas`].
//!
//! - [`interval`]: [`ValidityInterval`] `[from, to)` in astronomical years.
//! - [`geometry`]: integer micro-degree polygons and their validation.
//! - [`model`]: [`SourceManifest`], [`PolityAssertion`], [`BorderVersion`].
//! - [`Atlas`]: validated container with year queries that never interpolate.
//!
//! Rules enforced by [`Atlas::new`]:
//! - only sources whose manifest decision allows shipping and whose license
//!   permits commercial use (owner rule, #57);
//! - every border version belongs to a polity assertion that covers it;
//! - at most one border version per polity and layer for any year;
//! - all geometries are valid.

#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]
#![warn(missing_docs)]

pub mod geometry;
pub mod interval;
pub mod model;

mod atlas;
mod codec;

pub use atlas::{Atlas, AtlasError, Coverage, YearState};
pub use geometry::{GeometryError, MultiPolygon, Point, Polygon, Ring};
pub use interval::ValidityInterval;
pub use model::{
    BorderId, BorderVersion, Certainty, Layer, PolityAssertion, Precision, SourceDecision,
    SourceEntry, SourceManifest, SourceRef,
};
