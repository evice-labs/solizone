use super::SolizoneCheckpoint;

pub trait CheckpointBackend {
    type Error;

    fn save(&self, checkpoint: &SolizoneCheckpoint) -> Result<(), Self::Error>;

    fn load(&self) -> Result<Option<SolizoneCheckpoint>, Self::Error>;
}
