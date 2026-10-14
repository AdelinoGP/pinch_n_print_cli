//! Call-local, serialized access to host-owned layer preparation resources.
//!
//! The callback constructors adapt generated WIT imports without interpreting
//! the bytes. Native hosts use the same seam; no plan codec belongs in the SDK.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::{Rc, Weak};

use crate::error::ModuleError;

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
    /// A piece name is empty or contains a control character.
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

fn validate_name(name: &str) -> Result<(), PreparationError> {
    if name.is_empty() || name.chars().any(char::is_control) {
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

/// Read-only pilot fixture transport. Bytes remain opaque to the SDK.
pub struct PrintPreparationInput {
    read_fixture: Box<ReadCallback>,
}

impl PrintPreparationInput {
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
        }
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
}

impl PreparedPlanOutput {
    /// Adapt the host's generated `put` import, without a module-owned codec.
    pub fn from_callback(
        put: impl FnMut(&str, &[u8]) -> Result<(), PreparationError> + 'static,
    ) -> Self {
        Self {
            put: Box::new(put),
            names: BTreeSet::new(),
            poisoned: false,
            first_error: None,
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
            let detail = format!("layer preparation: poisoned-output; original put error: {error}");
            Err(ModuleError::fatal(POISONED_PREPARATION_CODE, detail))
        } else {
            Ok(())
        }
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

/// A fresh call-local, lazy view of the current owner's serialized plan.
pub struct PreparedPlanView {
    scope: Weak<PlanScope>,
    callbacks: Weak<PreparedPlanCallbacks>,
}

impl PreparedPlanView {
    /// Acquire a fresh facade; missing preparation is an error, not an empty plan.
    pub fn current() -> Result<Self, PreparationError> {
        let scope = CURRENT_PLAN
            .with(|slot| slot.borrow().clone())
            .ok_or(PreparationError::NotReady)?;
        let callbacks = Rc::new((scope.current)()?);
        let view = Self {
            scope: Rc::downgrade(&scope),
            callbacks: Rc::downgrade(&callbacks),
        };
        scope.views.borrow_mut().push(callbacks);
        Ok(view)
    }

    /// Open a named piece without copying its serialized contents.
    pub fn open(&self, name: &str) -> Result<Option<PreparedPiece>, PreparationError> {
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
        };
        scope.pieces.borrow_mut().push(piece);
        Ok(Some(view))
    }
}

/// A bounded reader for an opaque immutable piece, valid only in its call.
pub struct PreparedPiece {
    scope: Weak<PlanScope>,
    callbacks: Weak<PreparedPieceCallbacks>,
}

impl PreparedPiece {
    fn callbacks(&self) -> Result<Rc<PreparedPieceCallbacks>, PreparationError> {
        active_scope(&self.scope)?;
        self.callbacks.upgrade().ok_or(PreparationError::NotReady)
    }

    /// Return the serialized length. Unlike WIT's infallible import, this
    /// wrapper also rejects attempts to use a resource after its scope ended.
    pub fn len(&self) -> Result<u64, PreparationError> {
        Ok((self.callbacks()?.len)())
    }

    /// Return whether the serialized piece has no bytes.
    pub fn is_empty(&self) -> Result<bool, PreparationError> {
        Ok(self.len()? == 0)
    }

    /// Read up to `max_bytes`, truncating at EOF but rejecting offsets beyond it.
    pub fn read(&self, offset: u64, max_bytes: u64) -> Result<Vec<u8>, PreparationError> {
        let callbacks = self.callbacks()?;
        let len = (callbacks.len)();
        if offset > len {
            return Err(PreparationError::RangeError);
        }
        let remaining = len - offset;
        let take = max_bytes.min(remaining);
        bounded_bytes((callbacks.read)(offset, take)?, take)
    }
}
