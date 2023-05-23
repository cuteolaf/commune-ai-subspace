use super::*;
use crate::math::*;
use frame_support::sp_std::vec;
use frame_support::inherent::Vec;
use substrate_fixed::types::{I32F32, I64F64, I96F32, I110F18};
use frame_support::storage::IterableStorageDoubleMap;
use frame_support::storage::IterableStorageMap;

impl<T: Config> Pallet<T> {




    // Calculates reward  values,then updates rank, incentive, dividend, pruning_score, emission and bonds, and 
    // returns the emissions for uids/keys in a given `netuid`.
    //
    // # Args:
    // 	* 'token_emission': ( u64 ):
    //         - The total emission for the epoch.
    //
    pub fn epoch( token_emission: u64 ) -> Vec<(T::AccountId, u64)> {
        // Get subnetwork size.
        let n: u16 = Self::get_n();
        log::trace!( "n: {:?}", n );

        // ======================
        // == Active & updated ==
        // ======================

        // Get current block.
        let current_block: u64 = Self::get_current_block_as_u64();
        log::trace!( "current_block: {:?}", current_block );

        // Get activity cutoff.
        let activity_cutoff: u64 = Self::get_activity_cutoff() as u64;
        log::trace!( "activity_cutoff: {:?}", activity_cutoff );

        // Last update vector.
        let last_update: Vec<u64> = Self::get_last_update();
        log::trace!( "Last update: {:?}", &last_update );

        // Inactive mask.
        let inactive: Vec<bool> = last_update.iter().map(| updated | *updated + activity_cutoff < current_block ).collect();
        log::trace!( "Inactive: {:?}", inactive.clone() );

        // Logical negation of inactive.
        let active: Vec<bool> = inactive.iter().map(|&b| !b).collect();

        // Block at registration vector (block when each neuron was most recently registered).
        let block_at_registration: Vec<u64> = Self::get_block_at_registration();
        log::trace!( "Block at registration: {:?}", &block_at_registration );

        // ===========
        // == Stake ==
        // ===========

        let mut keys: Vec<(u16, T::AccountId)> = vec![];
        for ( uid_i, key ) in < Keys<T> as IterableStorageMap<u16, T::AccountId >>::iter_prefix() {
            keys.push( (uid_i, key) ); 
        }
        log::trace!( "keys: {:?}", &keys );

        // Access network stake as normalized vector.
        let mut stake_64: Vec<I64F64> = vec![ I64F64::from_num(0.0); n as usize ];
        for (uid_i, key) in keys.iter() {
            stake_64[ *uid_i as usize ] = I64F64::from_num( Self::get_total_stake_for_key( key ) );
        }
        inplace_normalize_64( &mut stake_64 );
        let stake: Vec<I32F32> = vec_fixed64_to_fixed32( stake_64 );
        // range: I32F32(0, 1)
        log::trace!( "S: {:?}", &stake );

        // Remove inactive stake.
        let mut active_stake: Vec<I32F32> = stake.clone();
        inplace_mask_vector( &inactive, &mut active_stake );
        log::trace!( "S (mask): {:?}", &active_stake );

        // Normalize active stake.
        inplace_normalize( &mut active_stake );
        log::trace!( "S (mask+norm): {:?}", &active_stake );

        // =============
        // == Weights ==
        // =============

        // Access network weights row normalized.
        let mut weights: Vec<Vec<(u16, I32F32)>> = Self::get_weights_sparse();

        // log::trace!( "W (permit): {:?}", &weights );

        // Remove self-weight by masking diagonal.
        weights = mask_diag_sparse( &weights );
        // log::trace!( "W (permit+diag): {:?}", &weights );

        // Remove weights referring to deregistered neurons.
        weights = vec_mask_sparse_matrix( &weights, &last_update, &block_at_registration, &| updated, registered | updated <= registered );
        // log::trace!( "W (permit+diag+outdate): {:?}", &weights );

        // Normalize remaining weights.
        inplace_row_normalize_sparse( &mut weights );
        // log::trace!( "W (mask+norm): {:?}", &weights );

        // =============================
        // == Ranks, Incentive ==
        // =============================

        // Compute ranks: r_j = SUM(i) w_ij * s_i.
        let mut incentive: Vec<I32F32> = matmul_sparse( &weights, &active_stake, n );
        // log::trace!( "R (after): {:?}", &ranks );

        inplace_normalize( &mut incentive );  // range: I32F32(0, 1)
        let incentive: Vec<I32F32> = ranks.clone();
        log::trace!( "I (=R): {:?}", &incentive );

        // =========================
        // == Bonds and Dividends ==
        // =========================

        // Access network bonds column normalized.
        let mut bonds: Vec<Vec<(u16, I32F32)>> = Self::get_bonds();
        // log::trace!( "B: {:?}", &bonds );
        
        // Remove bonds referring to deregistered neurons.
        bonds = vec_mask_sparse_matrix( &bonds, &last_update, &block_at_registration, &| updated, registered | updated <= registered );
        // log::trace!( "B (outdatedmask): {:?}", &bonds );

        // Normalize remaining bonds: sum_i b_ij = 1.
        inplace_col_normalize_sparse( &mut bonds, n );
        // log::trace!( "B (mask+norm): {:?}", &bonds );

        // Compute bonds delta column normalized.
        let mut bonds_delta: Vec<Vec<(u16, I32F32)>> = row_hadamard_sparse( &weights, &active_stake ); // ΔB = W◦S (outdated W masked)
        // log::trace!( "ΔB: {:?}", &bonds_delta );

        // Normalize bonds delta.
        inplace_col_normalize_sparse( &mut bonds_delta, n ); // sum_i b_ij = 1
        // log::trace!( "ΔB (norm): {:?}", &bonds_delta );
    

        // Compute dividends: d_i = SUM(j) b_ij * inc_j.
        // range: I32F32(0, 1)
        let mut dividends: Vec<I32F32> = matmul_transpose_sparse( &bonds_delta, &incentive );
        inplace_normalize( &mut dividends );
        log::trace!( "D: {:?}", &dividends );

        // =================================
        // == Emission and Pruning scores ==
        // =================================

        // Compute normalized emission scores. range: I32F32(0, 1)
        let mut normalized_emission: Vec<I32F32> = incentive.iter().zip( dividends.clone() ).map( |(ii, di)| ii + di ).collect();
        inplace_normalize( &mut normalized_emission );

        // If emission is zero, replace emission with normalized stake.
        if is_zero( &normalized_emission ) { // no weights set | outdated weights | self_weights
            if is_zero( &active_stake ) { // no active stake
                normalized_emission = stake.clone(); // do not mask inactive, assumes stake is normalized
            }
            else {
                normalized_emission = active_stake.clone(); // emission proportional to inactive-masked normalized stake
            }
        }
        
        // Compute rao based emission scores. range: I96F32(0, token_emission)
        let float_token_emission: I96F32 = I96F32::from_num( token_emission );
        let emission: Vec<I96F32> = normalized_emission.iter().map( |e: &I32F32| I96F32::from_num( *e ) * float_token_emission ).collect();
        let emission: Vec<u64> = emission.iter().map( |e: &I96F32| e.to_num::<u64>() ).collect();
        log::trace!( "nE: {:?}", &normalized_emission );
        log::trace!( "E: {:?}", &emission );

        // Set pruning scores.
        log::trace!( "P: {:?}", &pruning_scores );

        // ===================
        // == Value storage ==
        // ===================
        let cloned_emission: Vec<u64> = emission.clone();
        let cloned_incentive: Vec<u16> = incentive.iter().map(|xi| fixed_proportion_to_u16(*xi)).collect::<Vec<u16>>();
        let cloned_dividends: Vec<u16> = dividends.iter().map(|xi| fixed_proportion_to_u16(*xi)).collect::<Vec<u16>>();

        Active::<T>::insert( active.clone() );
        Emission::<T>::insert( cloned_emission );
        Incentive::<T>::insert( cloned_incentive );
        Dividends::<T>::insert( cloned_dividends );


        // Emission tuples ( keys, u64 emission)
        let mut result: Vec<(T::AccountId, u64)> = vec![]; 
        for ( uid_i, key ) in keys.iter() {
            result.push( ( key.clone(), emission[ *uid_i as usize ] ) );
        }
        result
    }

    pub fn get_normalized_stake()-> Vec<I32F32> {
        let n: usize = Self::get_n() as usize; 
        let mut stake_64: Vec<I64F64> = vec![ I64F64::from_num(0.0); n ]; 
        for neuron_uid in 0..n {
            stake_64[neuron_uid] = I64F64::from_num( Self::get_stake_for_uid_and_subnetwork( neuron_uid as u16 ) );
        }
        inplace_normalize_64( &mut stake_64 );
        let stake: Vec<I32F32> = vec_fixed64_to_fixed32( stake_64 );
        stake
    }

    pub fn get_block_at_registration()-> Vec<u64> { 
        let n: usize = Self::get_n() as usize;
        let mut block_at_registration: Vec<u64> = vec![ 0; n ];
        for neuron_uid in 0..n {
            if Keys::<T>::contains_key( neuron_uid as u16 ){
                block_at_registration[ neuron_uid ] = Self::get_neuron_block_at_registration( neuron_uid as u16 );
            }
        }
        block_at_registration
    }

    pub fn get_weights_sparse()-> Vec<Vec<(u16, I32F32)>> { 
        let n: usize = Self::get_n() as usize; 
        let mut weights: Vec<Vec<(u16, I32F32)>> = vec![ vec![]; n ]; 
        for ( uid_i, weights_i ) in < Weights<T> as IterableStorageMap<u16, Vec<(u16, u16)> >>::iter_prefix() {
            for (uid_j, weight_ij) in weights_i.iter() { 
                weights [ uid_i as usize ].push( ( *uid_j, u16_proportion_to_fixed( *weight_ij ) ));
            }
        }
        weights
    } 

    pub fn get_weights()-> Vec<Vec<I32F32>> { 
        let n: usize = Self::get_n() as usize; 
        let mut weights: Vec<Vec<I32F32>> = vec![ vec![ I32F32::from_num(0.0); n ]; n ]; 
        for ( uid_i, weights_i ) in < Weights<T> as IterableStorageMap<u16, Vec<(u16, u16)> >>::iter_prefix() {
            for (uid_j, weight_ij) in weights_i.iter() { 
                weights [ uid_i as usize ] [ *uid_j as usize ] = u16_proportion_to_fixed(  *weight_ij );
            }
        }
        weights
    }

    pub fn get_bonds()-> Vec<Vec<(u16, I32F32)>> { 
        let n: usize = Self::get_n() as usize; 
        let mut bonds: Vec<Vec<(u16, I32F32)>> = vec![ vec![]; n ]; 
        for ( uid_i, bonds_i ) in < Bonds<T> as IterableStorageMap<u16, Vec<(u16, u16)> >>::iter_prefix() {
            for (uid_j, bonds_ij) in bonds_i.iter() { 
                bonds [ uid_i as usize ].push( ( *uid_j, u16_proportion_to_fixed( *bonds_ij ) ));
            }
        }
        bonds
    } 



    pub fn block_step() {
        let block_number: u64 = Self::get_current_block_as_u64();
        log::debug!("block_step for block: {:?} ", block_number );
        // --- 1. Adjust difficulties.
		Self::adjust_registration_terms_for_network( );
        // --- 2. Drains emission tuples ( key, amount ).
        Self::drain_emission( block_number );
        // --- 3. Generates emission tuples from epoch functions.
		Self::generate_emission( block_number );
    }

    // Helper function which returns the number of blocks remaining before we will run the epoch on this
    // network. Networks run their epoch when (block_number + netuid + 1 ) % (tempo + 1) = 0
    //
    pub fn blocks_until_next_epoch( tempo: u16, block_number: u64 ) -> u64 { 
        if tempo == 0 { return 10 } // Special case: tempo = 0, the network never runs.
        // tempo | netuid | # first epoch block
        //   1        0               0
        //   1        1               1
        //   2        0               1
        //   2        1               0
        //   100      0              99
        //   100      1              98
        return tempo as u64 - ( block_number  as u64 + 1 ) % ( tempo as u64 + 1 )
    }

 

    pub fn has_loaded_emission_tuples( netuid: u16 ) -> bool { LoadedEmission::<T>::contains_key() }
    pub fn get_loaded_emission_tuples( netuid: u16 ) -> Vec<(T::AccountId, u64)> { LoadedEmission::<T>::get().unwrap() }

    // Reads from the loaded emission storage which contains lists of pending emission tuples ( key, amount )
    // and distributes small chunks of them at a time.
    //
    pub fn drain_emission( block_number: u64 ) {
        // --- 1. We iterate across each network.
        let tempo = Tempo::<T>::get()
        if !Self::has_loaded_emission_tuples() { continue } // There are no tuples to emit.
        let tuples_to_drain: Vec<(T::AccountId, u64)> = Self::get_loaded_emission_tuples();
        for (key, amount) in tuples_to_drain.iter() {                 
            Self::emit_inflation_through_account( &key, *amount );
        }            
        LoadedEmission::<T>::remove();
        }
    }

    // Iterates through networks queues more emission onto their pending storage.
    // If a network has no blocks left until tempo, we run the epoch function and generate
    // more token emission tuples for later draining onto accounts.
    //
    pub fn generate_emission( block_number: u64 ) {

        // --- 1. Iterate through network ids.
        let tempo =  Tempo::<T>::get();

        // --- 2. Queue the emission due to this network.
        let new_queued_emission = EmissionValues::<T>::get();
        PendingEmission::<T>::mutate( | queued | *queued += new_queued_emission );
        log::debug!("netuid_i: {:?} queued_emission: +{:?} ", new_queued_emission );  
        // --- 3. Check to see if this network has reached tempo.
        if Self::blocks_until_next_epoch( tempo, block_number ) != 0 {
            // --- 3.1 No epoch, increase blocks since last step and continue,
            Self::set_blocks_since_last_step( Self::get_blocks_since_last_step() + 1 );
            continue;
        } else {
            // --- 4 This network is at tempo and we are running its epoch.
            // First frain the queued emission.
            let emission_to_drain:u64 = PendingEmission::<T>::get(); 
            PendingEmission::<T>::insert( 0 );

            // --- 5. Run the epoch mechanism and return emission tuples for keys in the network.
            let emission_tuples_this_block: Vec<(T::AccountId, u64)> = Self::epoch( emission_to_drain );
                
            // --- 6. Check that the emission does not exceed the allowed total.
            let emission_sum: u128 = emission_tuples_this_block.iter().map( |(_account_id, e)| *e as u128 ).sum();
            if emission_sum > emission_to_drain as u128 { continue } // Saftey check.

            // --- 7. Sink the emission tuples onto the already loaded.
            let mut concat_emission_tuples: Vec<(T::AccountId, u64)> = emission_tuples_this_block.clone();
            if Self::has_loaded_emission_tuples() {
                // 7.a We already have loaded emission tuples, so we concat the new ones.
                let mut current_emission_tuples: Vec<(T::AccountId, u64)> = Self::get_loaded_emission_tuples();
                concat_emission_tuples.append( &mut current_emission_tuples );
            } 
            LoadedEmission::<T>::insert( concat_emission_tuples );

            // --- 8 Set counters.
            Self::set_blocks_since_last_step( 0 );
            Self::set_last_mechanism_step_block( block_number );  
    }      
    }
    
    // Distributes token inflation through the key based on emission. The call ensures that the inflation
    // is distributed onto the accounts in proportion of the stake delegated minus the take. This function
    // is called after an epoch to distribute the newly minted stake according to delegation.
    //
    pub fn emit_inflation_through_account( key: &T::AccountId, emission: u64) {
        

        // --- 2. The key is a delegate. We first distribute a proportion of the emission to the key
        // directly as a function of its 'take'
        let total_stake: u64 = Self::get_total_stake_for_key( key );
 
        let remaining_emission: u64 = emission ;

        // 3. -- The remaining emission goes to the owners in proportion to the stake delegated.
        for ( owning_key_i, stake_i ) in < Stake<T> as IterableStorageMap<T::AccountId,  u64 >>::iter() {
            
            // --- 4. The emission proportion is remaining_emission * ( stake / total_stake ).
            let stake_proportion: u64 = Self::calculate_stake_proportional_emission( stake_i, total_stake, remaining_emission );
            Self::increase_stake_on_account( &key , stake_proportion );
            log::debug!("owning_key_i: {:?}  emission: +{:?} ", owning_key_i, stake_proportion );

        }


    }


    // Returns emission awarded to a key as a function of its proportion of the total stake.
    //
    pub fn calculate_stake_proportional_emission( stake: u64, total_stake:u64, emission: u64 ) -> u64 {
        if total_stake == 0 { return 0 };
        let stake_proportion: I64F64 = I64F64::from_num( stake ) / I64F64::from_num( total_stake );
        let proportional_emission: I64F64 = I64F64::from_num( emission ) * stake_proportion;
        return proportional_emission.to_num::<u64>();
    }



    // Adjusts the network of every active network. Reseting state parameters.
    //
    pub fn adjust_registration_terms_for_network( ) {
        
        // --- 1. Iterate through each network.

        let last_adjustment_block: u64 = Self::get_last_adjustment_block();
        let adjustment_interval: u16 = Self::get_adjustment_interval();
        let current_block: u64 = Self::get_current_block_as_u64( ); 
        log::debug!("netuid: {:?} last_adjustment_block: {:?} adjustment_interval: {:?} current_block: {:?}", 
            
            last_adjustment_block,
            adjustment_interval,
            current_block
        );

        // --- 3. Check if we are at the adjustment interval for this network.
        // If so, we need to adjust the registration based on target and actual registrations.
        if ( current_block - last_adjustment_block ) >= adjustment_interval as u64 {

            let registrations_this_interval: u16 = Self::get_registrations_this_interval();
            let target_registrations_this_interval: u16 = Self::get_target_registrations_per_interval();

            // --- 6. Drain all counters for this network for this interval.
            Self::set_last_adjustment_block( current_block );
            Self::set_registrations_this_interval( 0 );
        }

        // --- 7. Drain block registrations for each network. Needed for registration rate limits.
        Self::set_registrations_this_block( 0 );
        }
}
