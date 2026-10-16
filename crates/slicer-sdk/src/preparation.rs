//! Call-local access to host-owned layer preparation resources.
//!
//! The callback constructors adapt generated WIT imports without interpreting
//! the bytes. Native hosts use the same seam; no plan codec belongs in the SDK.
//! Typed input reads return owned projections through a shared reader facade.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::{Rc, Weak};
use std::sync::Arc;

pub use slicer_schema::preparation::PreparationInputError;
use slicer_schema::preparation::PreparationTransportError;

use crate::error::ModuleError;

#[path = "preparation_input.rs"]
mod preparation_input;

pub use preparation_input::{
    PreparationActiveRegion, PreparationBridgeRegion, PreparationFacetClass, PreparationInputPhase,
    PreparationLayer, PreparationMeshObject, PreparationMeshQuery, PreparationMeshQueryResult,
    PreparationNonplanarShell, PreparationOverhangRegion, PreparationPaintLayer,
    PreparationPaintStroke, PreparationPaintValue, PreparationRaftPlan, PreparationRegion,
    PreparationRegionConfig, PreparationRegionKey, PreparationScoredSeamCandidate, PreparationSeam,
    PreparationSeamReason, PreparationSelectedTarget, PreparationSupportAnalysisBatch,
    PreparationSupportCandidate, PreparationSupportCandidateSource,
    PreparationSupportFamilyAssignment, PreparationSupportGeometryBatch,
    PreparationSupportGeometryEntry, PreparationSupportGeometryKey, PreparationSupportPlanBatch,
    PreparationSupportPlanDeclineReason, PreparationSupportPlanEntry, PreparationSupportPlanRole,
    PreparationSupportPlanRoleRegion, PreparationSupportPlanSkeleton, PreparationSupportSourceKey,
    PreparationSurfaceLayer, PreparationSurfaceObject, PreparationTargetSource,
    PreparationVariantChain, PrintPreparationRead,
};

/// Opt-in preparation capability, independent of ordinary layer construction.
///
/// Implementations must supply the preparation method; there is no default.
///
/// ```compile_fail,E0046
/// use slicer_sdk::preparation::LayerPreparation;
/// struct MissingPreparation;
/// impl LayerPreparation for MissingPreparation {}
/// ```
pub trait LayerPreparation {
    /// Prepare serialized named pieces without constructing a module instance.
    fn prepare_print(
        input: &PrintPreparationInput,
        output: &mut PreparedPlanOutput,
    ) -> Result<(), ModuleError>;
}

/// Errors exposed by the `slicer:layer-preparation` resource interfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparationError {
    /// No prepared plan is bound to this call.
    NotReady,
    /// A piece name is empty.
    InvalidName,
    /// A complete piece with this name was already staged.
    DuplicateName,
    /// An earlier staging operation failed.
    PoisonedOutput,
    /// The requested offset or returned byte count is out of bounds.
    RangeError,
    /// The requested range cannot be represented.
    ArithmeticOverflow,
    /// The resource does not belong to the active call's owner.
    AccessDenied,
}

impl std::fmt::Display for PreparationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotReady => "not-ready",
            Self::InvalidName => "invalid-name",
            Self::DuplicateName => "duplicate-name",
            Self::PoisonedOutput => "poisoned-output",
            Self::RangeError => "range-error",
            Self::ArithmeticOverflow => "arithmetic-overflow",
            Self::AccessDenied => "access-denied",
        })
    }
}

impl std::error::Error for PreparationError {}

impl From<PreparationTransportError> for PreparationError {
    fn from(error: PreparationTransportError) -> Self {
        match error {
            PreparationTransportError::NotReady => Self::NotReady,
            PreparationTransportError::InvalidName => Self::InvalidName,
            PreparationTransportError::DuplicateName => Self::DuplicateName,
            PreparationTransportError::PoisonedOutput => Self::PoisonedOutput,
            PreparationTransportError::RangeError => Self::RangeError,
            PreparationTransportError::ArithmeticOverflow => Self::ArithmeticOverflow,
            PreparationTransportError::AccessDenied => Self::AccessDenied,
        }
    }
}

impl From<PreparationError> for PreparationTransportError {
    fn from(error: PreparationError) -> Self {
        match error {
            PreparationError::NotReady => Self::NotReady,
            PreparationError::InvalidName => Self::InvalidName,
            PreparationError::DuplicateName => Self::DuplicateName,
            PreparationError::PoisonedOutput => Self::PoisonedOutput,
            PreparationError::RangeError => Self::RangeError,
            PreparationError::ArithmeticOverflow => Self::ArithmeticOverflow,
            PreparationError::AccessDenied => Self::AccessDenied,
        }
    }
}

/// Native host access to an immutable named-piece plan.
pub trait PreparedPlanRead {
    /// Validate readiness when acquiring a call-local current view.
    fn check_ready(&self) -> Result<(), PreparationTransportError> {
        Ok(())
    }

    /// Open a piece without decoding or copying its contents.
    fn open(
        &self,
        name: &str,
    ) -> Result<Option<Arc<dyn PreparedPieceRead + Send + Sync>>, PreparationTransportError>;
}

/// Native host access to opaque immutable piece bytes.
pub trait PreparedPieceRead {
    /// Return the serialized byte count.
    fn len(&self) -> u64;
    /// Return whether the immutable piece contains no bytes.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Read a bounded range, truncating at EOF.
    fn read(&self, offset: u64, max_bytes: u64) -> Result<Vec<u8>, PreparationTransportError>;
}

impl From<PreparationError> for ModuleError {
    fn from(error: PreparationError) -> Self {
        Self::fatal(1, format!("layer preparation: {error}"))
    }
}

type ReadCallback = dyn Fn(u64, u64) -> Result<Vec<u8>, PreparationError>;
type PutCallback = dyn FnMut(&str, &[u8]) -> Result<(), PreparationError>;
type OpenCallback = dyn Fn(&str) -> Result<Option<PreparedPieceCallbacks>, PreparationError>;
type CurrentCallback = dyn Fn() -> Result<PreparedPlanCallbacks, PreparationError>;

/// Stable completion discriminator for SDK-poisoned preparation staging.
/// Carried in the existing WIT module-error code, independently of diagnostics.
pub const POISONED_PREPARATION_CODE: u32 = 0x5052_5001;

fn poisoned_preparation_message(error: PreparationError) -> String {
    format!("layer preparation: poisoned-output; original put error: {error}")
}

/// Recognize the SDK-owned completion marker across the WIT boundary.
/// The code alone is not reserved against genuine module errors.
#[doc(hidden)]
pub fn is_poisoned_preparation_error(error: &ModuleError) -> bool {
    error.code == POISONED_PREPARATION_CODE
        && error.fatal
        && [
            PreparationError::NotReady,
            PreparationError::InvalidName,
            PreparationError::DuplicateName,
            PreparationError::PoisonedOutput,
            PreparationError::RangeError,
            PreparationError::ArithmeticOverflow,
            PreparationError::AccessDenied,
        ]
        .into_iter()
        .any(|fault| error.message == poisoned_preparation_message(fault))
}

fn validate_name(name: &str) -> Result<(), PreparationError> {
    if name.is_empty() {
        Err(PreparationError::InvalidName)
    } else {
        Ok(())
    }
}

fn bounded_bytes(bytes: Vec<u8>, max_bytes: u64) -> Result<Vec<u8>, PreparationError> {
    if u64::try_from(bytes.len()).map_err(|_| PreparationError::RangeError)? > max_bytes {
        Err(PreparationError::RangeError)
    } else {
        Ok(bytes)
    }
}

/// Read-only typed preparation input with a retained opaque pilot fixture seam.
pub struct PrintPreparationInput {
    read_fixture: Box<ReadCallback>,
    input_reader: Option<Arc<dyn PrintPreparationRead + Send + Sync>>,
}

impl PrintPreparationInput {
    /// Adapt an owned typed reader; the backing retains all read-policy checks.
    pub fn from_host_input(reader: Arc<dyn PrintPreparationRead + Send + Sync>) -> Self {
        let mut input = Self::from_callback(|_, _| Err(PreparationError::AccessDenied));
        input.input_reader = Some(reader);
        input
    }

    /// Adapt typed reads and the controlled fixture import from the same resource.
    pub fn from_host_input_with_fixture(
        reader: Arc<dyn PrintPreparationRead + Send + Sync>,
        read_fixture: impl Fn(u64, u64) -> Result<Vec<u8>, PreparationError> + 'static,
    ) -> Self {
        let mut input = Self::from_callback(read_fixture);
        input.input_reader = Some(reader);
        input
    }

    /// Adapt a native host reader while preserving every transport error variant.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_host_reader(
        reader: Arc<dyn Fn(u64, u64) -> Result<Vec<u8>, PreparationTransportError> + Send + Sync>,
    ) -> Self {
        Self::from_callback(move |offset, max_bytes| reader(offset, max_bytes).map_err(Into::into))
    }

    /// Adapt the host's generated `read-fixture` import.
    pub fn from_callback(
        read_fixture: impl Fn(u64, u64) -> Result<Vec<u8>, PreparationError> + 'static,
    ) -> Self {
        // An aborting WASM panic cannot carry its payload in the trap. Transport
        // the actual panic detail over the ordinary logging import first.
        // Enable the identical hook path for isolated native SDK controls too.
        #[cfg(any(target_arch = "wasm32", feature = "test"))]
        {
            static PANIC_REPORTING: std::sync::Once = std::sync::Once::new();
            PANIC_REPORTING.call_once(|| {
                let previous = std::panic::take_hook();
                std::panic::set_hook(Box::new(move |info| {
                    previous(info);
                    crate::host::log_error(&info.to_string());
                }));
            });
        }
        Self {
            read_fixture: Box::new(read_fixture),
            input_reader: None,
        }
    }

    fn input_reader(
        &self,
    ) -> Result<&(dyn PrintPreparationRead + Send + Sync), PreparationInputError> {
        self.input_reader
            .as_deref()
            .ok_or(PreparationInputError::AccessDenied)
    }

    /// Return the committed schedule length.
    pub fn layer_count(&self) -> Result<u64, PreparationInputError> {
        self.input_reader()?.layer_count()
    }

    /// Read a bounded batch of committed schedule layers.
    pub fn layers(
        &self,
        offset: u64,
        max_items: u64,
    ) -> Result<Vec<PreparationLayer>, PreparationInputError> {
        self.input_reader()?.layers(offset, max_items)
    }

    /// Read committed regions for one schedule layer.
    pub fn regions(
        &self,
        layer_index: u32,
        offset: u64,
        max_items: u64,
    ) -> Result<Vec<PreparationRegion>, PreparationInputError> {
        self.input_reader()?.regions(layer_index, offset, max_items)
    }

    /// Read exact-key effective region configs for one schedule layer.
    pub fn region_configs(
        &self,
        layer_index: u32,
        offset: u64,
        max_items: u64,
    ) -> Result<Vec<PreparationRegionConfig>, PreparationInputError> {
        self.input_reader()?
            .region_configs(layer_index, offset, max_items)
    }

    /// Read optional object-specific surface classifications.
    pub fn surface_objects(
        &self,
        offset: u64,
        max_items: u64,
    ) -> Result<Option<Vec<PreparationSurfaceObject>>, PreparationInputError> {
        self.input_reader()?.surface_objects(offset, max_items)
    }

    /// Read optional object-specific surface geometry for one schedule layer.
    pub fn surface_layers(
        &self,
        layer_index: u32,
        offset: u64,
        max_items: u64,
    ) -> Result<Option<Vec<PreparationSurfaceLayer>>, PreparationInputError> {
        self.input_reader()?
            .surface_layers(layer_index, offset, max_items)
    }

    /// Read world-space mesh metadata and lossless paint projections.
    pub fn mesh_objects(
        &self,
        offset: u64,
        max_items: u64,
    ) -> Result<Vec<PreparationMeshObject>, PreparationInputError> {
        self.input_reader()?.mesh_objects(offset, max_items)
    }

    /// Read world-space mesh vertices in millimeters.
    pub fn mesh_vertices(
        &self,
        object_id: &str,
        offset: u64,
        max_items: u64,
    ) -> Result<Vec<slicer_ir::Point3>, PreparationInputError> {
        self.input_reader()?
            .mesh_vertices(object_id, offset, max_items)
    }

    /// Read mesh index triples in their original order.
    pub fn mesh_triangles(
        &self,
        object_id: &str,
        offset: u64,
        max_items: u64,
    ) -> Result<Vec<(u32, u32, u32)>, PreparationInputError> {
        self.input_reader()?
            .mesh_triangles(object_id, offset, max_items)
    }

    /// Execute world-space mesh queries, preserving caller request order.
    pub fn mesh_queries(
        &self,
        requests: &[PreparationMeshQuery],
    ) -> Result<Vec<PreparationMeshQueryResult>, PreparationInputError> {
        self.input_reader()?.mesh_queries(requests)
    }

    /// Read optional exact-key seam plans for one schedule layer.
    pub fn seams(
        &self,
        layer_index: u32,
        offset: u64,
        max_items: u64,
    ) -> Result<Option<Vec<PreparationSeam>>, PreparationInputError> {
        self.input_reader()?.seams(layer_index, offset, max_items)
    }

    /// Independently range candidates, each geometry map and family assignments.
    /// Settings and the feasible envelope remain invariant; never zip collections.
    pub fn support_analysis(
        &self,
        offset: u64,
        max_items: u64,
    ) -> Result<Option<PreparationSupportAnalysisBatch>, PreparationInputError> {
        self.input_reader()?.support_analysis(offset, max_items)
    }

    /// Range support geometry entries, retaining height/distance metadata.
    pub fn support_geometry(
        &self,
        offset: u64,
        max_items: u64,
    ) -> Result<Option<PreparationSupportGeometryBatch>, PreparationInputError> {
        self.input_reader()?.support_geometry(offset, max_items)
    }

    /// Range support-plan entries, retaining optional raft metadata.
    pub fn support_plan(
        &self,
        offset: u64,
        max_items: u64,
    ) -> Result<Option<PreparationSupportPlanBatch>, PreparationInputError> {
        self.input_reader()?.support_plan(offset, max_items)
    }

    /// Read the frozen owner's selected invocation targets for one layer.
    pub fn selected_targets(
        &self,
        layer_index: u32,
        offset: u64,
        max_items: u64,
    ) -> Result<Vec<PreparationSelectedTarget>, PreparationInputError> {
        self.input_reader()?
            .selected_targets(layer_index, offset, max_items)
    }

    /// Read at most `max_bytes` bytes from a fixture offset.
    ///
    /// The callback owns the fixture length: reject offsets beyond EOF first,
    /// then clip by remaining bytes. This facade checks the returned count;
    /// it never adds the requested maximum to the offset.
    pub fn read_fixture(&self, offset: u64, max_bytes: u64) -> Result<Vec<u8>, PreparationError> {
        bounded_bytes((self.read_fixture)(offset, max_bytes)?, max_bytes)
    }
}

/// Write-only staging of unique, complete named pieces.
pub struct PreparedPlanOutput {
    put: Box<PutCallback>,
    names: BTreeSet<String>,
    poisoned: bool,
    first_error: Option<PreparationError>,
    synthesized_completion: RefCell<Option<(u32, String, bool)>>,
}

impl PreparedPlanOutput {
    /// Adapt a native host writer, retaining the pilot's atomic staging checks.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_host_writer(
        writer: Arc<dyn Fn(&str, &[u8]) -> Result<(), PreparationTransportError> + Send + Sync>,
    ) -> Self {
        Self::from_callback(move |name, bytes| writer(name, bytes).map_err(Into::into))
    }

    /// Adapt the host's generated `put` import, without a module-owned codec.
    pub fn from_callback(
        put: impl FnMut(&str, &[u8]) -> Result<(), PreparationError> + 'static,
    ) -> Self {
        Self {
            put: Box::new(put),
            names: BTreeSet::new(),
            poisoned: false,
            first_error: None,
            synthesized_completion: RefCell::new(None),
        }
    }

    /// Stage a whole piece exactly once. Any failure poisons further writes.
    ///
    /// Publication remains the host's responsibility after preparation succeeds.
    pub fn put(&mut self, name: &str, bytes: &[u8]) -> Result<(), PreparationError> {
        if self.poisoned {
            return Err(PreparationError::PoisonedOutput);
        }
        let result = (|| {
            validate_name(name)?;
            if self.names.contains(name) {
                return Err(PreparationError::DuplicateName);
            }
            (self.put)(name, bytes)?;
            self.names.insert(name.to_owned());
            Ok(())
        })();
        if let Err(error) = result {
            self.poisoned = true;
            self.first_error = Some(error);
        }
        result
    }

    /// Complete a generated preparation call. Caught staging errors must never
    /// turn a successful module return into publishable output.
    #[doc(hidden)]
    pub fn complete(&self, result: Result<(), ModuleError>) -> Result<(), ModuleError> {
        result?;
        if let Some(error) = self.first_error {
            let detail = poisoned_preparation_message(error);
            let error = ModuleError::fatal(POISONED_PREPARATION_CODE, detail);
            *self.synthesized_completion.borrow_mut() =
                Some((error.code, error.message.clone(), error.fatal));
            Err(error)
        } else {
            Ok(())
        }
    }

    /// Expose the first local writer rejection to the native host finalizer.
    #[doc(hidden)]
    pub fn writer_fault(&self) -> Option<PreparationTransportError> {
        self.first_error.map(Into::into)
    }

    /// Exact fields of the latest synthesized completion error, if any.
    /// Native finalizers compare these with the result actually returned.
    #[doc(hidden)]
    pub fn synthesized_completion(&self) -> Option<(u32, String, bool)> {
        self.synthesized_completion.borrow().clone()
    }
}

/// Host/import adapter for one fresh `prepared-plan-view` resource.
///
/// Generated guests capture the resource returned by `plan-access.current` in
/// the `open` callback. The scope, not a module instance, owns that resource.
#[doc(hidden)]
pub struct PreparedPlanCallbacks {
    open: Box<OpenCallback>,
}

impl PreparedPlanCallbacks {
    /// Adapt the generated view's `open` import.
    pub fn new(
        open: impl Fn(&str) -> Result<Option<PreparedPieceCallbacks>, PreparationError> + 'static,
    ) -> Self {
        Self {
            open: Box::new(open),
        }
    }
}

/// Host/import adapter for one immutable serialized `prepared-piece` resource.
#[doc(hidden)]
pub struct PreparedPieceCallbacks {
    len: Box<dyn Fn() -> u64>,
    read: Box<ReadCallback>,
}

impl PreparedPieceCallbacks {
    /// Adapt the generated piece's infallible `len` and fallible `read` imports.
    pub fn new(
        len: impl Fn() -> u64 + 'static,
        read: impl Fn(u64, u64) -> Result<Vec<u8>, PreparationError> + 'static,
    ) -> Self {
        Self {
            len: Box::new(len),
            read: Box::new(read),
        }
    }
}

struct PlanScope {
    current: Box<CurrentCallback>,
    views: RefCell<Vec<Rc<PreparedPlanCallbacks>>>,
    pieces: RefCell<Vec<Rc<PreparedPieceCallbacks>>>,
}

thread_local! {
    static CURRENT_PLAN: RefCell<Option<Rc<PlanScope>>> = const { RefCell::new(None) };
}

struct PlanScopeGuard {
    previous: Option<Rc<PlanScope>>,
}

impl Drop for PlanScopeGuard {
    fn drop(&mut self) {
        // Release resources outside the TLS borrow, including during unwind.
        let expired = CURRENT_PLAN.with(|slot| slot.replace(self.previous.take()));
        drop(expired);
    }
}

/// Install an implicit-owner read facade for exactly one synchronous call.
///
/// The immutable host facade is queried lazily: `current` runs for every
/// [`PreparedPlanView::current`] acquisition, not at scope entry. For WASM,
/// generated glue supplies a callback invoking `plan-access.current` and
/// adapting the returned resource imports. No raw resource index is cached.
///
/// Nested bindings are restored on return and unwind. Returned wrappers hold
/// only weak references, so neither they nor their resources survive the call.
/// The binding and wrappers cannot be shared with another thread.
pub fn with_prepared_plan<R>(
    current: impl Fn() -> Result<PreparedPlanCallbacks, PreparationError> + 'static,
    body: impl FnOnce() -> R,
) -> R {
    let scope = Rc::new(PlanScope {
        current: Box::new(current),
        views: RefCell::new(Vec::new()),
        pieces: RefCell::new(Vec::new()),
    });
    let previous = CURRENT_PLAN.with(|slot| slot.replace(Some(scope)));
    let _guard = PlanScopeGuard { previous };
    body()
}

fn active_scope(scope: &Weak<PlanScope>) -> Result<Rc<PlanScope>, PreparationError> {
    let scope = scope.upgrade().ok_or(PreparationError::NotReady)?;
    let active = CURRENT_PLAN.with(|slot| {
        slot.borrow()
            .as_ref()
            .is_some_and(|current| Rc::ptr_eq(current, &scope))
    });
    if active {
        Ok(scope)
    } else {
        Err(PreparationError::AccessDenied)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn host_plan_callbacks(view: Arc<dyn PreparedPlanRead + Send + Sync>) -> PreparedPlanCallbacks {
    PreparedPlanCallbacks::new(move |name| {
        view.open(name)
            .map(|piece| {
                piece.map(|piece| {
                    let reader = Arc::clone(&piece);
                    PreparedPieceCallbacks::new(
                        move || piece.len(),
                        move |offset, max_bytes| reader.read(offset, max_bytes).map_err(Into::into),
                    )
                })
            })
            .map_err(Into::into)
    })
}

/// Bind a native host view for one call, restoring prior bindings even on unwind.
#[cfg(not(target_arch = "wasm32"))]
pub fn with_prepared_plan_view<R>(
    view: Arc<dyn PreparedPlanRead + Send + Sync>,
    call: impl FnOnce() -> R,
) -> R {
    with_prepared_plan(
        move || {
            view.check_ready().map_err(PreparationError::from)?;
            Ok(host_plan_callbacks(Arc::clone(&view)))
        },
        call,
    )
}

/// A fresh call-local, lazy view of the current owner's serialized plan.
pub struct PreparedPlanView {
    scope: Weak<PlanScope>,
    callbacks: Weak<PreparedPlanCallbacks>,
    host: Option<Arc<dyn PreparedPlanRead + Send + Sync>>,
}

impl PreparedPlanView {
    /// Wrap a native host view; resource validity remains enforced by the host.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_host_view(view: Arc<dyn PreparedPlanRead + Send + Sync>) -> Self {
        Self {
            scope: Weak::new(),
            callbacks: Weak::new(),
            host: Some(view),
        }
    }

    /// Require a named piece without interpreting its bytes.
    pub fn require(&self, name: &str) -> Result<PreparedPiece, ModuleError> {
        self.open(name)?.ok_or_else(|| {
            ModuleError::fatal(57401, format!("required preparation piece missing: {name}"))
        })
    }

    /// Acquire a fresh facade; missing preparation is an error, not an empty plan.
    pub fn current() -> Result<Self, PreparationError> {
        let scope = CURRENT_PLAN
            .with(|slot| slot.borrow().clone())
            .ok_or(PreparationError::NotReady)?;
        let callbacks = Rc::new((scope.current)()?);
        let view = Self {
            scope: Rc::downgrade(&scope),
            callbacks: Rc::downgrade(&callbacks),
            host: None,
        };
        scope.views.borrow_mut().push(callbacks);
        Ok(view)
    }

    /// Open a named piece without copying its serialized contents.
    pub fn open(&self, name: &str) -> Result<Option<PreparedPiece>, PreparationError> {
        if let Some(host) = &self.host {
            validate_name(name)?;
            return host
                .open(name)
                .map_err(PreparationError::from)
                .map(|piece| {
                    piece.map(|host| PreparedPiece {
                        scope: Weak::new(),
                        callbacks: Weak::new(),
                        host: Some(host),
                    })
                });
        }
        let scope = active_scope(&self.scope)?;
        validate_name(name)?;
        let callbacks = self.callbacks.upgrade().ok_or(PreparationError::NotReady)?;
        let Some(piece) = (callbacks.open)(name)? else {
            return Ok(None);
        };
        let piece = Rc::new(piece);
        let view = PreparedPiece {
            scope: Rc::downgrade(&scope),
            callbacks: Rc::downgrade(&piece),
            host: None,
        };
        scope.pieces.borrow_mut().push(piece);
        Ok(Some(view))
    }
}

/// A bounded reader for an opaque immutable piece, valid only in its call.
pub struct PreparedPiece {
    scope: Weak<PlanScope>,
    callbacks: Weak<PreparedPieceCallbacks>,
    host: Option<Arc<dyn PreparedPieceRead + Send + Sync>>,
}

impl PreparedPiece {
    fn callbacks(&self) -> Result<Rc<PreparedPieceCallbacks>, PreparationError> {
        active_scope(&self.scope)?;
        self.callbacks.upgrade().ok_or(PreparationError::NotReady)
    }

    /// Return the serialized length. Unlike WIT's infallible import, this
    /// wrapper also rejects attempts to use a resource after its scope ended.
    pub fn len(&self) -> Result<u64, PreparationError> {
        if let Some(host) = &self.host {
            return Ok(host.len());
        }
        Ok((self.callbacks()?.len)())
    }

    /// Return whether the serialized piece has no bytes.
    pub fn is_empty(&self) -> Result<bool, PreparationError> {
        Ok(self.len()? == 0)
    }

    /// Read up to `max_bytes`, truncating at EOF but rejecting offsets beyond it.
    pub fn read(&self, offset: u64, max_bytes: u64) -> Result<Vec<u8>, PreparationError> {
        if let Some(host) = &self.host {
            let len = host.len();
            if offset > len {
                return Err(PreparationError::RangeError);
            }
            let take = max_bytes.min(len - offset);
            return bounded_bytes(
                host.read(offset, max_bytes)
                    .map_err(PreparationError::from)?,
                take,
            );
        }
        let callbacks = self.callbacks()?;
        let len = (callbacks.len)();
        if offset > len {
            return Err(PreparationError::RangeError);
        }
        let remaining = len - offset;
        let take = max_bytes.min(remaining);
        bounded_bytes((callbacks.read)(offset, max_bytes)?, take)
    }
}
