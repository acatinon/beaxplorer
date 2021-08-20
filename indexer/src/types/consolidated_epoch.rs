use std::ops::Div;

use db::models::EpochModel;
use db::schema::epochs;
use db::{PgConnection, RunQueryDsl};
use eth2::lighthouse::GlobalValidatorInclusionData;
use shared::utils::convert::IntoClampedI64;
use types::{Epoch, EthSpec};

use crate::errors::IndexerError;
use crate::persistable::Persistable;

use super::consolidated_block::ConsolidatedBlock;
use super::consolidated_validator::ConsolidatedValidator;

#[derive(Debug)]
pub struct ConsolidatedEpoch<E: EthSpec> {
    pub epoch: Epoch,
    pub blocks: Vec<ConsolidatedBlock<E>>,
    pub validators: Vec<ConsolidatedValidator>,
    pub validator_inclusion: GlobalValidatorInclusionData,
}

impl<E: EthSpec> ConsolidatedEpoch<E> {
    pub fn as_model(&self) -> Result<EpochModel, IndexerError> {
        let epoch = self.epoch.as_u64().into_i64();
        let total_validator_balance: i64 = self.get_total_validator_balance().into_i64();
        let eligible_ether = self
            .validator_inclusion
            .previous_epoch_active_gwei
            .into_i64();
        let voted_ether = self
            .validator_inclusion
            .previous_epoch_target_attesting_gwei
            .into_i64();
        let global_participation_rate = (self
            .validator_inclusion
            .previous_epoch_target_attesting_gwei as f64)
            .div(self.validator_inclusion.previous_epoch_active_gwei as f64);

        let e = EpochModel {
            epoch,
            blocks_count: self.blocks.len() as i32,
            proposer_slashings_count: self.get_proposer_slashings_count() as i32,
            attester_slashings_count: self.get_attester_slashings_count() as i32,
            attestations_count: self.get_attestations_count() as i32,
            deposits_count: self.get_deposits_count() as i32,
            voluntary_exits_count: self.get_voluntary_exits_count() as i32,
            validators_count: self.validators.len() as i32,
            average_validator_balance: total_validator_balance.div(self.validators.len() as i64),
            total_validator_balance: total_validator_balance,
            finalized: Some(global_participation_rate >= 2f64 / 3f64),
            eligible_ether: Some(eligible_ether),
            global_participation_rate: Some(global_participation_rate),
            voted_ether: Some(voted_ether),
        };

        Ok(e)
    }

    pub fn get_attestations_count(&self) -> usize {
        self.blocks.iter().map(|b| b.get_attestations_count()).sum()
    }

    pub fn get_deposits_count(&self) -> usize {
        self.blocks.iter().map(|b| b.get_deposits_count()).sum()
    }

    pub fn get_voluntary_exits_count(&self) -> usize {
        self.blocks
            .iter()
            .map(|b| b.get_voluntary_exits_count())
            .sum()
    }

    pub fn get_proposer_slashings_count(&self) -> usize {
        self.blocks
            .iter()
            .map(|b| b.get_proposer_slashings_count())
            .sum()
    }

    pub fn get_attester_slashings_count(&self) -> usize {
        self.blocks
            .iter()
            .map(|b| b.get_attester_slashings_count())
            .sum()
    }

    pub fn get_total_validator_balance(&self) -> u64 {
        self.validators
            .iter()
            .map(|v| v.0.validator.effective_balance)
            .sum()
    }
}

impl<E: EthSpec> Persistable for ConsolidatedEpoch<E> {
    fn persist(&self, db_connection: &PgConnection) -> Result<(), IndexerError> {
        let epoch_model = self.as_model()?;

        db::insert_into(epochs::table)
            .values(epoch_model)
            .execute(db_connection)?;

        for consolidated_block in &self.blocks {
            consolidated_block.persist(db_connection)?;
        }

        Ok(())
    }
}
