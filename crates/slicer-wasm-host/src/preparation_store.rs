//! Print-scoped opaque storage. Handles never retain payload copies.

use crate::preparation::{FrozenPreparationBinding, PreparationFailure};
use slicer_schema::preparation::PreparationTransportError as Transport;
use std::{
    collections::BTreeMap,
    fmt,
    sync::{Arc, Mutex, MutexGuard, TryLockError, Weak},
};

/// Redacted accounting snapshot; pieces contain names and lengths only.
#[derive(Clone, Debug)]
pub struct PreparedPlanMetadata {
    module_id: String,
    phase: &'static str,
    ready: bool,
    pieces: Vec<(String, u64)>,
    retained_payload_bytes: u64,
    retained_name_bytes: u64,
    staged_payload_bytes: u64,
    staged_name_bytes: u64,
    put_calls: u64,
    read_calls: u64,
    requested_read_bytes: u64,
    returned_read_bytes: u64,
    prepare_invocations: u64,
}

impl PreparedPlanMetadata {
    /// Return the owning module identity.
    pub fn module_id(&self) -> &str {
        &self.module_id
    }
    /// Return the current preparation phase.
    pub fn phase(&self) -> &str {
        self.phase
    }
    /// Report whether publication completed.
    pub fn ready(&self) -> bool {
        self.ready
    }
    /// Return published piece names and lengths.
    pub fn pieces(&self) -> &[(String, u64)] {
        &self.pieces
    }
    /// Return retained payload accounting.
    pub fn retained_payload_bytes(&self) -> u64 {
        self.retained_payload_bytes
    }
    /// Return retained name accounting.
    pub fn retained_name_bytes(&self) -> u64 {
        self.retained_name_bytes
    }
    /// Return staged payload accounting.
    pub fn staged_payload_bytes(&self) -> u64 {
        self.staged_payload_bytes
    }
    /// Return staged name accounting.
    pub fn staged_name_bytes(&self) -> u64 {
        self.staged_name_bytes
    }
    /// Return the number of accepted writes.
    pub fn put_calls(&self) -> u64 {
        self.put_calls
    }
    /// Return the number of completed reads.
    pub fn read_calls(&self) -> u64 {
        self.read_calls
    }
    /// Return cumulative requested read bytes.
    pub fn requested_read_bytes(&self) -> u64 {
        self.requested_read_bytes
    }
    /// Return cumulative returned read bytes.
    pub fn returned_read_bytes(&self) -> u64 {
        self.returned_read_bytes
    }
    /// Return the number of preparation invocations.
    pub fn prepare_invocations(&self) -> u64 {
        self.prepare_invocations
    }
}

struct Piece {
    id: u64,
    bytes: Vec<u8>,
    len: u64,
}
struct OwnerState {
    artifact_identity: Arc<()>,
    pieces: BTreeMap<String, Piece>,
    metadata: Arc<Mutex<PreparedPlanMetadata>>,
    writer_fault: Option<Transport>,
    active_reads: u64,
}
struct StoreState {
    disposed: bool,
    next_owner: u64,
    owners: BTreeMap<u64, OwnerState>,
}

/// One print's storage authority.
pub struct PreparedOwnerStore {
    state: Arc<Mutex<StoreState>>,
}
/// Unforgeable print/artifact owner token; holds no payload storage alive.
#[derive(Clone)]
pub struct PreparationOwner {
    store: Weak<Mutex<StoreState>>,
    id: u64,
    artifact_identity: Arc<()>,
    stage_id: String,
    artifact_kind: String,
    declared_views: Vec<String>,
    metadata: Arc<Mutex<PreparedPlanMetadata>>,
}
impl PreparationOwner {
    /// Count an actual adapter invocation, including calls returning failure.
    pub(crate) fn record_prepare_invocation(&self) -> Result<(), PreparationFailure> {
        let mut metadata = lock(&self.metadata);
        metadata.prepare_invocations = add(metadata.prepare_invocations, 1).map_err(transport)?;
        Ok(())
    }

    /// Recover the print authority without keeping a dead store alive.
    pub(crate) fn originating_store(&self) -> Result<PreparedOwnerStore, PreparationFailure> {
        let state = self.store.upgrade().ok_or(PreparationFailure::Disposed)?;
        Ok(PreparedOwnerStore { state })
    }

    /// Binding identity alone does not distinguish owners within one print.
    pub(crate) fn same_owner(&self, other: &PreparationOwner) -> bool {
        Weak::ptr_eq(&self.store, &other.store) && self.id == other.id
    }

    /// Hold the existing read-call guard across binding, invocation and finalization.
    pub(crate) fn active_guard(&self) -> Result<ReadGuard, PreparationFailure> {
        let store = self.originating_store()?;
        check_owner(&mut lock(&store.state), self)?;
        ReadGuard::acquire(self).map_err(transport)
    }

    /// Snapshot only names and lengths while publication is still pending.
    pub(crate) fn staged_description(&self) -> Result<StagedPreparation, PreparationFailure> {
        let store = self.originating_store()?;
        let mut state = lock(&store.state);
        let entry = check_owner(&mut state, self)?;
        if lock(&entry.metadata).phase != "staging" {
            return Err(transport(Transport::NotReady));
        }
        Ok(StagedPreparation {
            owner: self.clone(),
            pieces: entry
                .pieces
                .iter()
                .map(|(name, piece)| (name.clone(), piece.len))
                .collect(),
        })
    }

    /// Return a redacted accounting snapshot.
    pub fn metadata(&self) -> PreparedPlanMetadata {
        lock(&self.metadata).clone()
    }
    /// Return the owner's stage identity.
    pub fn stage_id(&self) -> &str {
        &self.stage_id
    }
    /// Return the owner's artifact kind.
    pub fn artifact_kind(&self) -> &str {
        &self.artifact_kind
    }
    /// Return the declared preparation views.
    pub fn declared_views(&self) -> &[String] {
        &self.declared_views
    }
    /// Return the first recorded writer failure.
    pub fn writer_fault(&self) -> Option<Transport> {
        let store = self.store.upgrade()?;
        let state = lock(&store);
        state
            .owners
            .get(&self.id)
            .and_then(|owner| owner.writer_fault)
    }
}
impl fmt::Debug for PreparationOwner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.metadata().fmt(f)
    }
}
/// An active preparation guard. Dropping aborts unpublished staging.
pub struct PreparationTransaction {
    owner: PreparationOwner,
    completed: bool,
}
/// Weak readable plan authority.
#[derive(Clone, Debug)]
pub struct PreparedPlanHandle {
    owner: PreparationOwner,
}
/// Weak readable piece authority with an infallible captured length.
#[derive(Clone, Debug)]
pub struct PreparedPieceHandle {
    owner: PreparationOwner,
    id: u64,
    len: u64,
}
/// Validation-only, immutable staging description; never exposes payloads.
pub struct StagedPreparation {
    owner: PreparationOwner,
    pieces: Vec<(String, u64)>,
}
impl StagedPreparation {
    /// Return the staging owner authority.
    pub fn owner(&self) -> &PreparationOwner {
        &self.owner
    }
    /// Return the owner's declared views.
    pub fn declared_views(&self) -> &[String] {
        self.owner.declared_views()
    }
    /// Return the number of staged pieces.
    pub fn piece_count(&self) -> usize {
        self.pieces.len()
    }
    /// Return staged piece names and lengths.
    pub fn pieces(&self) -> &[(String, u64)] {
        &self.pieces
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
fn add(a: u64, b: u64) -> Result<u64, Transport> {
    a.checked_add(b).ok_or(Transport::ArithmeticOverflow)
}
fn count(value: usize) -> Result<u64, Transport> {
    u64::try_from(value).map_err(|_| Transport::ArithmeticOverflow)
}
fn transport(error: Transport) -> PreparationFailure {
    PreparationFailure::Transport(error)
}
fn check_owner<'a>(
    state: &'a mut StoreState,
    owner: &PreparationOwner,
) -> Result<&'a mut OwnerState, PreparationFailure> {
    let entry = state
        .owners
        .get_mut(&owner.id)
        .ok_or(PreparationFailure::InvalidOwner)?;
    if !Arc::ptr_eq(&entry.artifact_identity, &owner.artifact_identity) {
        return Err(PreparationFailure::InvalidOwner);
    }
    if state.disposed || lock(&entry.metadata).phase == "disposed" {
        return Err(PreparationFailure::Disposed);
    }
    Ok(entry)
}

impl Default for PreparedOwnerStore {
    fn default() -> Self {
        Self::new()
    }
}
impl PreparedOwnerStore {
    /// Create an empty print-scoped authority.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(StoreState {
                disposed: false,
                next_owner: 0,
                owners: BTreeMap::new(),
            })),
        }
    }
    fn verify(&self, owner: &PreparationOwner) -> Result<(), PreparationFailure> {
        if !Weak::ptr_eq(&owner.store, &Arc::downgrade(&self.state)) {
            return Err(PreparationFailure::InvalidOwner);
        }
        Ok(())
    }
    /// Validate both print ownership and the frozen artifact before adapter work.
    pub(crate) fn validate_owner(
        &self,
        owner: &PreparationOwner,
        binding: &FrozenPreparationBinding,
    ) -> Result<(), PreparationFailure> {
        self.verify(owner)?;
        if !Arc::ptr_eq(&owner.artifact_identity, &binding.artifact_identity()) {
            return Err(PreparationFailure::InvalidOwner);
        }
        check_owner(&mut lock(&self.state), owner)?;
        Ok(())
    }
    /// Create an owner for a frozen artifact binding.
    pub fn create_owner(
        &self,
        binding: &FrozenPreparationBinding,
    ) -> Result<PreparationOwner, PreparationFailure> {
        let mut state = lock(&self.state);
        if state.disposed {
            return Err(PreparationFailure::Disposed);
        }
        let id = state.next_owner;
        let next = add(id, 1).map_err(transport)?;
        let metadata = Arc::new(Mutex::new(PreparedPlanMetadata {
            module_id: binding.module_id().clone(),
            phase: "unready",
            ready: false,
            pieces: vec![],
            retained_payload_bytes: 0,
            retained_name_bytes: 0,
            staged_payload_bytes: 0,
            staged_name_bytes: 0,
            put_calls: 0,
            read_calls: 0,
            requested_read_bytes: 0,
            returned_read_bytes: 0,
            prepare_invocations: 0,
        }));
        let owner = PreparationOwner {
            store: Arc::downgrade(&self.state),
            id,
            artifact_identity: binding.artifact_identity(),
            stage_id: binding.stage_id().clone(),
            artifact_kind: binding.artifact_kind().into(),
            declared_views: binding
                .declaration()
                .map_or_else(Vec::new, |d| d.views().to_vec()),
            metadata: Arc::clone(&metadata),
        };
        state.owners.insert(
            id,
            OwnerState {
                artifact_identity: Arc::clone(&owner.artifact_identity),
                pieces: BTreeMap::new(),
                metadata,
                writer_fault: None,
                active_reads: 0,
            },
        );
        state.next_owner = next;
        Ok(owner)
    }
    /// Begin unpublished staging for an unready owner.
    pub fn begin(
        &self,
        owner: &PreparationOwner,
    ) -> Result<PreparationTransaction, PreparationFailure> {
        self.verify(owner)?;
        let mut state = lock(&self.state);
        let entry = check_owner(&mut state, owner)?;
        let mut metadata = lock(&entry.metadata);
        if metadata.phase != "unready" {
            return Err(PreparationFailure::AlreadyStarted);
        }
        metadata.phase = "staging";
        entry.writer_fault = None;
        Ok(PreparationTransaction {
            owner: owner.clone(),
            completed: false,
        })
    }
    /// Obtain the currently published plan authority.
    pub fn current(
        &self,
        owner: &PreparationOwner,
    ) -> Result<PreparedPlanHandle, PreparationFailure> {
        self.verify(owner)?;
        let mut state = lock(&self.state);
        let entry = check_owner(&mut state, owner)?;
        if !lock(&entry.metadata).ready {
            return Err(transport(Transport::NotReady));
        }
        Ok(PreparedPlanHandle {
            owner: owner.clone(),
        })
    }
    /// Dispose an idle owner's retained storage.
    pub fn dispose_owner(&self, owner: &PreparationOwner) -> Result<(), PreparationFailure> {
        self.verify(owner)?;
        let mut state = match self.state.try_lock() {
            Ok(state) => state,
            Err(TryLockError::WouldBlock) => return Err(PreparationFailure::Busy),
            Err(TryLockError::Poisoned(error)) => error.into_inner(),
        };
        let entry = check_owner(&mut state, owner)?;
        if lock(&entry.metadata).phase == "staging" || entry.active_reads != 0 {
            return Err(PreparationFailure::Busy);
        }
        clear(entry, "disposed");
        Ok(())
    }
    /// Dispose all idle owners and the print authority.
    pub fn dispose(&self) -> Result<(), PreparationFailure> {
        let mut state = match self.state.try_lock() {
            Ok(state) => state,
            Err(TryLockError::WouldBlock) => return Err(PreparationFailure::Busy),
            Err(TryLockError::Poisoned(error)) => error.into_inner(),
        };
        if state
            .owners
            .values()
            .any(|entry| lock(&entry.metadata).phase == "staging" || entry.active_reads != 0)
        {
            return Err(PreparationFailure::Busy);
        }
        for entry in state.owners.values_mut() {
            clear(entry, "disposed");
        }
        state.disposed = true;
        Ok(())
    }
    #[cfg(test)]
    fn force_counters_to_limit(&self, owner: &PreparationOwner, field: &str) {
        let mut state = lock(&self.state);
        let entry = check_owner(&mut state, owner).expect("test owner");
        let mut m = lock(&entry.metadata);
        match field {
            "put_calls" => m.put_calls = u64::MAX,
            "staged_payload_bytes" => m.staged_payload_bytes = u64::MAX,
            "staged_name_bytes" => m.staged_name_bytes = u64::MAX,
            "read_calls" => m.read_calls = u64::MAX,
            "requested_read_bytes" => m.requested_read_bytes = u64::MAX,
            "returned_read_bytes" => m.returned_read_bytes = u64::MAX,
            "prepare_invocations" => m.prepare_invocations = u64::MAX,
            _ => panic!("unknown counter"),
        }
    }
}
fn clear(entry: &mut OwnerState, phase: &'static str) {
    entry.pieces.clear();
    let mut m = lock(&entry.metadata);
    m.phase = phase;
    m.ready = false;
    m.pieces.clear();
    m.retained_payload_bytes = 0;
    m.retained_name_bytes = 0;
    m.staged_payload_bytes = 0;
    m.staged_name_bytes = 0;
}

impl PreparationTransaction {
    /// Preserve SDK-local rejections that did not reach the store's put operation.
    pub(crate) fn record_writer_fault(&mut self, error: Transport) {
        if let Some(store) = self.owner.store.upgrade() {
            let mut state = lock(&store);
            if let Ok(entry) = check_owner(&mut state, &self.owner) {
                entry.writer_fault.get_or_insert(error);
            }
        }
    }

    /// Stage a uniquely named opaque payload.
    pub fn put(&mut self, name: &str, bytes: &[u8]) -> Result<(), Transport> {
        let store = self.owner.store.upgrade().ok_or(Transport::AccessDenied)?;
        let mut state = lock(&store);
        let entry = check_owner(&mut state, &self.owner).map_err(|_| Transport::AccessDenied)?;
        if entry.writer_fault.is_some() {
            return Err(Transport::PoisonedOutput);
        }
        let result = (|| {
            if name.is_empty() {
                return Err(Transport::InvalidName);
            }
            if entry.pieces.contains_key(name) {
                return Err(Transport::DuplicateName);
            }
            let len = count(bytes.len())?;
            let name_len = count(name.len())?;
            let id = count(entry.pieces.len())?;
            let mut m = lock(&entry.metadata);
            let payload = add(m.staged_payload_bytes, len)?;
            let names = add(m.staged_name_bytes, name_len)?;
            let calls = add(m.put_calls, 1)?;
            // All fallible checks precede either allocation or accounting mutation.
            entry.pieces.insert(
                name.to_owned(),
                Piece {
                    id,
                    bytes: bytes.to_vec(),
                    len,
                },
            );
            m.staged_payload_bytes = payload;
            m.staged_name_bytes = names;
            m.put_calls = calls;
            Ok(())
        })();
        if let Err(error) = result {
            entry.writer_fault = Some(error);
        }
        result
    }
    /// Validate and atomically publish successful staging.
    pub fn finish(
        mut self,
        result: Result<(), PreparationFailure>,
        validate: impl FnOnce(&StagedPreparation) -> Result<(), PreparationFailure>,
    ) -> Result<PreparedPlanHandle, PreparationFailure> {
        // The writer fault remains recorded in owner state even on module/trap failure.
        if matches!(
            &result,
            Err(PreparationFailure::Module(_) | PreparationFailure::Trap { .. })
        ) {
            return result.map(|()| PreparedPlanHandle {
                owner: self.owner.clone(),
            });
        }
        let store = self
            .owner
            .store
            .upgrade()
            .ok_or(PreparationFailure::Disposed)?;
        let staged = {
            let mut state = lock(&store);
            let entry = check_owner(&mut state, &self.owner)?;
            if entry.writer_fault.is_some() {
                return Err(transport(Transport::PoisonedOutput));
            }
            StagedPreparation {
                owner: self.owner.clone(),
                pieces: entry
                    .pieces
                    .iter()
                    .map(|(name, piece)| (name.clone(), piece.len))
                    .collect(),
            }
        };
        result?;
        // No store lock across user validation; it sees staging, not readiness.
        validate(&staged)?;
        let mut state = lock(&store);
        let entry = check_owner(&mut state, &self.owner)?;
        let mut m = lock(&entry.metadata);
        m.pieces = staged.pieces;
        m.retained_payload_bytes = m.staged_payload_bytes;
        m.retained_name_bytes = m.staged_name_bytes;
        m.staged_payload_bytes = 0;
        m.staged_name_bytes = 0;
        m.phase = "ready";
        m.ready = true;
        self.completed = true;
        Ok(PreparedPlanHandle {
            owner: self.owner.clone(),
        })
    }
}
impl Drop for PreparationTransaction {
    fn drop(&mut self) {
        if !self.completed {
            if let Some(store) = self.owner.store.upgrade() {
                let mut state = lock(&store);
                if let Ok(entry) = check_owner(&mut state, &self.owner) {
                    clear(entry, "unready");
                }
            }
        }
    }
}

impl PreparedPlanHandle {
    /// Return a redacted plan accounting snapshot.
    pub fn metadata(&self) -> PreparedPlanMetadata {
        self.owner.metadata()
    }
    /// Open an optional named published piece.
    pub fn open(&self, name: &str) -> Result<Option<PreparedPieceHandle>, Transport> {
        let store = self.owner.store.upgrade().ok_or(Transport::AccessDenied)?;
        let mut state = lock(&store);
        let entry = check_owner(&mut state, &self.owner).map_err(|_| Transport::AccessDenied)?;
        if !lock(&entry.metadata).ready {
            return Err(Transport::NotReady);
        }
        if name.is_empty() {
            return Err(Transport::InvalidName);
        }
        Ok(entry.pieces.get(name).map(|piece| PreparedPieceHandle {
            owner: self.owner.clone(),
            id: piece.id,
            len: piece.len,
        }))
    }
}

pub(crate) struct ReadGuard {
    owner: PreparationOwner,
}
impl ReadGuard {
    fn acquire(owner: &PreparationOwner) -> Result<Self, Transport> {
        let store = owner.store.upgrade().ok_or(Transport::AccessDenied)?;
        let mut state = lock(&store);
        let entry = check_owner(&mut state, owner).map_err(|_| Transport::AccessDenied)?;
        entry.active_reads = add(entry.active_reads, 1)?;
        Ok(Self {
            owner: owner.clone(),
        })
    }
}
impl Drop for ReadGuard {
    fn drop(&mut self) {
        if let Some(store) = self.owner.store.upgrade() {
            let mut state = lock(&store);
            if let Some(entry) = state.owners.get_mut(&self.owner.id) {
                entry.active_reads = entry
                    .active_reads
                    .checked_sub(1)
                    .expect("balanced read guard");
            }
        }
    }
}
impl PreparedPieceHandle {
    /// Return the captured piece length.
    pub fn len(&self) -> u64 {
        self.len
    }
    /// Report whether the captured piece is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    /// Read a bounded opaque range under owner authority.
    pub fn read(&self, offset: u64, max_bytes: u64) -> Result<Vec<u8>, Transport> {
        let _guard = ReadGuard::acquire(&self.owner)?;
        let store = self.owner.store.upgrade().ok_or(Transport::AccessDenied)?;
        let mut state = lock(&store);
        let entry = check_owner(&mut state, &self.owner).map_err(|_| Transport::AccessDenied)?;
        if !lock(&entry.metadata).ready {
            return Err(Transport::NotReady);
        }
        let piece = entry
            .pieces
            .values()
            .find(|piece| piece.id == self.id)
            .ok_or(Transport::AccessDenied)?;
        if offset > piece.len {
            return Err(Transport::RangeError);
        }
        let remaining = piece
            .len
            .checked_sub(offset)
            .ok_or(Transport::ArithmeticOverflow)?;
        let take = max_bytes.min(remaining);
        let take_index = usize::try_from(take).map_err(|_| Transport::ArithmeticOverflow)?;
        let start = usize::try_from(offset).map_err(|_| Transport::ArithmeticOverflow)?;
        let end = start
            .checked_add(take_index)
            .ok_or(Transport::ArithmeticOverflow)?;
        let bounded = piece
            .bytes
            .get(start..end)
            .ok_or(Transport::ArithmeticOverflow)?;
        let mut m = lock(&entry.metadata);
        let calls = add(m.read_calls, 1)?;
        let requested = add(m.requested_read_bytes, max_bytes)?;
        let returned = add(m.returned_read_bytes, take)?;
        let bytes = bounded.to_vec();
        m.read_calls = calls;
        m.requested_read_bytes = requested;
        m.returned_read_bytes = returned;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preparation::test_binding;

    fn owner(store: &PreparedOwnerStore) -> PreparationOwner {
        store
            .create_owner(&test_binding())
            .expect("create test owner")
    }
    fn publish(
        store: &PreparedOwnerStore,
        owner: &PreparationOwner,
        bytes: &[u8],
    ) -> PreparedPieceHandle {
        let mut transaction = store.begin(owner).expect("begin");
        assert_eq!(transaction.put("numbers", bytes), Ok(()));
        transaction
            .finish(Ok(()), |_| Ok(()))
            .expect("publish")
            .open("numbers")
            .expect("open")
            .expect("piece")
    }
    fn assert_unready(store: &PreparedOwnerStore, owner: &PreparationOwner) {
        assert!(matches!(
            store.current(owner),
            Err(PreparationFailure::Transport(Transport::NotReady))
        ));
        let m = owner.metadata();
        assert_eq!(m.phase(), "unready");
        assert!(!m.ready());
        assert_eq!(m.pieces(), &[]);
        assert_eq!(m.retained_payload_bytes(), 0);
        assert_eq!(m.retained_name_bytes(), 0);
        assert_eq!(m.staged_payload_bytes(), 0);
        assert_eq!(m.staged_name_bytes(), 0);
    }

    #[test]
    fn pub_crate_seam_rejects_foreign_owner() {
        let a = PreparedOwnerStore::new();
        let b = PreparedOwnerStore::new();
        let binding = test_binding();
        let first = a.create_owner(&binding).expect("first owner");
        let second = a.create_owner(&binding).expect("second owner");
        assert!(matches!(
            b.validate_owner(&first, &binding),
            Err(PreparationFailure::InvalidOwner)
        ));
        assert!(matches!(a.validate_owner(&first, &binding), Ok(())));
        assert!(matches!(a.validate_owner(&second, &binding), Ok(())));
        assert_eq!(first.same_owner(&second), false);
        assert_eq!(first.same_owner(&first.clone()), true);
        assert!(matches!(
            a.validate_owner(&first, &test_binding()),
            Err(PreparationFailure::InvalidOwner)
        ));
        let recovered = first.originating_store().expect("originating store");
        assert!(matches!(recovered.validate_owner(&first, &binding), Ok(())));
        let mut transaction = recovered.begin(&first).expect("begin via recovered store");
        assert_eq!(transaction.put("piece", &[3, 5]), Ok(()));
        let staged = first.staged_description().expect("staged description");
        assert_eq!(staged.owner().same_owner(&first), true);
        assert_eq!(staged.declared_views(), first.declared_views());
        assert_eq!(staged.piece_count(), 1);
        assert_eq!(staged.pieces(), &[("piece".to_owned(), 2)]);
        drop(transaction);
        drop(recovered);
        drop(a);
        assert!(matches!(
            first.originating_store(),
            Err(PreparationFailure::Disposed)
        ));
    }

    #[test]
    fn pub_crate_seam_active_guard_blocks_disposal() {
        let store = PreparedOwnerStore::new();
        let first = owner(&store);
        let guard = first.active_guard().expect("whole-call guard");
        assert!(matches!(
            store.dispose_owner(&first),
            Err(PreparationFailure::Busy)
        ));
        assert!(matches!(store.dispose(), Err(PreparationFailure::Busy)));
        drop(guard);
        assert!(matches!(store.dispose_owner(&first), Ok(())));

        let second = owner(&store);
        let result = std::panic::catch_unwind(|| {
            let _guard = second.active_guard().expect("unwind guard");
            assert!(matches!(
                store.dispose_owner(&second),
                Err(PreparationFailure::Busy)
            ));
            panic!("whole-call unwind");
        });
        let panic = result.expect_err("scope must unwind");
        assert_eq!(panic.downcast_ref::<&str>(), Some(&"whole-call unwind"));
        assert!(matches!(store.dispose_owner(&second), Ok(())));
        assert!(matches!(store.dispose(), Ok(())));
    }

    #[test]
    fn checked_accounting_never_wraps() {
        for field in ["put_calls", "staged_payload_bytes", "staged_name_bytes"] {
            let store = PreparedOwnerStore::new();
            let owner = owner(&store);
            let mut transaction = store.begin(&owner).expect("begin");
            store.force_counters_to_limit(&owner, field);
            assert_eq!(
                transaction.put("a", &[1]),
                Err(Transport::ArithmeticOverflow)
            );
            let m = owner.metadata();
            assert_eq!(
                m.put_calls(),
                if field == "put_calls" { u64::MAX } else { 0 }
            );
            assert_eq!(
                m.staged_payload_bytes(),
                if field == "staged_payload_bytes" {
                    u64::MAX
                } else {
                    0
                }
            );
            assert_eq!(
                m.staged_name_bytes(),
                if field == "staged_name_bytes" {
                    u64::MAX
                } else {
                    0
                }
            );
            assert_eq!(
                lock(&store.state)
                    .owners
                    .get(&owner.id)
                    .expect("state")
                    .pieces
                    .len(),
                0
            );
            assert_eq!(owner.writer_fault(), Some(Transport::ArithmeticOverflow));
            assert_eq!(transaction.put("b", &[2]), Err(Transport::PoisonedOutput));
            assert!(matches!(
                transaction.finish(Ok(()), |_| panic!("poison must precede validation")),
                Err(PreparationFailure::Transport(Transport::PoisonedOutput))
            ));
            assert_unready(&store, &owner);
        }
        for field in ["read_calls", "requested_read_bytes", "returned_read_bytes"] {
            let store = PreparedOwnerStore::new();
            let owner = owner(&store);
            let piece = publish(&store, &owner, &[3, 5, 11, 17]);
            store.force_counters_to_limit(&owner, field);
            assert_eq!(piece.read(0, 1), Err(Transport::ArithmeticOverflow));
            let m = owner.metadata();
            assert_eq!(
                m.read_calls(),
                if field == "read_calls" { u64::MAX } else { 0 }
            );
            assert_eq!(
                m.requested_read_bytes(),
                if field == "requested_read_bytes" {
                    u64::MAX
                } else {
                    0
                }
            );
            assert_eq!(
                m.returned_read_bytes(),
                if field == "returned_read_bytes" {
                    u64::MAX
                } else {
                    0
                }
            );
            assert_eq!(m.retained_payload_bytes(), 4);
            assert_eq!(m.retained_name_bytes(), 7);
            assert_eq!(m.put_calls(), 1);
            assert_eq!(m.prepare_invocations(), 0);
        }
        let store = PreparedOwnerStore::new();
        let owner = owner(&store);
        let piece = publish(&store, &owner, &[3, 5, 11, 17]);
        assert_eq!(piece.read(0, 1), Ok(vec![3]));
        assert_eq!(piece.read(1, u64::MAX), Err(Transport::ArithmeticOverflow));
        assert_eq!(owner.metadata().read_calls(), 1);
        assert_eq!(owner.metadata().requested_read_bytes(), 1);
        assert_eq!(owner.metadata().returned_read_bytes(), 1);
        lock(&store.state).next_owner = u64::MAX;
        assert!(matches!(
            store.create_owner(&test_binding()),
            Err(PreparationFailure::Transport(Transport::ArithmeticOverflow))
        ));
        assert_eq!(lock(&store.state).next_owner, u64::MAX);
        assert_eq!(lock(&store.state).owners.len(), 1);
        // usize -> u64 cannot overflow on this target; exercise the same checked
        // production helper at its largest representable input instead.
        assert_eq!(count(4), Ok(4));
        assert_eq!(count(0), Ok(0));
        #[cfg(target_pointer_width = "64")]
        assert_eq!(count(usize::MAX), Ok(u64::MAX));
        #[cfg(target_pointer_width = "32")]
        assert_eq!(count(usize::MAX), Ok(4_294_967_295));

        let store = PreparedOwnerStore::new();
        let owner = super::tests::owner(&store);
        store.force_counters_to_limit(&owner, "prepare_invocations");
        assert!(matches!(
            owner.record_prepare_invocation(),
            Err(PreparationFailure::Transport(Transport::ArithmeticOverflow))
        ));
        assert_eq!(owner.metadata().prepare_invocations(), u64::MAX);
        assert_unready(&store, &owner);
    }

    #[test]
    fn failed_put_poisons_and_never_publishes() {
        for empty in [false, true] {
            let store = PreparedOwnerStore::new();
            let owner = owner(&store);
            let mut transaction = store.begin(&owner).expect("begin");
            assert_eq!(transaction.put("a", &[1]), Ok(()));
            let expected = if empty {
                Transport::InvalidName
            } else {
                Transport::DuplicateName
            };
            assert_eq!(
                transaction.put(if empty { "" } else { "a" }, &[2]),
                Err(expected)
            );
            assert_eq!(transaction.put("b", &[2]), Err(Transport::PoisonedOutput));
            assert!(matches!(
                transaction.finish(Ok(()), |_| panic!("poison must not validate")),
                Err(PreparationFailure::Transport(Transport::PoisonedOutput))
            ));
            assert_eq!(owner.writer_fault(), Some(expected));
            assert_unready(&store, &owner);
        }
        let store = PreparedOwnerStore::new();
        let owner = owner(&store);
        let mut transaction = store.begin(&owner).expect("begin");
        assert_eq!(transaction.put("", &[1]), Err(Transport::InvalidName));
        match transaction.finish(
            Err(PreparationFailure::Module(
                slicer_sdk::error::ModuleError::fatal(42, "original"),
            )),
            |_| panic!("module failure"),
        ) {
            Err(PreparationFailure::Module(error)) => {
                assert_eq!(error.code, 42);
                assert_eq!(error.message, "original");
                assert!(error.fatal);
            }
            other => panic!("wrong original failure: {other:?}"),
        }
        assert_eq!(owner.writer_fault(), Some(Transport::InvalidName));
        assert_unready(&store, &owner);
        let mut transaction = store.begin(&owner).expect("retry");
        assert_eq!(transaction.put("", &[]), Err(Transport::InvalidName));
        match transaction.finish(
            Err(PreparationFailure::Trap {
                detail: "original trap".into(),
            }),
            |_| panic!("trap failure"),
        ) {
            Err(PreparationFailure::Trap { detail }) => assert_eq!(detail, "original trap"),
            other => panic!("wrong trap failure: {other:?}"),
        }
        assert_unready(&store, &owner);
    }

    #[test]
    fn finish_publishes_atomically_or_not_at_all() {
        let store = PreparedOwnerStore::new();
        let owner = owner(&store);
        let mut transaction = store.begin(&owner).expect("begin");
        assert!(matches!(
            store.begin(&owner),
            Err(PreparationFailure::AlreadyStarted)
        ));
        assert_eq!(transaction.put("a", &[1, 2]), Ok(()));
        match transaction.finish(Ok(()), |staged| {
            assert_eq!(staged.piece_count(), 1);
            assert_eq!(staged.pieces(), &[("a".to_owned(), 2)]);
            assert_eq!(staged.declared_views(), &["view"]);
            assert_eq!(staged.owner().metadata().phase(), "staging");
            assert!(matches!(
                store.current(&owner),
                Err(PreparationFailure::Transport(Transport::NotReady))
            ));
            Err(PreparationFailure::PublicationRejected {
                detail: "reject".into(),
            })
        }) {
            Err(PreparationFailure::PublicationRejected { detail }) => assert_eq!(detail, "reject"),
            other => panic!("wrong rejection: {other:?}"),
        }
        assert_unready(&store, &owner);
        let mut transaction = store.begin(&owner).expect("retry");
        assert_eq!(transaction.put("a", &[3, 5]), Ok(()));
        assert_eq!(transaction.put("b", &[11, 17]), Ok(()));
        let plan = transaction
            .finish(Ok(()), |staged| {
                assert_eq!(staged.pieces(), &[("a".to_owned(), 2), ("b".to_owned(), 2)]);
                assert_eq!(staged.owner().metadata().phase(), "staging");
                assert!(!staged.owner().metadata().ready());
                assert_eq!(staged.owner().metadata().retained_payload_bytes(), 0);
                Ok(())
            })
            .expect("publish");
        assert_eq!(
            plan.open("a").expect("open").expect("a").read(0, 2),
            Ok(vec![3, 5])
        );
        assert_eq!(
            plan.open("b").expect("open").expect("b").read(0, 2),
            Ok(vec![11, 17])
        );
        assert_eq!(
            plan.metadata().pieces(),
            &[("a".to_owned(), 2), ("b".to_owned(), 2)]
        );
        assert!(plan.metadata().ready());
        assert!(matches!(
            store.begin(&owner),
            Err(PreparationFailure::AlreadyStarted)
        ));
        let empty = super::tests::owner(&store);
        let empty_plan = store
            .begin(&empty)
            .expect("empty begin")
            .finish(Ok(()), |staged| {
                assert_eq!(staged.piece_count(), 0);
                assert_eq!(staged.owner().metadata().phase(), "staging");
                Ok(())
            })
            .expect("empty publish");
        assert!(empty_plan.metadata().ready());
        assert_eq!(empty_plan.metadata().pieces(), &[]);
        let dropped = super::tests::owner(&store);
        let mut transaction = store.begin(&dropped).expect("begin dropped");
        assert_eq!(transaction.put("a", &[1]), Ok(()));
        drop(transaction);
        assert_unready(&store, &dropped);
    }

    #[test]
    fn open_distinguishes_missing_optional_from_unready() {
        let store = PreparedOwnerStore::new();
        let owner = owner(&store);
        assert_unready(&store, &owner);
        let unready_handle = PreparedPlanHandle {
            owner: owner.clone(),
        };
        assert!(matches!(
            unready_handle.open("missing"),
            Err(Transport::NotReady)
        ));
        let plan = store
            .begin(&owner)
            .expect("begin")
            .finish(Ok(()), |_| Ok(()))
            .expect("publish");
        assert!(matches!(plan.open("missing"), Ok(None)));
        assert!(matches!(plan.open(""), Err(Transport::InvalidName)));
    }

    #[test]
    fn read_arithmetic_clips_before_conversion() {
        let cases = [
            (1, 2, Ok(vec![5, 11])),
            (0, u64::MAX, Ok(vec![3, 5, 11, 17])),
            (1, u64::MAX, Ok(vec![5, 11, 17])),
            (4, 0, Ok(vec![])),
            (4, 1, Ok(vec![])),
            (4, u64::MAX, Ok(vec![])),
            (5, 0, Err(Transport::RangeError)),
            (u64::MAX, 0, Err(Transport::RangeError)),
            (u64::MAX, 1, Err(Transport::RangeError)),
        ];
        for (offset, maximum, expected) in cases {
            let store = PreparedOwnerStore::new();
            let owner = owner(&store);
            let piece = publish(&store, &owner, &[3, 5, 11, 17]);
            assert_eq!(piece.len(), 4);
            assert_eq!(piece.read(offset, maximum), expected);
            if offset > 4 {
                assert_eq!(owner.metadata().read_calls(), 0);
                assert_eq!(owner.metadata().requested_read_bytes(), 0);
                assert_eq!(owner.metadata().returned_read_bytes(), 0);
            } else {
                assert_eq!(owner.metadata().read_calls(), 1);
                assert_eq!(owner.metadata().requested_read_bytes(), maximum);
            }
        }
    }

    #[test]
    fn disposal_is_busy_under_active_guard_and_denies_later_reads() {
        let store = PreparedOwnerStore::new();
        let first = owner(&store);
        let second = owner(&store);
        let transaction = store.begin(&first).expect("begin");
        assert!(matches!(
            store.dispose_owner(&first),
            Err(PreparationFailure::Busy)
        ));
        assert!(matches!(store.dispose(), Err(PreparationFailure::Busy)));
        drop(transaction);
        let piece = publish(&store, &first, &[3, 5, 11, 17]);
        let other = publish(&store, &second, &[23]);
        let plan = store.current(&first).expect("current");
        let guard = ReadGuard::acquire(&first).expect("active read");
        assert!(matches!(
            store.dispose_owner(&first),
            Err(PreparationFailure::Busy)
        ));
        assert!(matches!(store.dispose(), Err(PreparationFailure::Busy)));
        drop(guard);
        store.dispose_owner(&first).expect("dispose owner");
        assert_eq!(piece.read(0, 1), Err(Transport::AccessDenied));
        assert_eq!(piece.len(), 4);
        assert!(matches!(plan.open("numbers"), Err(Transport::AccessDenied)));
        assert_eq!(other.read(0, 1), Ok(vec![23]));
        assert_eq!(first.metadata().retained_payload_bytes(), 0);
        assert_eq!(first.metadata().retained_name_bytes(), 0);
        store.dispose().expect("dispose print");
        assert_eq!(second.metadata().phase(), "disposed");
        assert_eq!(second.metadata().retained_payload_bytes(), 0);
        assert_eq!(second.metadata().retained_name_bytes(), 0);
        assert_eq!(other.read(0, 1), Err(Transport::AccessDenied));
        assert_eq!(other.len(), 1);
        assert!(matches!(
            store.create_owner(&test_binding()),
            Err(PreparationFailure::Disposed)
        ));
        let orphan = {
            let store = PreparedOwnerStore::new();
            let owner = owner(&store);
            publish(&store, &owner, &[31])
        };
        assert_eq!(orphan.read(0, 1), Err(Transport::AccessDenied));
        assert_eq!(orphan.len(), 1);
    }

    #[test]
    fn metadata_is_redacted_and_counts_names_and_payload_separately() {
        let store = PreparedOwnerStore::new();
        let owner = owner(&store);
        let mut transaction = store.begin(&owner).expect("begin");
        assert_eq!(transaction.put("numbers", &[3, 5, 11, 17]), Ok(()));
        assert_eq!(transaction.put("π", &[]), Ok(()));
        assert_eq!(owner.metadata().staged_payload_bytes(), 4);
        assert_eq!(owner.metadata().staged_name_bytes(), 9);
        let plan = transaction.finish(Ok(()), |_| Ok(())).expect("publish");
        let m = plan.metadata();
        assert_eq!(m.module_id(), "test.module");
        assert_eq!(m.phase(), "ready");
        assert!(m.ready());
        assert_eq!(
            m.pieces(),
            &[("numbers".to_owned(), 4), ("π".to_owned(), 0)]
        );
        assert_eq!(m.retained_payload_bytes(), 4);
        assert_eq!(m.retained_name_bytes(), 9);
        assert_eq!(m.staged_payload_bytes(), 0);
        assert_eq!(m.staged_name_bytes(), 0);
        assert_eq!(m.put_calls(), 2);
        assert_eq!(m.read_calls(), 0);
        assert_eq!(m.requested_read_bytes(), 0);
        assert_eq!(m.returned_read_bytes(), 0);
        assert_eq!(m.prepare_invocations(), 0);
        assert_eq!(format!("{m:?}"), "PreparedPlanMetadata { module_id: \"test.module\", phase: \"ready\", ready: true, pieces: [(\"numbers\", 4), (\"π\", 0)], retained_payload_bytes: 4, retained_name_bytes: 9, staged_payload_bytes: 0, staged_name_bytes: 0, put_calls: 2, read_calls: 0, requested_read_bytes: 0, returned_read_bytes: 0, prepare_invocations: 0 }");
    }

    #[test]
    fn owners_and_stores_are_isolated() {
        let a = PreparedOwnerStore::new();
        let b = PreparedOwnerStore::new();
        let binding = test_binding();
        let a1 = a.create_owner(&binding).expect("a1");
        let a2 = a.create_owner(&binding).expect("a2");
        let b1 = b.create_owner(&binding).expect("b1");
        let p1 = publish(&a, &a1, &[3, 5, 11, 17]);
        let p2 = publish(&a, &a2, &[23, 29]);
        let pb = publish(&b, &b1, &[31]);
        assert!(matches!(
            a.current(&b1),
            Err(PreparationFailure::InvalidOwner)
        ));
        assert!(matches!(
            b.begin(&a1),
            Err(PreparationFailure::InvalidOwner)
        ));
        assert!(matches!(
            b.dispose_owner(&a2),
            Err(PreparationFailure::InvalidOwner)
        ));
        assert_eq!(p1.read(1, 2), Ok(vec![5, 11]));
        assert_eq!(p2.read(1, 2), Ok(vec![29]));
        assert_eq!(pb.read(0, 1), Ok(vec![31]));
        a.dispose_owner(&a1).expect("dispose a1");
        assert_eq!(p1.read(0, 1), Err(Transport::AccessDenied));
        assert_eq!(p2.read(0, 1), Ok(vec![23]));
        assert_eq!(pb.read(0, 1), Ok(vec![31]));
        let fresh = a
            .current(&a2)
            .expect("fresh")
            .open("numbers")
            .expect("open")
            .expect("piece");
        assert_eq!(fresh.read(0, 2), Ok(vec![23, 29]));
    }
}
