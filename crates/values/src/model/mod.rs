use bincode::{Decode, Encode};
use std::time::SystemTime;

use thiserror::Error;

use crate::model::objects::ObjAttr;
use crate::model::verbs::Vid;
use crate::var::error::Error;
use crate::var::objid::Objid;

pub mod defset;
pub mod r#match;
pub mod objects;
pub mod objset;
pub mod permissions;
pub mod propdef;
pub mod props;
pub mod verb_info;
pub mod verbdef;
pub mod verbs;
pub mod world_state;

/// The result code from a commit/complete operation on the world's state.
#[derive(Debug, Eq, PartialEq)]
pub enum CommitResult {
    Success, // Value was committed
    ConflictRetry, // Value was not committed due to conflict, caller should abort and retry tx
             // TODO: timeout/task-too-long/error?
}

/// Errors related to the world state and operations on it.
#[derive(Error, Debug, Eq, PartialEq, Clone, Decode, Encode)]
pub enum WorldStateError {
    #[error("Object not found: {0}")]
    ObjectNotFound(Objid),
    #[error("Object already exists: {0}")]
    ObjectAlreadyExists(Objid),
    #[error("Could not set/get object attribute; {0} on {1}")]
    ObjectAttributeError(ObjAttr, Objid),
    #[error("Recursive move detected: {0} -> {1}")]
    RecursiveMove(Objid, Objid),

    #[error("Object permission denied")]
    ObjectPermissionDenied,

    #[error("Property not found: {0}.{1}")]
    PropertyNotFound(Objid, String),
    #[error("Property permission denied")]
    PropertyPermissionDenied,
    #[error("Property definition not found: {0}.{1}")]
    PropertyDefinitionNotFound(Objid, String),
    #[error("Duplicate property definition: {0}.{1}")]
    DuplicatePropertyDefinition(Objid, String),
    #[error("Property type mismatch")]
    PropertyTypeMismatch,

    #[error("Verb not found: {0}:{1}")]
    VerbNotFound(Objid, String),
    #[error("Verb definition not {0:?}")]
    InvalidVerb(Vid),

    #[error("Invalid verb, decode error: {0}:{1}")]
    VerbDecodeError(Objid, String),
    #[error("Verb permission denied")]
    VerbPermissionDenied,
    #[error("Verb already exists: {0}:{1}")]
    DuplicateVerb(Objid, String),

    #[error("Failed object match: {0}")]
    FailedMatch(String),
    #[error("Ambiguous object match: {0}")]
    AmbiguousMatch(String),

    // Catch-alls for system level object DB errors.
    #[error("DB communications/internal error: {0}")]
    DatabaseError(String),
}

/// Translations from WorldStateError to MOO error codes.
impl WorldStateError {
    pub fn to_error_code(&self) -> Error {
        match self {
            Self::ObjectNotFound(_) => Error::E_INVIND,
            Self::ObjectPermissionDenied => Error::E_PERM,
            Self::RecursiveMove(_, _) => Error::E_RECMOVE,
            Self::VerbNotFound(_, _) => Error::E_VERBNF,
            Self::VerbPermissionDenied => Error::E_PERM,
            Self::InvalidVerb(_) => Error::E_VERBNF,
            Self::DuplicateVerb(_, _) => Error::E_INVARG,
            Self::PropertyNotFound(_, _) => Error::E_PROPNF,
            Self::PropertyPermissionDenied => Error::E_PERM,
            Self::PropertyDefinitionNotFound(_, _) => Error::E_PROPNF,
            Self::DuplicatePropertyDefinition(_, _) => Error::E_INVARG,
            Self::PropertyTypeMismatch => Error::E_TYPE,
            _ => {
                panic!("Unhandled error code: {:?}", self);
            }
        }
    }
}

impl From<WorldStateError> for Error {
    fn from(val: WorldStateError) -> Self {
        val.to_error_code()
    }
}

pub fn world_state_err(err: WorldStateError) -> Error {
    err.into()
}

/// A narrative event is a record of something that happened in the world, and is what `bf_notify`
/// or similar ultimately create.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct NarrativeEvent {
    /// When the event happened, in the server's system time.
    timestamp: SystemTime,
    /// The object that authored or caused the event.
    author: Objid,
    /// The event itself.
    pub event: Event,
}

/// Types of events we can send to the session.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub enum Event {
    /// The typical "something happened" descriptive event.
    TextNotify(String),
    // TODO: other events that might happen here would be things like (local) "object moved" or "object
    //   created."
}

impl NarrativeEvent {
    #[must_use]
    pub fn notify_text(author: Objid, event: String) -> Self {
        Self {
            timestamp: SystemTime::now(),
            author,
            event: Event::TextNotify(event),
        }
    }

    #[must_use]
    pub fn timestamp(&self) -> SystemTime {
        self.timestamp
    }
    #[must_use]
    pub fn author(&self) -> Objid {
        self.author
    }
    #[must_use]
    pub fn event(&self) -> Event {
        self.event.clone()
    }
}

/// Errors related to command matching.
#[derive(Debug, Error, Clone, Decode, Encode, Eq, PartialEq)]
pub enum CommandError {
    #[error("Could not parse command")]
    CouldNotParseCommand,
    #[error("Could not find object match for command")]
    NoObjectMatch,
    #[error("Could not find verb match for command")]
    NoCommandMatch,
    #[error("Could not start transaction due to database error: {0}")]
    DatabaseError(WorldStateError),
    #[error("Permission denied")]
    PermissionDenied,
}
