use lb_zone_sdk::{
    node_types::Inscription,
    sequencer::{Error as SequencerError, PublishResult, SequencerCheckpoint, SequencerClient},
};

use revm::primitives::B256;

use crate::publisher::{BlockPublisher, PublicationPayload};

#[derive(Debug)]
pub enum LogosPublisherError {
    InscriptionTooLarge { size: usize },
    Sequencer(SequencerError),
}

#[derive(Debug)]
pub struct LogosPublicationReceipt {
    pub block_hash: B256,
    pub publish_result: PublishResult,
    pub checkpoint: SequencerCheckpoint,
}

pub struct LogosPublisher {
    client: SequencerClient,
}

impl LogosPublisher {
    pub fn new(client: SequencerClient) -> Self {
        Self { client }
    }
}

impl BlockPublisher for LogosPublisher {
    type Output = LogosPublicationReceipt;
    type Error = LogosPublisherError;

    async fn publish(&self, payload: PublicationPayload) -> Result<Self::Output, Self::Error> {
        let size = payload.bytes.len();

        let inscription = Inscription::try_from(payload.bytes)
            .map_err(|_| LogosPublisherError::InscriptionTooLarge { size })?;

        let (publish_result, checkpoint) = self
            .client
            .publish(inscription)
            .await
            .map_err(LogosPublisherError::Sequencer)?;

        Ok(LogosPublicationReceipt {
            block_hash: payload.block_hash,
            publish_result,
            checkpoint,
        })
    }
}
