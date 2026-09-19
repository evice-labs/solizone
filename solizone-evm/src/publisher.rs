use crate::block::SolizoneBlock;
use revm::primitives::B256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicationPayload {
    pub block_hash: B256,
    pub bytes: Vec<u8>,
}

#[allow(async_fn_in_trait)]
pub trait BlockPublisher {
    type Output;
    type Error;

    async fn publish(&self, payload: PublicationPayload) -> Result<Self::Output, Self::Error>;
}

pub async fn publish_block<P>(publisher: &P, block: &SolizoneBlock) -> Result<P::Output, P::Error>
where
    P: BlockPublisher,
{
    let payload = prepare_block_for_publication(block);

    publisher.publish(payload).await
}

pub fn prepare_block_for_publication(block: &SolizoneBlock) -> PublicationPayload {
    block.validate().expect("refusing to publish invalid block");

    PublicationPayload {
        block_hash: block.hash(),
        bytes: block.encode(),
    }
}
