use std::{sync::Arc, time::Instant};

use eth2::types::{BlockId, CommitteeData, ProposerData};
use lighthouse_types::{BeaconBlock, Epoch, EthSpec, Hash256, Signature, Slot};
use tokio::sync::RwLock;
use types::{
    attestation::{AttestationModel, AttestationsModelWithId},
    block::{BlockExtendedModel, BlockExtendedModelWithId, BlockModel, BlockModelWithId},
    committee::{CommitteeModel, CommitteesModelWithId},
};

use crate::{beacon_node_client::BeaconNodeClient, errors::IndexerError};

#[derive(Debug, Clone)]
pub struct ConsolidatedBlock<E: EthSpec> {
    pub epoch: Epoch,
    pub slot: Slot,
    pub block: Option<BeaconBlock<E>>,
    pub block_root: Option<Hash256>,
    pub signature: Signature,
    pub status: BlockStatus,
    pub proposer: u64,
    pub sync_participation_rate: Option<f64>,
    pub committees: Arc<Vec<CommitteeData>>,
}

#[derive(Debug, Clone)]
pub enum BlockStatus {
    Scheduled = 0,
    Proposed = 1,
    Missed = 2,
    Orphaned = 3,
}

impl std::fmt::Display for BlockStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl<E: EthSpec> ConsolidatedBlock<E> {
    pub async fn new(
        epoch: Epoch,
        slot: Slot,
        proposer_duties_lock: Arc<RwLock<Option<Vec<ProposerData>>>>,
        committees: Arc<Vec<CommitteeData>>,
        client: BeaconNodeClient,
    ) -> Result<Self, IndexerError> {
        let start = Instant::now();
        let block = BlockId::Slot(slot);
        let block_response = client.get_block::<E>(block).await?;
        let duration = start.elapsed();
        log::trace!("get_block duration: {:?}", duration);
        let block_root = client.get_block_root(block).await.ok().map(|x| x.data.root);

        if let Some(block_response) = block_response {
            let (beacon_block, signature) = block_response.data.deconstruct();
            let start = Instant::now();
            let duration = start.elapsed();
            let sync_participation_rate = beacon_block
                .body()
                .sync_aggregate()
                .map(|sync_aggregate| {
                    sync_aggregate.num_set_bits() as f64
                        / sync_aggregate.sync_committee_bits.len() as f64
                })
                .ok();
            log::trace!("get_block_root duration: {:?}", duration);
            let consolidated_block = ConsolidatedBlock {
                epoch,
                slot: beacon_block.slot(),
                block: Some(beacon_block.clone()),
                block_root,
                signature,
                status: BlockStatus::Proposed,
                proposer: beacon_block.proposer_index(),
                sync_participation_rate,
                committees,
            };

            return Ok(consolidated_block);
        } else {
            let mut proposer_duties = proposer_duties_lock.read().await.clone();

            if proposer_duties.is_none() {
                let mut proposer_duties_writable = proposer_duties_lock.write().await;
                proposer_duties_writable
                    .replace(client.get_validator_duties_proposer(epoch).await?);
                proposer_duties = proposer_duties_writable.clone();
            }

            if let Some(proposer_duties) = proposer_duties {
                let status = match block_root {
                    Some(_) => BlockStatus::Orphaned,
                    None => BlockStatus::Missed,
                };
                for proposer in proposer_duties {
                    if proposer.slot == slot {
                        let consolidated_block = ConsolidatedBlock {
                            epoch,
                            slot: proposer.slot,
                            block: None,
                            block_root,
                            signature: Signature::empty(),
                            status,
                            proposer: proposer.validator_index,
                            sync_participation_rate: None,
                            committees,
                        };

                        return Ok(consolidated_block);
                    }
                }
            }
        }

        Err(IndexerError::ElementNotFound(block.to_string()))
    }

    pub fn get_attestations_count(&self) -> usize {
        match self.block.clone() {
            None => 0,
            Some(block) => block.body().attestations().len(),
        }
    }

    pub fn get_deposits_count(&self) -> usize {
        match self.block.clone() {
            None => 0,
            Some(block) => block.body().deposits().len(),
        }
    }

    pub fn get_voluntary_exits_count(&self) -> usize {
        match self.block.clone() {
            None => 0,
            Some(block) => block.body().voluntary_exits().len(),
        }
    }

    pub fn get_proposer_slashings_count(&self) -> usize {
        match self.block.clone() {
            None => 0,
            Some(block) => block.body().proposer_slashings().len(),
        }
    }

    pub fn get_attester_slashings_count(&self) -> usize {
        match self.block.clone() {
            None => 0,
            Some(block) => block.body().attester_slashings().len(),
        }
    }
}

impl<E: EthSpec> From<&ConsolidatedBlock<E>> for BlockModelWithId {
    fn from(value: &ConsolidatedBlock<E>) -> Self {
        let model = match &value.block {
            Some(block) => BlockModel {
                epoch: value.epoch.as_u64(),
                proposer_slashings_count: block.body().proposer_slashings().len(),
                attester_slashings_count: block.body().attester_slashings().len(),
                attestations_count: block.body().attestations().len(),
                deposits_count: block.body().deposits().len(),
                voluntary_exits_count: block.body().voluntary_exits().len(),
                proposer: value.proposer,
                status: value.status.to_string(),
            },
            None => BlockModel {
                epoch: value.epoch.as_u64(),
                proposer_slashings_count: 0,
                attester_slashings_count: 0,
                attestations_count: 0,
                deposits_count: 0,
                voluntary_exits_count: 0,
                proposer: value.proposer,
                status: value.status.to_string(),
            },
        };

        BlockModelWithId {
            id: value.slot.as_u64(),
            model,
        }
    }
}

impl<E: EthSpec> From<&ConsolidatedBlock<E>> for BlockExtendedModelWithId {
    fn from(value: &ConsolidatedBlock<E>) -> Self {
        let model = match &value.block {
            Some(block) => BlockExtendedModel {
                block_root: value.block_root.unwrap_or_default().as_bytes().to_vec(),
                parent_root: block.parent_root().as_bytes().to_vec(),
                state_root: block.state_root().as_bytes().to_vec(),
                randao_reveal: block.body().randao_reveal().to_string().as_bytes().to_vec(),
                signature: value.signature.to_string().as_bytes().to_vec(),
                graffiti: block.body().graffiti().to_string().as_bytes().to_vec(),
                graffiti_text: block.body().graffiti().to_string(),
                votes_count: 0,
                eth1data_deposit_root: block.body().eth1_data().deposit_root.as_bytes().to_vec(),
                eth1data_deposit_count: block.body().eth1_data().deposit_count,
                eth1data_block_hash: block.body().eth1_data().block_hash.as_bytes().to_vec(),
            },
            None => BlockExtendedModel {
                block_root: value.block_root.unwrap_or_default().as_bytes().to_vec(),
                parent_root: vec![],
                state_root: vec![],
                randao_reveal: vec![],
                signature: vec![],
                graffiti: vec![],
                graffiti_text: String::default(),
                votes_count: 0,
                eth1data_deposit_root: vec![],
                eth1data_deposit_count: 0,
                eth1data_block_hash: vec![],
            },
        };

        BlockExtendedModelWithId {
            id: value.slot.as_u64(),
            model,
        }
    }
}

impl<E: EthSpec> From<&ConsolidatedBlock<E>> for CommitteesModelWithId {
    fn from(value: &ConsolidatedBlock<E>) -> Self {
        let slot = value.slot;
        let r = value
            .committees
            .iter()
            .filter_map(|x| {
                if x.slot == slot {
                    let model = CommitteeModel {
                        index: x.index,
                        validators: x.validators.clone(),
                    };
                    Some(model)
                } else {
                    None
                }
            })
            .collect::<Vec<CommitteeModel>>();

        CommitteesModelWithId {
            id: slot.as_u64(),
            model: r,
        }
    }
}

impl<E: EthSpec> From<&ConsolidatedBlock<E>> for AttestationsModelWithId {
    fn from(value: &ConsolidatedBlock<E>) -> Self {
        let slot = value.slot;

        let r = match &value.block {
            Some(block) => block
                .body()
                .attestations()
                .iter()
                .map(AttestationModel::from)
                .collect::<Vec<AttestationModel>>(),
            None => Vec::new(),
        };

        AttestationsModelWithId {
            id: slot.as_u64(),
            model: r,
        }
    }
}
