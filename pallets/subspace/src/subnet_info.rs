use super::*;
use frame_support::IterableStorageDoubleMap;
use frame_support::storage::IterableStorageMap;
use frame_support::pallet_prelude::{Decode, Encode};
extern crate alloc;
use alloc::vec::Vec;
use codec::Compact;

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug)]
pub struct NetInfo {
    immunity_period: Compact<u16>,
    min_allowed_weights: Compact<u16>,
    max_weights_limit: Compact<u16>,
    max_allowed_uids: Compact<u16>,
    blocks_since_last_step: Compact<u64>,
    tempo: Compact<u16>,
    emission_values: Compact<u64>,
}

impl<T: Config> Pallet<T> {
	pub fn get_net_info() -> Option<NetInfo> {
        if !Self::if_subnet_exist() {
            return None;
        }

        let immunity_period = Self::get_immunity_period();
        let min_allowed_weights = Self::get_min_allowed_weights();
        let max_weights_limit = Self::get_max_weight_limit();
        let max_allowed_uids = Self::get_max_allowed_uids();
        let blocks_since_last_step = Self::get_blocks_since_last_step();
        let tempo = Self::get_tempo();
        let emission_values = Self::get_emission_value();



        return Some(NetInfo {
            immunity_period: immunity_period.into(),
            netuid: netuid.into(),
            min_allowed_weights: min_allowed_weights.into(),
            max_weights_limit: max_weights_limit.into(),
            max_allowed_uids: max_allowed_uids.into(),
            blocks_since_last_step: blocks_since_last_step.into(),
            tempo: tempo.into(),
            emission_values: emission_values.into(),
        })
	}

}

