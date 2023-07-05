use super::*;
use crate::math::*;
use frame_support::sp_std::vec;
use frame_support::inherent::Vec;
use substrate_fixed::types::{I32F32, I64F64, I96F32, I110F18};
use frame_support::storage::{IterableStorageMap, IterableStorageDoubleMap};

impl<T: Config> Pallet<T> { 

    pub fn block_step( ) {
        let block_number: u64 = Self::get_current_block_as_u64();
        log::debug!("block_step for block: {:?} ", block_number );
        for ( netuid, tempo )  in <Tempo<T> as IterableStorageMap<u16, u16>>::iter() {
            RegistrationsThisBlock::<T>::mutate(netuid,  |val| *val = 0 );
            let new_queued_emission : u64 = Self::calculate_network_emission( netuid );
            PendingEmission::<T>::mutate( netuid, | queued | *queued += new_queued_emission );
            log::debug!("netuid_i: {:?} queued_emission: +{:?} ", netuid, new_queued_emission );  
            if  (block_number + netuid as u64) % (tempo as u64) > 0 {
                continue;
            }
            let emission_to_drain:u64 = PendingEmission::<T>::get( netuid ).clone(); 
            Self::epoch( netuid, emission_to_drain );
            PendingEmission::<T>::insert( netuid, 0 );

        }
    }


    pub fn epoch( netuid: u16, token_emission: u64 ) {
        // Get subnetwork size.
        let n: u16 = Self::get_subnetwork_n( netuid );
        log::trace!( "n: {:?}", n );

        // Get current block.
        let current_block: u64 = Self::get_current_block_as_u64();
        log::trace!( "current_block: {:?}", current_block );


        // Block at registration vector (block when each module was most recently registered).
        let block_at_registration: Vec<u64> = Self::get_block_at_registration( netuid );
        log::trace!( "Block at registration: {:?}", &block_at_registration );

        // ===========
        // == Stake ==
        // ===========

        let mut keys: Vec<(u16, T::AccountId)> = Self::get_keys( netuid );
        // Access network stake as normalized vector.
        let mut stake_64: Vec<I64F64> = vec![ I64F64::from_num(0.0); n as usize ];
        for (uid_i, key) in keys.iter() {
            stake_64[ *uid_i as usize ] = I64F64::from_num( Self::get_stake_for_key(netuid, key ));
        }
        let mut stake: Vec<I32F32> = vec_fixed64_to_fixed32( stake_64 );



        // range: I32F32(0, 1)
        log::trace!( "S: {:?}", &stake );

        // Normalize active stake.
        inplace_normalize( &mut stake );
        log::trace!( "S (mask+norm): {:?}", &stake );

        // =============
        // == Weights ==
        // =============

        // Access network weights row normalized.
        let mut weights: Vec<Vec<(u16, I32F32)>> = Self::get_weights_sparse( netuid );
        log::trace!( "W (permit): {:?}", &weights );

        // Remove self-weight by masking diagonal.
        weights = mask_diag_sparse( &weights );
        log::trace!( "W (permit+diag): {:?}", &weights );

        // Normalize remaining weights.
        inplace_row_normalize_sparse( &mut weights );
        log::trace!( "W (mask+norm): {:?}", &weights );

        // =============================
        // ==  Incentive ==
        // =============================

        // Compute incentive: r_j = SUM(i) w_ij * s_i.
        let mut incentive: Vec<I32F32> = matmul_sparse( &weights, &stake, n );
        // If emission is zero, do an even split.
        if is_zero( &incentive ) { // no weights set
            for (uid_i, key) in keys.iter() {
                incentive[ *uid_i as usize ] = I32F32::from_num(1)/I32F32::from_num(incentive.len());
            }
        }
        inplace_normalize( &mut incentive );  // range: I32F32(0, 1)
        log::trace!( "Incentive: {:?}", &incentive );
        

        // Compute bonds delta column normalized.
        let mut bonds: Vec<Vec<(u16, I32F32)>> = row_hadamard_sparse( &weights, &stake ); // ΔB = W◦S (outdated W masked)
        log::trace!( "ΔB: {:?}", &bonds );

        // Normalize bonds delta.
        inplace_col_normalize_sparse( &mut bonds, n ); // sum_i b_ij = 1
        log::trace!( "ΔB (norm): {:?}", &bonds );
        
        // Compute dividends: d_i = SUM(j) b_ij * inc_j.
        // range: I32F32(0, 1)
        let mut dividends: Vec<I32F32> = matmul_transpose_sparse( &bonds, &incentive );
        inplace_normalize( &mut dividends );
        log::trace!( "D: {:?}", &dividends );


        // =================================
        // == Emission==
        // =================================

        // Computer emissions from dividends and incentives seperately.
        let incentive_emission: Vec<I32F32> = incentive.iter().map( |ii| (ii * I32F32::from_num(token_emission))/2 ).collect();
        let dividends_emission: Vec<I32F32> = incentive.iter().map( |ii| (ii * I32F32::from_num(token_emission))/2 ).collect();




        let emission: Vec<I32F32> = dividends_emission.iter().zip( incentive_emission.iter() ).map( |(d, i)| (d + i) ).collect();
        
        // Compute rao based emission scores. range: I96F32(0, token_emission)
        let emission: Vec<u64> = emission.iter().map( |e: &I32F32| e.to_num::<u64>() ).collect();
        log::trace!( "E: {:?}", &emission );

        // ===================
        // == Value storage ==
        // ===================
        Emission::<T>::insert( netuid, emission.clone() );
        let cloned_incentive: Vec<u16> = incentive.iter().map(|xi| fixed_proportion_to_u16(*xi)).collect::<Vec<u16>>();
        Incentive::<T>::insert( netuid, cloned_incentive );
        let cloned_dividends: Vec<u16> = dividends.iter().map(|xi| fixed_proportion_to_u16(*xi)).collect::<Vec<u16>>();
        Dividends::<T>::insert( netuid, cloned_dividends );

        // Emission tuples ( keys, u64 emission)
        for ( uid_i, key ) in keys.iter() {
            Self::increase_stake_on_account(netuid, &key.clone(), &key.clone() , incentive_emission[ *uid_i as usize ].to_num::<u64>() );
        }

    
        // Emission tuples ( keys, u64 emission)
        let mut result: Vec<(T::AccountId, u64)> = vec![]; 
        for ( uid_i, key ) in keys.iter() {
            // If no dividend emission, then we do not need to do anything.
            if dividends_emission[ *uid_i as usize ] > I32F32::from_num(0.0) {
                let stake_vector: Vec<(T::AccountId, I32F32)> = <Stake<T>>::get( netuid, &key.clone() ).iter().map( |(k, v)| (k.clone(), I32F32::from_num(v.clone())) ).collect();
                let total_stake: I32F32 = stake_vector.iter().map( |(_, stake)| stake ).sum();
                for ( staking_key, staking_value) in stake_vector.iter(){
                    // normalize stake vector
                    let staker_dividend_emission: I32F32 = (dividends_emission[ *uid_i as usize ] * ( staking_value ));
                    Self::increase_stake_on_account(netuid, &staking_key.clone(), &key.clone() , staker_dividend_emission.to_num::<u64>());
                }
            }
        }

    }

    pub fn get_normalized_stake( netuid:u16 ) -> Vec<I32F32> {
        let n: usize = Self::get_subnetwork_n( netuid ) as usize; 
        let mut stake_64: Vec<I64F64> = vec![ I64F64::from_num(0.0); n ]; 
        for module_uid in 0..n {
            stake_64[module_uid] = I64F64::from_num( Self::get_stake_for_uid( netuid, module_uid as u16 ) );
        }
        inplace_normalize_64( &mut stake_64 );
        let stake: Vec<I32F32> = vec_fixed64_to_fixed32( stake_64 );
        stake
    }

    pub fn get_block_at_registration( netuid:u16 ) -> Vec<u64> { 
        let n: usize = Self::get_subnetwork_n( netuid ) as usize;
        let mut block_at_registration: Vec<u64> = vec![ 0; n ];
        for module_uid in 0..n {
            if Keys::<T>::contains_key( netuid, module_uid as u16 ){
                block_at_registration[ module_uid ] = Self::get_module_block_at_registration( netuid, module_uid as u16 );
            }
        }
        block_at_registration
    }

    pub fn get_weights_sparse( netuid:u16 ) -> Vec<Vec<(u16, I32F32)>> { 
        let n: usize = Self::get_subnetwork_n( netuid ) as usize; 
        let mut weights: Vec<Vec<(u16, I32F32)>> = vec![ vec![]; n ]; 
        for ( uid_i, weights_i ) in < Weights<T> as IterableStorageDoubleMap<u16 ,u16, Vec<(u16, u16)> >>::iter_prefix( netuid ) {
            for (uid_j, weight_ij) in weights_i.iter() { 
                weights [ uid_i as usize ].push( ( *uid_j, u16_proportion_to_fixed( *weight_ij ) ));
            }
        }
        weights
    } 

    pub fn get_weights( netuid:u16 ) -> Vec<Vec<I32F32>> { 
        let n: usize = Self::get_subnetwork_n( netuid ) as usize; 
        let mut weights: Vec<Vec<I32F32>> = vec![ vec![ I32F32::from_num(0.0); n ]; n ]; 
        for ( uid_i, weights_i ) in < Weights<T> as IterableStorageDoubleMap<u16,u16, Vec<(u16, u16)> >>::iter_prefix( netuid ) {
            for (uid_j, weight_ij) in weights_i.iter() { 
                weights [ uid_i as usize ] [ *uid_j as usize ] = u16_proportion_to_fixed(  *weight_ij );
            }
        }
        weights
    }


    pub fn blocks_until_next_epoch( netuid: u16, tempo: u16, block_number: u64 ) -> u64 { 
        if tempo == 0 { return 10 } // Special case: epoch = 0, the network never runs.
        // epoch | netuid | # first epoch block
        //   1        0               0
        //   1        1               1
        //   2        0               1
        //   2        1               0
        //   100      0              99
        //   100      1              98
        return tempo as u64 - ( block_number + netuid as u64 + 1 ) % ( tempo as u64 + 1 )
    }

 


}