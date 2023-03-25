use super::*;
use frame_support::storage::IterableStorageMap;
use frame_support::pallet_prelude::{Decode, Encode};
extern crate alloc;
use alloc::vec::Vec;
use codec::Compact;

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug)]
pub struct NeuronInfo<T: Config> {
    key: T::AccountId,
    uid: Compact<u16>,
    netuid: Compact<u16>,
    active: bool,
    stake: Vec<(T::AccountId, Compact<u64>)>, // map of key to stake on this neuron/key (includes delegations)
    rank: Compact<u16>,
    emission: Compact<u64>,
    incentive: Compact<u16>,
    dividends: Compact<u16>,
    last_update: Compact<u64>,
    weights: Vec<(Compact<u16>, Compact<u16>)>, // Vec of (uid, weight)
    bonds: Vec<(Compact<u16>, Compact<u16>)>, // Vec of (uid, bond)
    pruning_score: Compact<u16>,
}



impl<T: Config> Pallet<T> {
	pub fn get_neurons(netuid: u16) -> Vec<NeuronInfo<T>> {
        if !Self::if_subnet_exist(netuid) {
            return Vec::new();
        }

        let mut neurons = Vec::new();
        let n = Self::get_subnetwork_n(netuid);
        for uid in 0..n {
            let uid = uid;
            let netuid = netuid;

            let _neuron = Self::get_neuron_subnet_exists(netuid, uid);
            let neuron;
            if _neuron.is_none() {
                break; // No more neurons
            } else {
                // No error, key was registered
                neuron = _neuron.expect("Neuron should exist");
            }

            neurons.push( neuron );
        }
        neurons
	}

    fn get_neuron_subnet_exists(netuid: u16, uid: u16) -> Option<NeuronInfo<T>> {
        let key = Self::get_key_for_net_and_uid(netuid, uid);
        let axon_info = Self::get_axon_info( netuid, &key.clone() );


                
        let active = Self::get_active_for_uid( netuid, uid as u16 );
        let rank = Self::get_rank_for_uid( netuid, uid as u16 );
        let emission = Self::get_emission_for_uid( netuid, uid as u16 );
        let incentive = Self::get_incentive_for_uid( netuid, uid as u16 );
        let dividends = Self::get_dividends_for_uid( netuid, uid as u16 );
        let pruning_score = Self::get_pruning_score_for_uid( netuid, uid as u16 );
        let last_update = Self::get_last_update_for_uid( netuid, uid as u16 );

        let weights = <Weights<T>>::get(netuid, uid).iter()
            .filter_map(|(i, w)| if *w > 0 { Some((i.into(), w.into())) } else { None })
            .collect::<Vec<(Compact<u16>, Compact<u16>)>>();
        
        let bonds = <Bonds<T>>::get(netuid, uid).iter()
            .filter_map(|(i, b)| if *b > 0 { Some((i.into(), b.into())) } else { None })
            .collect::<Vec<(Compact<u16>, Compact<u16>)>>();
        
        let stake: Vec<(T::AccountId, Compact<u64>)> = < Stake<T> as IterableStorageMap<T::AccountId, u64> >::iter()
            .map(|(key, stake)| (key, stake.into()))
            .collect();

        let neuron = NeuronInfo {
            key: key.clone(),
            uid: uid.into(),
            netuid: netuid.into(),
            active,
            stake,
            rank: rank.into(),
            emission: emission.into(),
            incentive: incentive.into(),
            dividends: dividends.into(),
            last_update: last_update.into(),
            weights,
            bonds,
            pruning_score: pruning_score.into()
        };
        
        return Some(neuron);
    }

    pub fn get_neuron(netuid: u16, uid: u16) -> Option<NeuronInfo<T>> {
        if !Self::if_subnet_exist(netuid) {
            return None;
        }

        let neuron = Self::get_neuron_subnet_exists(netuid, uid);
        neuron
	}



}

