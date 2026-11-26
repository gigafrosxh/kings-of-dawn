//! F-01 – Start year and scenarios (issue #11).
//!
//! - [`year`]: YearPicker (100 BC .. current year, no year 0).
//! - [`atlas`]: atlas contract for F-02 and the synthetic M0 fixture.
//! - [`resolver`]: ScenarioResolver (year + mode -> initialisable state).
//! - [`ScenarioSetup`]: kernel domain with typed commands, queries and events;
//!   the dataset version is part of the state and therefore fixed in saves.

#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]
#![warn(missing_docs)]

pub mod atlas;
pub mod resolver;
pub mod year;

use std::collections::BTreeMap;

use kod_core::codec::{Decode, DecodeError, Decoder, Encode, Encoder};
use kod_core::{ActorId, CommandError, Domain, IdAllocator, TickContext};

use atlas::{AtlasSnapshot, DataQuality, DatasetVersion, PolityId, PolityRecord};
use resolver::{DataStatus, ResolvedScenario, ScenarioMode, resolve};
use year::YearPicker;

kod_core::define_id!(
    /// Saved start-point bookmark.
    BookmarkId,
    "bm"
);

/// Maximum length of names (bookmarks, custom polities) in bytes.
pub const MAX_NAME_LEN: usize = 64;
/// First id of custom (sandbox) polities; atlas ids stay below.
pub const CUSTOM_POLITY_BASE: u32 = 1_000_000;

/// A saved start configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bookmark {
    /// Display name.
    pub name: String,
    /// Astronomical year.
    pub year: i32,
    /// Mode.
    pub mode: ScenarioMode,
    /// Selected polity.
    pub polity: Option<PolityId>,
}

/// Setup state of a new game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupState {
    /// Latest selectable year (host-supplied current calendar year).
    pub max_year: i32,
    /// Atlas the scenario is resolved against (fixed in saves).
    pub atlas: AtlasSnapshot,
    /// Selected astronomical year.
    pub year: i32,
    /// Selected mode.
    pub mode: ScenarioMode,
    /// Selected polity (player nation).
    pub polity: Option<PolityId>,
    /// Sandbox-only custom polities, valid in every year.
    pub custom_polities: BTreeMap<PolityId, String>,
    /// Bookmarks.
    pub bookmarks: BTreeMap<BookmarkId, Bookmark>,
    /// Bookmark id allocator.
    pub bookmark_ids: IdAllocator,
    /// Custom polity id allocator.
    pub custom_ids: IdAllocator,
    /// Set once the scenario is confirmed; afterwards setup is read-only.
    pub confirmed: bool,
}

impl SetupState {
    /// New setup for an atlas; starts at 100 BC, historical mode.
    pub fn new(atlas: AtlasSnapshot, current_year: i32) -> Self {
        let picker = YearPicker::new(current_year);
        Self {
            max_year: picker.range().1,
            atlas,
            year: picker.year(),
            mode: ScenarioMode::Historical,
            polity: None,
            custom_polities: BTreeMap::new(),
            bookmarks: BTreeMap::new(),
            bookmark_ids: IdAllocator::default(),
            custom_ids: IdAllocator::starting_at(CUSTOM_POLITY_BASE),
            confirmed: false,
        }
    }

    fn picker(&self) -> YearPicker {
        let mut p = YearPicker::new(self.max_year);
        p.select(self.year).expect("state year is always in range");
        p
    }

    /// Resolved scenario for the current selection (atlas + custom polities).
    pub fn resolved(&self) -> ResolvedScenario {
        let mut r = resolve(&self.atlas, self.year, self.mode);
        if self.mode.allows_custom_polities() {
            for (&id, name) in &self.custom_polities {
                r.polities.push(PolityRecord {
                    id,
                    name: name.clone(),
                    valid_from: year::MIN_START_YEAR,
                    valid_to: self.max_year + 1,
                    quality: DataQuality::Synthetic,
                });
            }
            if r.status == DataStatus::Missing && !r.polities.is_empty() {
                r.status = DataStatus::Complete;
            }
        }
        r
    }

    fn polity_selectable(&self, id: PolityId, year: i32, mode: ScenarioMode) -> bool {
        self.atlas.polity(id).is_some_and(|p| p.valid_in(year))
            || (mode.allows_custom_polities() && self.custom_polities.contains_key(&id))
    }

    fn name_ok(name: &str) -> Result<(), CommandError> {
        if name.trim().is_empty() || name.len() > MAX_NAME_LEN {
            Err(CommandError::InvalidArgument("name length".into()))
        } else {
            Ok(())
        }
    }
}

/// Setup commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupCommand {
    /// Select an astronomical year.
    SelectYear(i32),
    /// Move the year by a delta (clamped).
    StepYear(i32),
    /// Select the mode.
    SelectMode(ScenarioMode),
    /// Select the player polity (None clears).
    SelectPolity(Option<PolityId>),
    /// Create a custom polity (sandbox only).
    CreateCustomPolity(String),
    /// Bookmark the current selection.
    AddBookmark(String),
    /// Restore a bookmark.
    ApplyBookmark(BookmarkId),
    /// Delete a bookmark.
    RemoveBookmark(BookmarkId),
    /// Lock the setup and start the game.
    Confirm,
}

/// Setup events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupEvent {
    /// Year changed.
    YearChanged(i32),
    /// Mode changed.
    ModeChanged(ScenarioMode),
    /// Polity selection changed (also when cleared because it no longer exists).
    PolityChanged(Option<PolityId>),
    /// Custom polity created.
    CustomPolityCreated(PolityId),
    /// Bookmark added.
    BookmarkAdded(BookmarkId),
    /// Bookmark removed.
    BookmarkRemoved(BookmarkId),
    /// Setup confirmed.
    Confirmed,
}

/// Setup queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupQuery {
    /// Full view of the current selection.
    View,
}

/// Read model for the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupView {
    /// Resolved scenario.
    pub scenario: ResolvedScenario,
    /// Selected polity.
    pub polity: Option<PolityId>,
    /// Picker range.
    pub range: (i32, i32),
    /// Quick-select years.
    pub presets: [i32; 9],
    /// Bookmarks.
    pub bookmarks: Vec<(BookmarkId, Bookmark)>,
    /// Confirmed.
    pub confirmed: bool,
}

/// The F-01 setup domain.
#[derive(Debug, Clone, Copy)]
pub struct ScenarioSetup;

impl ScenarioSetup {
    /// Clears the polity if it is not selectable after a year/mode change.
    fn revalidate_polity(state: &mut SetupState, events: &mut Vec<SetupEvent>) {
        if let Some(id) = state.polity
            && !state.polity_selectable(id, state.year, state.mode)
        {
            state.polity = None;
            events.push(SetupEvent::PolityChanged(None));
        }
    }
}

impl Domain for ScenarioSetup {
    const SCHEMA_ID: &'static str = "kod.scenario.setup";
    const SCHEMA_VERSION: u32 = 1;

    type State = SetupState;
    type Command = SetupCommand;
    type Event = SetupEvent;
    type Query = SetupQuery;
    type Answer = SetupView;

    fn validate(
        state: &SetupState,
        _actor: ActorId,
        command: &SetupCommand,
    ) -> Result<(), CommandError> {
        if state.confirmed {
            return Err(CommandError::Conflict("scenario already confirmed".into()));
        }
        match command {
            SetupCommand::SelectYear(y) => state
                .picker()
                .check(*y)
                .map(|_| ())
                .map_err(|e| CommandError::InvalidArgument(format!("year {y}: {e:?}"))),
            SetupCommand::StepYear(_) | SetupCommand::SelectMode(_) => Ok(()),
            SetupCommand::SelectPolity(None) => Ok(()),
            SetupCommand::SelectPolity(Some(id)) => {
                if state.polity_selectable(*id, state.year, state.mode) {
                    Ok(())
                } else {
                    Err(CommandError::UnknownTarget(format!(
                        "{id} not available in {}",
                        year::label_de(state.year)
                    )))
                }
            }
            SetupCommand::CreateCustomPolity(name) => {
                if !state.mode.allows_custom_polities() {
                    return Err(CommandError::Unauthorized);
                }
                SetupState::name_ok(name)?;
                if state.custom_ids.peek() == u32::MAX {
                    return Err(CommandError::Conflict("custom polity ids exhausted".into()));
                }
                Ok(())
            }
            SetupCommand::AddBookmark(name) => {
                SetupState::name_ok(name)?;
                if state.bookmarks.values().any(|b| b.name == *name) {
                    return Err(CommandError::Conflict(format!("bookmark {name} exists")));
                }
                Ok(())
            }
            SetupCommand::ApplyBookmark(id) => {
                let b = state
                    .bookmarks
                    .get(id)
                    .ok_or_else(|| CommandError::UnknownTarget(id.to_string()))?;
                state
                    .picker()
                    .check(b.year)
                    .map_err(|_| CommandError::Conflict("bookmark year out of range".into()))?;
                Ok(())
            }
            SetupCommand::RemoveBookmark(id) => {
                if state.bookmarks.contains_key(id) {
                    Ok(())
                } else {
                    Err(CommandError::UnknownTarget(id.to_string()))
                }
            }
            SetupCommand::Confirm => {
                let polity = state
                    .polity
                    .ok_or_else(|| CommandError::Conflict("no polity selected".into()))?;
                if state.polity_selectable(polity, state.year, state.mode) {
                    Ok(())
                } else {
                    Err(CommandError::Conflict(
                        "selected polity not available".into(),
                    ))
                }
            }
        }
    }

    fn apply(
        state: &mut SetupState,
        _ctx: &mut TickContext<'_>,
        _actor: ActorId,
        command: &SetupCommand,
        events: &mut Vec<SetupEvent>,
    ) {
        match command {
            SetupCommand::SelectYear(y) => {
                state.year = *y;
                events.push(SetupEvent::YearChanged(*y));
                Self::revalidate_polity(state, events);
            }
            SetupCommand::StepYear(delta) => {
                let mut p = state.picker();
                p.step(*delta);
                if p.year() != state.year {
                    state.year = p.year();
                    events.push(SetupEvent::YearChanged(state.year));
                    Self::revalidate_polity(state, events);
                }
            }
            SetupCommand::SelectMode(mode) => {
                state.mode = *mode;
                events.push(SetupEvent::ModeChanged(*mode));
                Self::revalidate_polity(state, events);
            }
            SetupCommand::SelectPolity(p) => {
                state.polity = *p;
                events.push(SetupEvent::PolityChanged(*p));
            }
            SetupCommand::CreateCustomPolity(name) => {
                let id = PolityId::new(state.custom_ids.allocate().expect("validated"));
                state.custom_polities.insert(id, name.clone());
                events.push(SetupEvent::CustomPolityCreated(id));
            }
            SetupCommand::AddBookmark(name) => {
                let id =
                    BookmarkId::new(state.bookmark_ids.allocate().expect("u32 bookmark space"));
                state.bookmarks.insert(
                    id,
                    Bookmark {
                        name: name.clone(),
                        year: state.year,
                        mode: state.mode,
                        polity: state.polity,
                    },
                );
                events.push(SetupEvent::BookmarkAdded(id));
            }
            SetupCommand::ApplyBookmark(id) => {
                let b = state.bookmarks[id].clone();
                state.year = b.year;
                state.mode = b.mode;
                state.polity = b.polity;
                events.push(SetupEvent::YearChanged(b.year));
                events.push(SetupEvent::ModeChanged(b.mode));
                events.push(SetupEvent::PolityChanged(b.polity));
                Self::revalidate_polity(state, events);
            }
            SetupCommand::RemoveBookmark(id) => {
                state.bookmarks.remove(id);
                events.push(SetupEvent::BookmarkRemoved(*id));
            }
            SetupCommand::Confirm => {
                state.confirmed = true;
                events.push(SetupEvent::Confirmed);
            }
        }
    }

    fn query(state: &SetupState, _query: &SetupQuery) -> SetupView {
        let picker = state.picker();
        SetupView {
            scenario: state.resolved(),
            polity: state.polity,
            range: picker.range(),
            presets: picker.presets(),
            bookmarks: state
                .bookmarks
                .iter()
                .map(|(k, v)| (*k, v.clone()))
                .collect(),
            confirmed: state.confirmed,
        }
    }
}

/// Checks that a loaded setup still matches the atlas available at runtime;
/// a save stays bound to the dataset it was created with.
pub fn check_dataset(state: &SetupState, available: &DatasetVersion) -> Result<(), String> {
    if state.atlas.dataset == *available {
        Ok(())
    } else {
        Err(format!(
            "save uses dataset {} {} ({}), runtime offers {} {} ({})",
            state.atlas.dataset.id,
            state.atlas.dataset.version,
            state.atlas.dataset.checksum,
            available.id,
            available.version,
            available.checksum
        ))
    }
}

// ---- canonical encoding -------------------------------------------------

impl Encode for ScenarioMode {
    fn encode(&self, enc: &mut Encoder) {
        enc.put(&(*self as u8));
    }
}

impl Decode for ScenarioMode {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        let tag: u8 = dec.get()?;
        Self::from_u8(tag).ok_or(DecodeError::InvalidTag {
            ty: "ScenarioMode",
            tag: tag.into(),
        })
    }
}

impl Encode for Bookmark {
    fn encode(&self, enc: &mut Encoder) {
        enc.put(&self.name);
        enc.put(&self.year);
        enc.put(&self.mode);
        enc.put(&self.polity);
    }
}

impl Decode for Bookmark {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            name: dec.get()?,
            year: dec.get()?,
            mode: dec.get()?,
            polity: dec.get()?,
        })
    }
}

impl Encode for SetupState {
    fn encode(&self, enc: &mut Encoder) {
        enc.put(&self.max_year);
        enc.put(&self.atlas);
        enc.put(&self.year);
        enc.put(&self.mode);
        enc.put(&self.polity);
        enc.put(&self.custom_polities);
        enc.put(&self.bookmarks);
        enc.put(&self.bookmark_ids);
        enc.put(&self.custom_ids);
        enc.put(&self.confirmed);
    }
}

impl Decode for SetupState {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        let state = Self {
            max_year: dec.get()?,
            atlas: dec.get()?,
            year: dec.get()?,
            mode: dec.get()?,
            polity: dec.get()?,
            custom_polities: dec.get()?,
            bookmarks: dec.get()?,
            bookmark_ids: dec.get()?,
            custom_ids: dec.get()?,
            confirmed: dec.get()?,
        };
        if YearPicker::new(state.max_year).check(state.year).is_err() {
            return Err(DecodeError::Invalid("setup year out of range"));
        }
        Ok(state)
    }
}

impl Encode for SetupCommand {
    fn encode(&self, enc: &mut Encoder) {
        match self {
            Self::SelectYear(y) => {
                enc.put(&0u8);
                enc.put(y);
            }
            Self::StepYear(d) => {
                enc.put(&1u8);
                enc.put(d);
            }
            Self::SelectMode(m) => {
                enc.put(&2u8);
                enc.put(m);
            }
            Self::SelectPolity(p) => {
                enc.put(&3u8);
                enc.put(p);
            }
            Self::CreateCustomPolity(n) => {
                enc.put(&4u8);
                enc.put(n);
            }
            Self::AddBookmark(n) => {
                enc.put(&5u8);
                enc.put(n);
            }
            Self::ApplyBookmark(id) => {
                enc.put(&6u8);
                enc.put(id);
            }
            Self::RemoveBookmark(id) => {
                enc.put(&7u8);
                enc.put(id);
            }
            Self::Confirm => enc.put(&8u8),
        }
    }
}

impl Decode for SetupCommand {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(match dec.get::<u8>()? {
            0 => Self::SelectYear(dec.get()?),
            1 => Self::StepYear(dec.get()?),
            2 => Self::SelectMode(dec.get()?),
            3 => Self::SelectPolity(dec.get()?),
            4 => Self::CreateCustomPolity(dec.get()?),
            5 => Self::AddBookmark(dec.get()?),
            6 => Self::ApplyBookmark(dec.get()?),
            7 => Self::RemoveBookmark(dec.get()?),
            8 => Self::Confirm,
            tag => {
                return Err(DecodeError::InvalidTag {
                    ty: "SetupCommand",
                    tag: tag.into(),
                });
            }
        })
    }
}

impl Encode for SetupEvent {
    fn encode(&self, enc: &mut Encoder) {
        match self {
            Self::YearChanged(y) => {
                enc.put(&0u8);
                enc.put(y);
            }
            Self::ModeChanged(m) => {
                enc.put(&1u8);
                enc.put(m);
            }
            Self::PolityChanged(p) => {
                enc.put(&2u8);
                enc.put(p);
            }
            Self::CustomPolityCreated(id) => {
                enc.put(&3u8);
                enc.put(id);
            }
            Self::BookmarkAdded(id) => {
                enc.put(&4u8);
                enc.put(id);
            }
            Self::BookmarkRemoved(id) => {
                enc.put(&5u8);
                enc.put(id);
            }
            Self::Confirmed => enc.put(&6u8),
        }
    }
}

impl Decode for SetupEvent {
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(match dec.get::<u8>()? {
            0 => Self::YearChanged(dec.get()?),
            1 => Self::ModeChanged(dec.get()?),
            2 => Self::PolityChanged(dec.get()?),
            3 => Self::CustomPolityCreated(dec.get()?),
            4 => Self::BookmarkAdded(dec.get()?),
            5 => Self::BookmarkRemoved(dec.get()?),
            6 => Self::Confirmed,
            tag => {
                return Err(DecodeError::InvalidTag {
                    ty: "SetupEvent",
                    tag: tag.into(),
                });
            }
        })
    }
}
