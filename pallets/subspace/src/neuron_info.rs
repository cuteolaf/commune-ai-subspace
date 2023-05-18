use super::*;
use frame_support::storage::IterableStorageMap;
use frame_support::pallet_prelude::{Decode, Encode};
extern crate alloc;
use alloc::vec::Vec;
use codec::Compact;

#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug)]
pub struct NeuronSubnetInfo<T: Config> {
    key: T::AccountId,
    uid: Compact<u16>,
    active: bool,
    context: Vec<u8>,
    name: Vec<u8>,
    last_update: Compact<u64>,
    // Subnet Info
    stake: Vec<(T::AccountId, Compact<u64>)>, // map of key to stake on this neuron/key (includes delegations)
    rank: Compact<u16>,
    emission: Compact<u64>,
    incentive: Compact<u16>,
    dividends: Compact<u16>,
    weights: Vec<(Compact<u16>, Compact<u16>)>, // Vec of (uid, weight)
    bonds: Vec<(Compact<u16>, Compact<u16>)>, // Vec of (uid, bond)
    pruning_score: Compact<u16>,
}

impl<T: Config> Pallet<T> {
	pub fn get_neurons(netuid: u16) -> Vec<NeuronSubnetInfo<T>> {
        if !Self::if_subnet_exist(netuid) {
            return Vec::new();
        }

        let mut neurons = Vec::new();
        let n = Self::get_subnetwork_n(netuid);
        for uid in 0..n {
            let uid = uid;
            let netuid = netuid;

            let _neuron = Self::get_neuron(uid);
            let neuron;
            if _neuron.is_none() {
                break; // No more neurons
            } else {
                // No error, key was registered
                neuron = _neuron.expect("Neuron should exist");
            }

            neurons.push( neuron );
        }
        return neurons;
	}

    fn get_neuron( uid: u16) -> Option<NeuronSubnetInfo<T>> {
        
        let key = Self::get_key_for_uid(uid);
        let neuron_info = Self::get_neuron_info( &key.clone() );    
        let active = Self::get_active_for_uid( uid as u16 );
        let rank = Self::get_rank_for_uid(  uid as u16 );
        let emission = Self::get_emission_for_uid(  uid as u16 );
        let incentive = Self::get_incentive_for_uid(  uid as u16 );
        let dividends = Self::get_dividends_for_uid(  uid as u16 );
        let pruning_score = Self::get_pruning_score_for_uid(  uid as u16 );
        let last_update = Self::get_last_update_for_uid(  uid as u16 );
        let context = Self::get_context_for_uid(  uid as u16 );
        let name = Self::get_name_for_uid(  uid as u16 );

        let weights = <Weights<T>>::get( uid).iter()
            .filter_map(|(i, w)| if *w > 0 { Some((i.into(), w.into())) } else { None })
            .collect::<Vec<(Compact<u16>, Compact<u16>)>>();
        
        let bonds = <Bonds<T>>::get( uid).iter()
            .filter_map(|(i, b)| if *b > 0 { Some((i.into(), b.into())) } else { None })
            .collect::<Vec<(Compact<u16>, Compact<u16>)>>();
        
        let stake: Vec<(T::AccountId, Compact<u64>)> = < Stake<T> as IterableStorageMap<T::AccountId, u64> >::iter()
            .map(|(key, stake)| (key, stake.into()))
            .collect();

        let neuron = NeuronNetInfo {
            key: key.clone(),
            uid: uid.into(),
            active: active,
            stake: stake,
            rank: rank.into(),
            emission: emission.into(),
            incentive: incentive.into(),
            dividends: dividends.into(),
            last_update: last_update.into(),
            weights: weights,
            bonds: bonds,
            pruning_score: pruning_score.into(),
            context: context.clone(),
            name: name.clone()
        };
        
        return Some(neuron);
    }
}

