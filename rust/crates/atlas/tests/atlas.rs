//! Contract tests for the F-02 atlas model (#12).

use kod_atlas::{
    Atlas, AtlasError, BorderId, BorderVersion, Certainty, Coverage, GeometryError, Layer,
    MultiPolygon, Point, PolityAssertion, Polygon, Precision, Ring, SourceDecision, SourceEntry,
    SourceManifest, SourceRef, ValidityInterval,
};
use kod_core::codec::{Decode, Encode};
use kod_scenario::atlas::{CoverageGap, DataQuality, PolityId};

const M: i32 = 1_000_000;

fn square(x0: i32, y0: i32, s: i32) -> MultiPolygon {
    let p = |x: i32, y: i32| Point::new(x * M, y * M);
    MultiPolygon(vec![Polygon {
        exterior: Ring(vec![
            p(x0, y0),
            p(x0 + s, y0),
            p(x0 + s, y0 + s),
            p(x0, y0 + s),
            p(x0, y0),
        ]),
        holes: vec![],
    }])
}

fn iv(from: i32, to: i32) -> ValidityInterval {
    ValidityInterval::new(from, to).unwrap()
}

fn src(id: &str, decision: SourceDecision, commercial: bool, licenses: &[&str]) -> SourceEntry {
    SourceEntry {
        id: id.into(),
        name: format!("Test source {id}"),
        version: "test".into(),
        decision,
        commercial_use: commercial,
        shippable_licenses: licenses.iter().map(|s| s.to_string()).collect(),
        attribution: format!("© Test {id}"),
    }
}

fn manifest() -> SourceManifest {
    SourceManifest {
        sources: vec![
            src("A", SourceDecision::Bundle, true, &["CC-BY-4.0"]),
            src("B", SourceDecision::ToolingOnly, true, &["GPL-3.0"]),
            src("D", SourceDecision::Exclude, false, &["CC-BY-NC-SA-4.0"]),
            src("F", SourceDecision::Conditional, true, &["CC-BY-4.0"]),
            src("X", SourceDecision::Bundle, false, &["Custom-NC"]),
        ],
    }
}

fn sref(source: &str, record: &str, license: &str) -> SourceRef {
    SourceRef {
        source: source.into(),
        record: record.into(),
        license: license.into(),
    }
}

fn polity(id: u32, name: &str, from: i32, to: i32) -> PolityAssertion {
    PolityAssertion {
        polity: PolityId::new(id),
        name: name.into(),
        interval: iv(from, to),
        quality: DataQuality::Synthetic,
        source: sref("A", &format!("pol-{id}"), "CC-BY-4.0"),
    }
}

fn border(
    id: u32,
    pol: u32,
    layer: Layer,
    from: i32,
    to: i32,
    geom: MultiPolygon,
) -> BorderVersion {
    BorderVersion {
        id: BorderId::new(id),
        polity: PolityId::new(pol),
        layer,
        interval: iv(from, to),
        certainty: Certainty::Certain,
        precision: Precision::Year,
        geometry: geom,
        source: sref("A", &format!("brd-{id}"), "CC-BY-4.0"),
    }
}

fn gap(from: i32, to: i32) -> CoverageGap {
    CoverageGap {
        from,
        to,
        reason: "Testlücke".into(),
    }
}

fn build(
    polities: Vec<PolityAssertion>,
    borders: Vec<BorderVersion>,
    gaps: Vec<CoverageGap>,
) -> Result<Atlas, Vec<AtlasError>> {
    Atlas::new(
        "kod.atlas.test".into(),
        "1".into(),
        manifest(),
        polities,
        borders,
        gaps,
    )
}

/// Polity 1 exists 0..1000 with two border versions and a hole 300..400
/// between them; polity 2 exists 500..1000; a gap is declared 600..700.
fn sample() -> Atlas {
    build(
        vec![
            polity(1, "Testreich", 0, 1000),
            polity(2, "Testbund", 500, 1000),
        ],
        vec![
            border(10, 1, Layer::DeFacto, 0, 300, square(0, 0, 10)),
            border(11, 1, Layer::DeFacto, 400, 1000, square(0, 0, 20)),
            border(12, 1, Layer::DeJure, 0, 1000, square(0, 0, 30)),
            border(20, 2, Layer::DeFacto, 500, 1000, square(40, 0, 10)),
        ],
        vec![gap(600, 700)],
    )
    .expect("sample atlas is valid")
}

#[test]
fn year_query_returns_exact_versions() {
    let a = sample();
    let s = a.state_at(250);
    assert_eq!(s.coverage, Coverage::Covered);
    let ids: Vec<u32> = s.borders.iter().map(|b| b.id.get()).collect();
    assert_eq!(ids, vec![10, 12], "de facto first, then de jure");
    assert_eq!(s.polities.len(), 1);

    let s = a.state_at(500);
    let ids: Vec<u32> = s.borders.iter().map(|b| b.id.get()).collect();
    assert_eq!(ids, vec![11, 20, 12]);
}

#[test]
fn no_interpolation_between_versions() {
    let a = sample();
    // 300..400: no de facto version of polity 1, nothing declared.
    let s = a.state_at(350);
    assert!(s.borders.iter().all(|b| b.layer == Layer::DeJure));
    assert_eq!(
        s.coverage,
        Coverage::Undeclared,
        "missing data is explicit, not interpolated"
    );
}

#[test]
fn declared_gap_is_reported() {
    let a = sample();
    match a.state_at(650).coverage {
        Coverage::Gap(g) => assert_eq!((g.from, g.to), (600, 700)),
        other => panic!("expected gap, got {other:?}"),
    }
    assert_eq!(a.state_at(5000).coverage, Coverage::Undeclared);
}

#[test]
fn provenance_is_traceable() {
    let a = sample();
    let r = a.provenance(BorderId::new(11)).unwrap();
    assert_eq!(
        (r.source.as_str(), r.record.as_str(), r.license.as_str()),
        ("A", "brd-11", "CC-BY-4.0")
    );
    assert!(a.provenance(BorderId::new(99)).is_none());
}

#[test]
fn only_commercially_shippable_sources() {
    let mut b = border(30, 1, Layer::DeFacto, 0, 100, square(0, 0, 1));
    let cases = [
        (sref("B", "x", "GPL-3.0"), "tooling-only"),
        (sref("D", "x", "CC-BY-NC-SA-4.0"), "excluded"),
        (sref("X", "x", "Custom-NC"), "non-commercial"),
        (sref("F", "x", "ODbL-1.0"), "record license"),
        (sref("Z", "x", "CC0-1.0"), "unknown"),
    ];
    for (r, what) in cases {
        b.source = r.clone();
        let err = build(vec![polity(1, "T", 0, 1000)], vec![b.clone()], vec![]).unwrap_err();
        let expected = match what {
            "tooling-only" | "excluded" => AtlasError::SourceNotShippable(r),
            "non-commercial" => AtlasError::NotCommercial(r),
            "record license" => AtlasError::RecordLicenseNotShippable(r),
            _ => AtlasError::UnknownSource(r),
        };
        assert_eq!(err, vec![expected], "{what}");
    }
    b.source = sref("F", "x", "CC-BY-4.0");
    assert!(build(vec![polity(1, "T", 0, 1000)], vec![b], vec![]).is_ok());
}

#[test]
fn structural_invariants() {
    let p = vec![polity(1, "T", 0, 100)];
    let overlap = build(
        p.clone(),
        vec![
            border(1, 1, Layer::DeFacto, 0, 60, square(0, 0, 1)),
            border(2, 1, Layer::DeFacto, 50, 100, square(0, 0, 1)),
        ],
        vec![],
    );
    assert_eq!(
        overlap.unwrap_err(),
        vec![AtlasError::OverlappingBorders(
            BorderId::new(1),
            BorderId::new(2)
        )]
    );

    let outside = build(
        p.clone(),
        vec![border(1, 1, Layer::DeFacto, 50, 150, square(0, 0, 1))],
        vec![],
    );
    assert_eq!(
        outside.unwrap_err(),
        vec![AtlasError::BorderWithoutPolity(BorderId::new(1))]
    );

    let dup_polity = build(
        vec![polity(1, "T", 0, 100), polity(1, "T2", 50, 200)],
        vec![],
        vec![],
    );
    assert_eq!(
        dup_polity.unwrap_err(),
        vec![AtlasError::OverlappingAssertions(PolityId::new(1))]
    );

    let bad_geom = build(
        p.clone(),
        vec![border(1, 1, Layer::DeFacto, 0, 10, MultiPolygon(vec![]))],
        vec![],
    );
    assert_eq!(
        bad_geom.unwrap_err(),
        vec![AtlasError::Geometry(BorderId::new(1), GeometryError::Empty)]
    );

    let bad_gap = build(
        p,
        vec![],
        vec![CoverageGap {
            from: 5,
            to: 5,
            reason: "x".into(),
        }],
    );
    assert_eq!(bad_gap.unwrap_err(), vec![AtlasError::InvalidGap(5, 5)]);
}

#[test]
fn codec_roundtrip_and_checksum() {
    let a = sample();
    let bytes = a.to_canonical_bytes();
    let b = Atlas::from_canonical_bytes(&bytes).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.checksum(), b.checksum());
    assert_eq!(bytes, b.to_canonical_bytes(), "encoding is canonical");
}

#[test]
fn corrupted_files_are_rejected() {
    let bytes = sample().to_canonical_bytes();
    assert!(Atlas::from_canonical_bytes(&bytes[..bytes.len() - 1]).is_err());
    let mut wrong_magic = bytes.clone();
    wrong_magic[0] = b'X';
    assert!(Atlas::from_canonical_bytes(&wrong_magic).is_err());
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(Atlas::from_canonical_bytes(&trailing).is_err());
}

#[test]
fn non_commercial_manifest_invalidates_atlas() {
    // Same objects, but source A re-labelled as non-commercial: every object fails.
    let a = sample();
    let mut m = a.manifest().clone();
    m.sources[0].commercial_use = false;
    let err = Atlas::new(
        "t".into(),
        "1".into(),
        m,
        a.polities().to_vec(),
        a.borders().to_vec(),
        vec![],
    )
    .unwrap_err();
    assert_eq!(err.len(), a.polities().len() + a.borders().len());
    assert!(
        err.iter()
            .all(|e| matches!(e, AtlasError::NotCommercial(_)))
    );
}
