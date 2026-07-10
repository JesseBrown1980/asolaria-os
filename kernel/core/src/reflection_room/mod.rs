//! Event-sourced reflection-room core.
//!
//! This module stores committed metadata for explicit agent artifacts, never hidden
//! chain-of-thought. Bodies remain in the HBP store and are referenced by full SHA-256 plus
//! HBI byte offset/length. A prepared event is invisible until the single room writer commits
//! it. The active metadata window is bounded at 2,000 events; five sealed windows form a 10,000
//! checkpoint and five checkpoints form a 50,000 super-gulp.
//!
//! This is a pure `no_std` kernel contract. It does not launch a model, register with the live
//! fabric, persist disk bytes, or claim the historical GNN/white-room runtime has been ported.

use alloc::vec::Vec;
use sha2::{Digest, Sha256};

/// Maximum number of committed event envelopes held in the active room window.
pub const ACTIVE_ENVELOPE_WINDOW: u32 = 2_000;
/// Number of 2K waves accumulated into one 10K checkpoint.
pub const GULPS_PER_CHECKPOINT: u32 = 5;
/// Number of 10K checkpoints accumulated into one 50K super-gulp.
pub const CHECKPOINTS_PER_SUPER_GULP: u32 = 5;
/// Event count represented by one checkpoint.
pub const CHECKPOINT_ENVELOPES: u32 = ACTIVE_ENVELOPE_WINDOW * GULPS_PER_CHECKPOINT;
/// Event count represented by one super-gulp.
pub const SUPER_GULP_ENVELOPES: u32 = CHECKPOINT_ENVELOPES * CHECKPOINTS_PER_SUPER_GULP;
/// Current selector floor. Older 35D/47D/49D forms are bridge strata, not this room ceiling.
pub const MIN_SELECTOR_DIMS: u16 = 60;
/// All-zero SHA-256 used only as the genesis/absent-parent marker.
pub const GENESIS_SHA256: [u8; 32] = [0u8; 32];

/// Logical channels in one reflection room.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomEventKind {
    TaskIn = 1,
    WorkNote = 2,
    ToolResult = 3,
    TranslatedNote = 4,
    SelfReflection = 5,
    PeerReview = 6,
    SupervisorControl = 7,
    FinalResult = 8,
}

impl RoomEventKind {
    fn requires_parent(self) -> bool {
        matches!(
            self,
            Self::TranslatedNote | Self::SelfReflection | Self::PeerReview | Self::FinalResult
        )
    }
}

/// Evidence/authority state carried by the event; placement never implies authority.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityState {
    Unverified = 0,
    Measured = 1,
    Canon = 2,
    Held = 3,
}

/// Recall disclosure projection used across colonies. The six fabric tiers map to 0/5/9.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecallAccessLevel {
    Public = 0,
    Federation = 5,
    OwnerPrivate = 9,
}

/// PII classification carried into the room gate. Unknown or consented PII can never enter L0/L5.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PiiState {
    None = 0,
    KeyedConsented = 1,
    Unknown = 2,
}

/// Direct HBI seek pointer to an externally persisted HBP body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BodyRef {
    pub offset: u64,
    pub length: u32,
    pub sha256: [u8; 32],
}

/// Candidate explicit-artifact event. Full PIDs, verbs, glyph/language axes, and the complete
/// 60D+ selector remain content-addressed by SHA; 8-byte handles are routing aids only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventDraft {
    pub kind: RoomEventKind,
    pub source_handle8: [u8; 8],
    pub source_pid_sha256: [u8; 32],
    pub target_handle8: [u8; 8],
    pub target_pid_sha256: [u8; 32],
    pub verb_sha256: [u8; 32],
    pub timestamp_ns: u64,
    pub parent_sha256: [u8; 32],
    pub body: BodyRef,
    pub selector_dims: u16,
    pub selector_tuple_sha256: [u8; 32],
    pub authority: AuthorityState,
    pub access_level: RecallAccessLevel,
    pub pii_state: PiiState,
    pub owner_pid_sha256: [u8; 32],
    pub idempotency_key: [u8; 32],
}

/// Fully committed event identity. `full_sha256` owns durable identity; handles only route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommittedEvent {
    pub sequence: u64,
    pub room_handle8: [u8; 8],
    pub room_id_sha256: [u8; 32],
    pub previous_sha256: [u8; 32],
    pub full_sha256: [u8; 32],
    pub draft: EventDraft,
}

/// Prepared-but-not-visible event. Only [`RoomWriter::commit`] can publish it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparedEvent {
    event: CommittedEvent,
}

impl PreparedEvent {
    pub fn full_sha256(&self) -> [u8; 32] {
        self.event.full_sha256
    }

    pub fn sequence(&self) -> u64 {
        self.event.sequence
    }
}

/// Read-only committed head. Prepared events never appear here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomHead {
    pub room_handle8: [u8; 8],
    pub room_id_sha256: [u8; 32],
    pub next_sequence: u64,
    pub head_sha256: [u8; 32],
    pub active_events: u32,
    pub total_events: u64,
    pub total_gulps: u64,
    pub total_checkpoints: u64,
    pub total_super_gulps: u64,
}

/// Result of a commit attempt. Duplicate idempotency keys resolve to the original commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitOutcome {
    Committed(CommittedEvent),
    Duplicate(CommittedEvent),
}

/// Hierarchical product emitted when one full active window is sealed.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GulpTier {
    Gulp2k = 1,
    Checkpoint10k = 2,
    SuperGulp50k = 3,
}

/// Proof returned before active metadata may be reused for the next wave.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GulpSeal {
    pub tier: GulpTier,
    pub sealed_events: u32,
    pub cumulative_events: u64,
    pub gulp_index: u64,
    pub checkpoint_index: u64,
    pub super_gulp_index: u64,
    pub room_head_sha256: [u8; 32],
    pub distilled_product_sha256: [u8; 32],
    pub seal_sha256: [u8; 32],
}

/// Deterministic room failures; every path is explicit and fail-closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomError {
    RoomIdentityMissing,
    BodyMissing,
    BodyHashMissing,
    IdempotencyKeyMissing,
    SelectorTupleMissing,
    SelectorDimsTooSmall { observed: u16 },
    PiiRequiresOwnerPrivate,
    OwnerIdentityMissing,
    ParentMissing,
    ParentNotCommitted,
    ActiveWindowFull,
    StalePreparedEvent,
    SequenceOverflow,
    GulpNotReady { active_events: u32 },
    DistilledProductMissing,
}

/// Single ordered commit writer for one logical room.
pub struct RoomWriter {
    room_handle8: [u8; 8],
    room_id_sha256: [u8; 32],
    next_sequence: u64,
    head_sha256: [u8; 32],
    recent: Vec<CommittedEvent>,
    total_events: u64,
    total_gulps: u64,
}

impl RoomWriter {
    /// Create a room. The full room SHA owns identity; the 8-byte handle only routes.
    pub fn new(room_handle8: [u8; 8], room_id_sha256: [u8; 32]) -> Result<Self, RoomError> {
        if is_zero_sha(&room_id_sha256) {
            return Err(RoomError::RoomIdentityMissing);
        }
        Ok(Self {
            room_handle8,
            room_id_sha256,
            next_sequence: 1,
            head_sha256: GENESIS_SHA256,
            recent: Vec::with_capacity(ACTIVE_ENVELOPE_WINDOW as usize),
            total_events: 0,
            total_gulps: 0,
        })
    }

    /// Return only committed state; a prepared write cannot leak through this boundary.
    pub fn read_committed_head(&self) -> RoomHead {
        RoomHead {
            room_handle8: self.room_handle8,
            room_id_sha256: self.room_id_sha256,
            next_sequence: self.next_sequence,
            head_sha256: self.head_sha256,
            active_events: self.recent.len() as u32,
            total_events: self.total_events,
            total_gulps: self.total_gulps,
            total_checkpoints: self.total_gulps / GULPS_PER_CHECKPOINT as u64,
            total_super_gulps: self.total_gulps
                / (GULPS_PER_CHECKPOINT * CHECKPOINTS_PER_SUPER_GULP) as u64,
        }
    }

    /// Prepare canonical bytes against the current committed head without publishing them.
    pub fn prepare(&self, draft: EventDraft) -> Result<PreparedEvent, RoomError> {
        validate_draft(self, &draft)?;
        let duplicate = self
            .recent
            .iter()
            .any(|event| event.draft.idempotency_key == draft.idempotency_key);
        if self.recent.len() >= ACTIVE_ENVELOPE_WINDOW as usize && !duplicate {
            return Err(RoomError::ActiveWindowFull);
        }
        let sequence = self
            .next_sequence
            .checked_add(0)
            .ok_or(RoomError::SequenceOverflow)?;
        if sequence == u64::MAX {
            return Err(RoomError::SequenceOverflow);
        }
        let full_sha256 = event_sha256(
            &self.room_id_sha256,
            &self.room_handle8,
            sequence,
            &self.head_sha256,
            &draft,
        );
        Ok(PreparedEvent {
            event: CommittedEvent {
                sequence,
                room_handle8: self.room_handle8,
                room_id_sha256: self.room_id_sha256,
                previous_sha256: self.head_sha256,
                full_sha256,
                draft,
            },
        })
    }

    /// Atomically publish a prepared event. A competing stale preparation is rejected.
    pub fn commit(&mut self, prepared: PreparedEvent) -> Result<CommitOutcome, RoomError> {
        if let Some(existing) = self
            .recent
            .iter()
            .find(|event| event.draft.idempotency_key == prepared.event.draft.idempotency_key)
        {
            return Ok(CommitOutcome::Duplicate(*existing));
        }
        if self.recent.len() >= ACTIVE_ENVELOPE_WINDOW as usize {
            return Err(RoomError::ActiveWindowFull);
        }
        if prepared.event.sequence != self.next_sequence
            || prepared.event.previous_sha256 != self.head_sha256
        {
            return Err(RoomError::StalePreparedEvent);
        }
        let committed = prepared.event;
        self.recent.push(committed);
        self.head_sha256 = committed.full_sha256;
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(RoomError::SequenceOverflow)?;
        self.total_events = self
            .total_events
            .checked_add(1)
            .ok_or(RoomError::SequenceOverflow)?;
        Ok(CommitOutcome::Committed(committed))
    }

    /// Seal exactly one 2K active window after its distilled product has been persisted.
    /// The caller-provided SHA is the content address of that committed product, not a score.
    pub fn seal_gulp(&mut self, distilled_product_sha256: [u8; 32]) -> Result<GulpSeal, RoomError> {
        let active = self.recent.len() as u32;
        if active != ACTIVE_ENVELOPE_WINDOW {
            return Err(RoomError::GulpNotReady {
                active_events: active,
            });
        }
        if is_zero_sha(&distilled_product_sha256) {
            return Err(RoomError::DistilledProductMissing);
        }
        let gulp_index = self
            .total_gulps
            .checked_add(1)
            .ok_or(RoomError::SequenceOverflow)?;
        let checkpoint_index = gulp_index / GULPS_PER_CHECKPOINT as u64;
        let super_divisor = (GULPS_PER_CHECKPOINT * CHECKPOINTS_PER_SUPER_GULP) as u64;
        let super_gulp_index = gulp_index / super_divisor;
        let tier = if gulp_index % super_divisor == 0 {
            GulpTier::SuperGulp50k
        } else if gulp_index % GULPS_PER_CHECKPOINT as u64 == 0 {
            GulpTier::Checkpoint10k
        } else {
            GulpTier::Gulp2k
        };
        let seal_sha256 = gulp_seal_sha256(
            &self.room_id_sha256,
            &self.head_sha256,
            self.total_events,
            gulp_index,
            tier,
            &distilled_product_sha256,
        );
        let seal = GulpSeal {
            tier,
            sealed_events: ACTIVE_ENVELOPE_WINDOW,
            cumulative_events: self.total_events,
            gulp_index,
            checkpoint_index,
            super_gulp_index,
            room_head_sha256: self.head_sha256,
            distilled_product_sha256,
            seal_sha256,
        };
        self.recent.clear();
        self.total_gulps = gulp_index;
        Ok(seal)
    }
}

fn validate_draft(writer: &RoomWriter, draft: &EventDraft) -> Result<(), RoomError> {
    if draft.body.length == 0 {
        return Err(RoomError::BodyMissing);
    }
    if is_zero_sha(&draft.body.sha256) {
        return Err(RoomError::BodyHashMissing);
    }
    if is_zero_sha(&draft.idempotency_key) {
        return Err(RoomError::IdempotencyKeyMissing);
    }
    if is_zero_sha(&draft.selector_tuple_sha256) {
        return Err(RoomError::SelectorTupleMissing);
    }
    if draft.selector_dims < MIN_SELECTOR_DIMS {
        return Err(RoomError::SelectorDimsTooSmall {
            observed: draft.selector_dims,
        });
    }
    if draft.access_level != RecallAccessLevel::OwnerPrivate && draft.pii_state != PiiState::None {
        return Err(RoomError::PiiRequiresOwnerPrivate);
    }
    if draft.access_level == RecallAccessLevel::OwnerPrivate && is_zero_sha(&draft.owner_pid_sha256)
    {
        return Err(RoomError::OwnerIdentityMissing);
    }
    if draft.kind.requires_parent() {
        if is_zero_sha(&draft.parent_sha256) {
            return Err(RoomError::ParentMissing);
        }
        let parent_known = draft.parent_sha256 == writer.head_sha256
            || writer
                .recent
                .iter()
                .any(|event| event.full_sha256 == draft.parent_sha256);
        if !parent_known {
            return Err(RoomError::ParentNotCommitted);
        }
    }
    Ok(())
}

fn event_sha256(
    room_id_sha256: &[u8; 32],
    room_handle8: &[u8; 8],
    sequence: u64,
    previous_sha256: &[u8; 32],
    draft: &EventDraft,
) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"ASOLARIA-REFLECTION-ROOM-EVENT-V1");
    h.update(room_id_sha256);
    h.update(room_handle8);
    h.update(sequence.to_be_bytes());
    h.update(previous_sha256);
    h.update([draft.kind as u8]);
    h.update(draft.source_handle8);
    h.update(draft.source_pid_sha256);
    h.update(draft.target_handle8);
    h.update(draft.target_pid_sha256);
    h.update(draft.verb_sha256);
    h.update(draft.timestamp_ns.to_be_bytes());
    h.update(draft.parent_sha256);
    h.update(draft.body.offset.to_be_bytes());
    h.update(draft.body.length.to_be_bytes());
    h.update(draft.body.sha256);
    h.update(draft.selector_dims.to_be_bytes());
    h.update(draft.selector_tuple_sha256);
    h.update([draft.authority as u8]);
    h.update([draft.access_level as u8]);
    h.update([draft.pii_state as u8]);
    h.update(draft.owner_pid_sha256);
    h.update(draft.idempotency_key);
    digest_array(h)
}

fn gulp_seal_sha256(
    room_id_sha256: &[u8; 32],
    head_sha256: &[u8; 32],
    total_events: u64,
    gulp_index: u64,
    tier: GulpTier,
    distilled_product_sha256: &[u8; 32],
) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"ASOLARIA-REFLECTION-ROOM-GULP-SEAL-V1");
    h.update(room_id_sha256);
    h.update(head_sha256);
    h.update(total_events.to_be_bytes());
    h.update(gulp_index.to_be_bytes());
    h.update([tier as u8]);
    h.update(distilled_product_sha256);
    digest_array(h)
}

fn digest_array(hasher: Sha256) -> [u8; 32] {
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

fn is_zero_sha(value: &[u8; 32]) -> bool {
    value.iter().all(|byte| *byte == 0)
}

/// Deterministic novelty/saturation result. Similarity is supplied by an external semantic
/// evaluator; the kernel only enforces the bounded halt rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoveltyDecision {
    Continue { novelty_bps: u16 },
    LowNovelty { novelty_bps: u16, run: u8 },
    Saturated { novelty_bps: u16, run: u8 },
}

/// Configuration failures for the deterministic novelty gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoveltyError {
    EpsilonOutOfRange,
    ZeroSaturationLimit,
    SimilarityOutOfRange,
}

/// Stops reflection echo after K consecutive committed low-novelty observations.
pub struct NoveltyGate {
    epsilon_bps: u16,
    saturation_limit: u8,
    low_novelty_run: u8,
}

impl NoveltyGate {
    pub fn new(epsilon_bps: u16, saturation_limit: u8) -> Result<Self, NoveltyError> {
        if epsilon_bps > 10_000 {
            return Err(NoveltyError::EpsilonOutOfRange);
        }
        if saturation_limit == 0 {
            return Err(NoveltyError::ZeroSaturationLimit);
        }
        Ok(Self {
            epsilon_bps,
            saturation_limit,
            low_novelty_run: 0,
        })
    }

    pub fn observe_similarity(
        &mut self,
        similarity_bps: u16,
    ) -> Result<NoveltyDecision, NoveltyError> {
        if similarity_bps > 10_000 {
            return Err(NoveltyError::SimilarityOutOfRange);
        }
        let novelty_bps = 10_000 - similarity_bps;
        if novelty_bps < self.epsilon_bps {
            self.low_novelty_run = self.low_novelty_run.saturating_add(1);
            if self.low_novelty_run >= self.saturation_limit {
                Ok(NoveltyDecision::Saturated {
                    novelty_bps,
                    run: self.low_novelty_run,
                })
            } else {
                Ok(NoveltyDecision::LowNovelty {
                    novelty_bps,
                    run: self.low_novelty_run,
                })
            }
        } else {
            self.low_novelty_run = 0;
            Ok(NoveltyDecision::Continue { novelty_bps })
        }
    }
}

/// Deterministic result vocabulary used after a committed reflection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReflectionGateDecision {
    Continue,
    Correct,
    Hold,
    SendToReview,
    Complete,
    Saturated,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(parts: &[&[u8]]) -> [u8; 32] {
        let mut h = Sha256::new();
        for part in parts {
            h.update(part);
        }
        digest_array(h)
    }

    fn room() -> RoomWriter {
        RoomWriter::new(*b"room0001", hash(&[b"full-room-pid-60d"])).unwrap()
    }

    fn draft(kind: RoomEventKind, nonce: u64, parent_sha256: [u8; 32]) -> EventDraft {
        let n = nonce.to_be_bytes();
        EventDraft {
            kind,
            source_handle8: *b"source01",
            source_pid_sha256: hash(&[b"source-pid", &n]),
            target_handle8: *b"target01",
            target_pid_sha256: hash(&[b"target-pid", &n]),
            verb_sha256: hash(&[b"verb", &n]),
            timestamp_ns: nonce + 1,
            parent_sha256,
            body: BodyRef {
                offset: nonce * 100,
                length: 64,
                sha256: hash(&[b"body", &n]),
            },
            selector_dims: 60,
            selector_tuple_sha256: hash(&[b"selector-60d", &n]),
            authority: AuthorityState::Measured,
            access_level: RecallAccessLevel::Federation,
            pii_state: PiiState::None,
            owner_pid_sha256: GENESIS_SHA256,
            idempotency_key: hash(&[b"idempotency", &n]),
        }
    }

    #[test]
    fn hierarchy_is_2k_10k_50k() {
        assert_eq!(ACTIVE_ENVELOPE_WINDOW, 2_000);
        assert_eq!(CHECKPOINT_ENVELOPES, 10_000);
        assert_eq!(SUPER_GULP_ENVELOPES, 50_000);
    }

    #[test]
    fn prepared_event_is_invisible_until_commit() {
        let mut writer = room();
        let before = writer.read_committed_head();
        let prepared = writer
            .prepare(draft(RoomEventKind::TaskIn, 1, GENESIS_SHA256))
            .unwrap();
        assert_eq!(writer.read_committed_head(), before);
        let committed = match writer.commit(prepared).unwrap() {
            CommitOutcome::Committed(event) => event,
            CommitOutcome::Duplicate(_) => panic!("first event cannot be duplicate"),
        };
        let after = writer.read_committed_head();
        assert_eq!(after.active_events, 1);
        assert_eq!(after.total_events, 1);
        assert_eq!(after.head_sha256, committed.full_sha256);
    }

    #[test]
    fn competing_preparation_is_rejected_after_head_moves() {
        let mut writer = room();
        let first = writer
            .prepare(draft(RoomEventKind::TaskIn, 1, GENESIS_SHA256))
            .unwrap();
        let stale = writer
            .prepare(draft(RoomEventKind::TaskIn, 2, GENESIS_SHA256))
            .unwrap();
        writer.commit(first).unwrap();
        assert_eq!(writer.commit(stale), Err(RoomError::StalePreparedEvent));
    }

    #[test]
    fn duplicate_idempotency_key_returns_original_commit() {
        let mut writer = room();
        let original_draft = draft(RoomEventKind::TaskIn, 1, GENESIS_SHA256);
        let first = writer.prepare(original_draft).unwrap();
        let original = match writer.commit(first).unwrap() {
            CommitOutcome::Committed(event) => event,
            CommitOutcome::Duplicate(_) => panic!("first event cannot be duplicate"),
        };
        let replay = writer.prepare(original_draft).unwrap();
        assert_eq!(
            writer.commit(replay),
            Ok(CommitOutcome::Duplicate(original))
        );
        assert_eq!(writer.read_committed_head().total_events, 1);
    }

    #[test]
    fn reflection_requires_a_committed_parent() {
        let mut writer = room();
        let missing = draft(RoomEventKind::SelfReflection, 2, hash(&[b"missing"]));
        assert!(matches!(
            writer.prepare(missing),
            Err(RoomError::ParentNotCommitted)
        ));

        let work = writer
            .prepare(draft(RoomEventKind::WorkNote, 1, GENESIS_SHA256))
            .unwrap();
        let work = match writer.commit(work).unwrap() {
            CommitOutcome::Committed(event) => event,
            CommitOutcome::Duplicate(_) => unreachable!(),
        };
        let reflection = writer
            .prepare(draft(RoomEventKind::SelfReflection, 2, work.full_sha256))
            .unwrap();
        assert!(matches!(
            writer.commit(reflection),
            Ok(CommitOutcome::Committed(_))
        ));
    }

    #[test]
    fn full_pid_hashes_not_short_handles_own_event_identity() {
        let writer = room();
        let one = writer
            .prepare(draft(RoomEventKind::TaskIn, 1, GENESIS_SHA256))
            .unwrap();
        let mut changed = draft(RoomEventKind::TaskIn, 1, GENESIS_SHA256);
        changed.source_pid_sha256 = hash(&[b"different-full-source-pid"]);
        let two = writer.prepare(changed).unwrap();
        assert_ne!(one.full_sha256(), two.full_sha256());
    }

    #[test]
    fn selector_floor_is_60d() {
        let writer = room();
        let mut old_bridge = draft(RoomEventKind::TaskIn, 1, GENESIS_SHA256);
        old_bridge.selector_dims = 49;
        assert_eq!(
            writer.prepare(old_bridge),
            Err(RoomError::SelectorDimsTooSmall { observed: 49 })
        );
    }

    #[test]
    fn pii_is_fail_closed_to_owner_private_l9() {
        let writer = room();
        let mut public = draft(RoomEventKind::TaskIn, 1, GENESIS_SHA256);
        public.access_level = RecallAccessLevel::Public;
        public.pii_state = PiiState::KeyedConsented;
        assert_eq!(
            writer.prepare(public),
            Err(RoomError::PiiRequiresOwnerPrivate)
        );

        let mut unknown_federation = draft(RoomEventKind::TaskIn, 2, GENESIS_SHA256);
        unknown_federation.pii_state = PiiState::Unknown;
        assert_eq!(
            writer.prepare(unknown_federation),
            Err(RoomError::PiiRequiresOwnerPrivate)
        );

        let mut owner_private = draft(RoomEventKind::TaskIn, 3, GENESIS_SHA256);
        owner_private.access_level = RecallAccessLevel::OwnerPrivate;
        owner_private.pii_state = PiiState::KeyedConsented;
        assert_eq!(
            writer.prepare(owner_private),
            Err(RoomError::OwnerIdentityMissing)
        );
        owner_private.owner_pid_sha256 = hash(&[b"consenting-owner-human-pid"]);
        assert!(writer.prepare(owner_private).is_ok());
    }

    #[test]
    fn twenty_five_windows_form_one_50k_super_gulp() {
        let mut writer = room();
        let mut last_seal = None;
        for wave in 0u64..25 {
            for slot in 0u64..ACTIVE_ENVELOPE_WINDOW as u64 {
                let nonce = wave * ACTIVE_ENVELOPE_WINDOW as u64 + slot + 1;
                let prepared = writer
                    .prepare(draft(RoomEventKind::WorkNote, nonce, GENESIS_SHA256))
                    .unwrap();
                writer.commit(prepared).unwrap();
            }
            let product = hash(&[b"distilled-product", &wave.to_be_bytes()]);
            last_seal = Some(writer.seal_gulp(product).unwrap());
            assert_eq!(writer.read_committed_head().active_events, 0);
        }
        let seal = last_seal.unwrap();
        assert_eq!(seal.tier, GulpTier::SuperGulp50k);
        assert_eq!(seal.cumulative_events, 50_000);
        assert_eq!(seal.gulp_index, 25);
        assert_eq!(seal.checkpoint_index, 5);
        assert_eq!(seal.super_gulp_index, 1);
        let head = writer.read_committed_head();
        assert_eq!(head.total_events, 50_000);
        assert_eq!(head.total_checkpoints, 5);
        assert_eq!(head.total_super_gulps, 1);
    }

    #[test]
    fn gulp_refuses_to_release_without_full_window_and_product() {
        let mut writer = room();
        assert_eq!(
            writer.seal_gulp(hash(&[b"product"])),
            Err(RoomError::GulpNotReady { active_events: 0 })
        );
        for nonce in 1..=ACTIVE_ENVELOPE_WINDOW as u64 {
            let prepared = writer
                .prepare(draft(RoomEventKind::WorkNote, nonce, GENESIS_SHA256))
                .unwrap();
            writer.commit(prepared).unwrap();
        }
        assert_eq!(
            writer.seal_gulp(GENESIS_SHA256),
            Err(RoomError::DistilledProductMissing)
        );
        assert_eq!(
            writer.read_committed_head().active_events,
            ACTIVE_ENVELOPE_WINDOW
        );
    }

    #[test]
    fn novelty_gate_saturates_and_resets() {
        let mut gate = NoveltyGate::new(500, 3).unwrap();
        assert!(matches!(
            gate.observe_similarity(9_600).unwrap(),
            NoveltyDecision::LowNovelty { run: 1, .. }
        ));
        assert!(matches!(
            gate.observe_similarity(9_700).unwrap(),
            NoveltyDecision::LowNovelty { run: 2, .. }
        ));
        assert!(matches!(
            gate.observe_similarity(9_800).unwrap(),
            NoveltyDecision::Saturated { run: 3, .. }
        ));
        assert!(matches!(
            gate.observe_similarity(8_000).unwrap(),
            NoveltyDecision::Continue { .. }
        ));
        assert!(matches!(
            gate.observe_similarity(9_900).unwrap(),
            NoveltyDecision::LowNovelty { run: 1, .. }
        ));
    }
}
