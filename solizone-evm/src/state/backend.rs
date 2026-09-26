use super::StateSnapshot;

/// Persistence boundary for Solizone EVM state.
///
/// Execution works with `MemoryState`.
/// Backends are responsible only for storing and retrieving snapshots.
pub trait StateBackend {
    type Error;

    fn save(&self, snapshot: &StateSnapshot) -> Result<(), Self::Error>;

    fn load(&self) -> Result<Option<StateSnapshot>, Self::Error>;
}
