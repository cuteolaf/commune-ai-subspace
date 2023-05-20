
use super::*;
use frame_support::inherent::Vec;
use sp_core::U256;
use frame_support::pallet_prelude::DispatchResult;
use crate::system::ensure_root;

impl<T: Config> Pallet<T> {

    // ========================
	// ==== Global Setters ====
	// ========================
    pub fn set_tempo( tempo: u16 ) { Tempo::<T>::insert(  tempo ); }
    pub fn set_last_adjustment_block( last_adjustment_block: u64 ) { LastAdjustmentBlock::<T>::insert(last_adjustment_block ); }
    pub fn set_blocks_since_last_step( blocks_since_last_step: u64 ) { BlocksSinceLastStep::<T>::insert(blocks_since_last_step ); }
    pub fn set_registrations_this_block( registrations_this_block: u16 ) { RegistrationsThisBlock::<T>::insert(registrations_this_block); }
    pub fn set_last_mechanism_step_block( last_mechanism_step_block: u64 ) { LastMechansimStepBlock::<T>::insert(last_mechanism_step_block); }
    pub fn set_registrations_this_interval( registrations_this_interval: u16 ) { RegistrationsThisInterval::<T>::insert(registrations_this_interval); }
    pub fn set_pow_registrations_this_interval( pow_registrations_this_interval: u16 ) { POWRegistrationsThisInterval::<T>::insert(pow_registrations_this_interval); }

    // ========================
	// ==== Global Getters ====
	// ========================
    pub fn get_total_issuance() -> u64 { TotalIssuance::<T>::get() }
    pub fn get_block_emission() -> u64 { BlockEmission::<T>::get() }
    pub fn get_current_block_as_u64( ) -> u64 { TryInto::try_into( <frame_system::Pallet<T>>::block_number() ).ok().expect("blockchain will not exceed 2^64 blocks; QED.") }

    // ==============================
	// ==== Yuma params ====
	// ==============================
    pub fn get_rank( ) -> Vec<u16> { Rank::<T>::get() }
    pub fn get_active( ) -> Vec<bool> { Active::<T>::get() }
    pub fn get_emission( ) -> Vec<u64> { Emission::<T>::get() }
    pub fn get_incentive( ) -> Vec<u16> { Incentive::<T>::get() }
    pub fn get_dividends( ) -> Vec<u16> { Dividends::<T>::get() }
    pub fn get_last_update( ) -> Vec<u64> { LastUpdate::<T>::get() }
    pub fn get_pruning_score( ) -> Vec<u16> { PruningScores::<T>::get() }

    
    pub fn set_last_update_for_uid( uid: u16, last_update: u64 ) { 
        let mut updated_last_update_vec = Self::get_last_update(); 
        if (uid as usize) < updated_last_update_vec.len() { 
            updated_last_update_vec[uid as usize] = last_update;
            LastUpdate::<T>::insert(updated_last_update_vec );
        }  
    }
    pub fn set_active_for_uid(uid: u16, active: bool ) { 
        let mut updated_active_vec = Self::get_active(); 
        if (uid as usize) < updated_active_vec.len() { 
            updated_active_vec[uid as usize] = active;
            Active::<T>::insert(updated_active_vec );
        }  
    }
    pub fn set_pruning_score_for_uid(uid: u16, pruning_score: u16 ) {
        log::info!("SubnetworkN::<T>::get() = {:?}", SubnetworkN::<T>::get() );
        log::info!("uid = {:?}", uid );
        assert!( uid < SubnetworkN::<T>::get() );
        PruningScores::<T>::mutate(|v| v[uid as usize] = pruning_score );
    }

    pub fn get_rank_for_uid(uid: u16) -> u16 { let vec = Rank::<T>::get(); if (uid as usize) < vec.len() { return vec[uid as usize] } else{ return 0 } }
    pub fn get_emission_for_uid(uid: u16) -> u64 {let vec =  Emission::<T>::get(); if (uid as usize) < vec.len() { return vec[uid as usize] } else{ return 0 } }
    pub fn get_active_for_uid(uid: u16) -> bool { let vec = Active::<T>::get(); if (uid as usize) < vec.len() { return vec[uid as usize] } else{ return false } }
    pub fn get_incentive_for_uid(uid: u16) -> u16 { let vec = Incentive::<T>::get(); if (uid as usize) < vec.len() { return vec[uid as usize] } else{ return 0 } }
    pub fn get_dividends_for_uid(uid: u16) -> u16 { let vec = Dividends::<T>::get(); if (uid as usize) < vec.len() { return vec[uid as usize] } else{ return 0 } }
    pub fn get_last_update_for_uid(uid: u16) -> u64 { let vec = LastUpdate::<T>::get(); if (uid as usize) < vec.len() { return vec[uid as usize] } else{ return 0 } }
    pub fn get_pruning_score_for_uid(uid: u16) -> u16 { let vec = PruningScores::<T>::get(); if (uid as usize) < vec.len() { return vec[uid as usize] } else{ return u16::MAX } }

    pub fn get_context_for_uid(uid: u16) -> Vec<u8> { 
        let key = Self::get_key_for_uid(uid);
        let neuron= Neurons::<T>::get(key ).unwrap();
        return neuron.context.clone();
     }

    pub fn get_name_for_uid(uid: u16) -> Vec<u8> { 
        let key = Self::get_key_for_uid(uid);
        let neuron= Neurons::<T>::get(key ).unwrap();
        return neuron.name.clone();
    
    }

    // ============================
	// ==== Subnetwork Getters ====
	// ============================
    pub fn get_tempo( ) -> u16{ Tempo::<T>::get() }
    pub fn get_emission_value( ) -> u64 { EmissionValues::<T>::get() }
    pub fn get_pending_emission( ) -> u64{ PendingEmission::<T>::get() }
    pub fn get_last_adjustment_block( ) -> u64 { LastAdjustmentBlock::<T>::get() }
    pub fn get_blocks_since_last_step() -> u64 { BlocksSinceLastStep::<T>::get() }
    pub fn get_registrations_this_block( ) -> u16 { RegistrationsThisBlock::<T>::get() }
    pub fn get_last_mechanism_step_block( ) -> u64 { LastMechansimStepBlock::<T>::get() }
    pub fn get_registrations_this_interval( ) -> u16 { RegistrationsThisInterval::<T>::get() } 
    pub fn get_pow_registrations_this_interval( ) -> u16 { POWRegistrationsThisInterval::<T>::get() } 
    pub fn get_neuron_block_at_registration( neuron_uid: u16 ) -> u64 { BlockAtRegistration::<T>::get(neuron_uid )}

    // ========================
	// ==== Rate Limiting =====
	// ========================
	pub fn get_last_tx_block( key: &T::AccountId ) -> u64 { LastTxBlock::<T>::get( key ) }
	pub fn exceeds_tx_rate_limit( prev_tx_block: u64, current_block: u64 ) -> bool {
        let rate_limit: u64 = Self::get_tx_rate_limit();
		if rate_limit == 0 || prev_tx_block == 0 {
			return false;
		}
        return current_block - prev_tx_block <= rate_limit;
    }



	// Configure tx rate limiting
	pub fn get_tx_rate_limit() -> u64 { TxRateLimit::<T>::get() }
    pub fn set_tx_rate_limit( tx_rate_limit: u64 ) { TxRateLimit::<T>::put( tx_rate_limit ) }
    pub fn do_sudo_set_tx_rate_limit( origin: T::RuntimeOrigin, tx_rate_limit: u64 ) -> DispatchResult { 
        ensure_root( origin )?;
        Self::set_tx_rate_limit( tx_rate_limit );
        log::info!("TxRateLimitSet( tx_rate_limit: {:?} ) ", tx_rate_limit );
        Self::deposit_event( Event::TxRateLimitSet( tx_rate_limit ) );
        Ok(()) 
    }

    pub fn get_serving_rate_limit( ) -> u64 { ServingRateLimit::<T>::get() }
    pub fn set_serving_rate_limit( serving_rate_limit: u64 ) { ServingRateLimit::<T>::insert(serving_rate_limit ) }
    pub fn do_sudo_set_serving_rate_limit( origin: T::RuntimeOrigin, serving_rate_limit: u64 ) -> DispatchResult { 
        let key = ensure_signed( origin )?;
        Self::set_serving_rate_limit(serving_rate_limit );
        log::info!("ServingRateLimitSet( serving_rate_limit: {:?} ) ", serving_rate_limit );
        Self::deposit_event( Event::ServingRateLimitSet(serving_rate_limit ) );
        Ok(()) 
    }


    pub fn get_weights_set_rate_limit( ) -> u64 { WeightsSetRateLimit::<T>::get() }
    pub fn set_weights_set_rate_limit( weights_set_rate_limit: u64 ) { WeightsSetRateLimit::<T>::insert(weights_set_rate_limit ); }
    pub fn do_sudo_set_weights_set_rate_limit( origin: T::RuntimeOrigin, weights_set_rate_limit: u64 ) -> DispatchResult { 
        ensure_root( origin )?;
        Self::set_weights_set_rate_limit(weights_set_rate_limit );
        log::info!("WeightsSetRateLimitSet( weights_set_rate_limit: {:?} ) ",weights_set_rate_limit);
        Self::deposit_event( Event::WeightsSetRateLimitSet(weights_set_rate_limit) );
        Ok(()) 
    }

    pub fn get_adjustment_interval( ) -> u16 { AdjustmentInterval::<T>::get() }
    pub fn set_adjustment_interval( adjustment_interval: u16 ) { AdjustmentInterval::<T>::insert(adjustment_interval ); }
    pub fn do_set_adjustment_interval( origin: T::RuntimeOrigin, adjustment_interval: u16 ) -> DispatchResult { 
        ensure_root( origin )?;
        Self::set_adjustment_interval(adjustment_interval );
        log::info!("AdjustmentIntervalSet( adjustment_interval: {:?} ) ",adjustment_interval);
        Self::deposit_event( Event::AdjustmentIntervalSet(adjustment_interval) );
        Ok(()) 
    }

    pub fn get_max_weight_limit( ) -> u16 { MaxWeightsLimit::<T>::get() }    
    pub fn set_max_weight_limit( max_weight_limit: u16 ) { MaxWeightsLimit::<T>::insert(max_weight_limit ); }
    pub fn do_sudo_set_max_weight_limit( origin:T::RuntimeOrigin, max_weight_limit: u16 ) -> DispatchResult {
        ensure_root( origin )?;
        Self::set_max_weight_limit(max_weight_limit );
        log::info!("MaxWeightLimitSet(  max_weight_limit: {:?} ) ",max_weight_limit);
        Self::deposit_event( Event::MaxWeightLimitSet(max_weight_limit ) );
        Ok(())
    }

    pub fn get_immunity_period() -> u16 { ImmunityPeriod::<T>::get() }
    pub fn set_immunity_period( immunity_period: u16 ) { ImmunityPeriod::<T>::insert(immunity_period ); }
    pub fn do_sudo_set_immunity_period( origin:T::RuntimeOrigin, immunity_period: u16 ) -> DispatchResult {
        ensure_root( origin )?;
        Self::set_immunity_period(immunity_period );
        log::info!("ImmunityPeriodSet(  immunity_period: {:?} ) ",immunity_period);
        Self::deposit_event(Event::ImmunityPeriodSet(immunity_period));
        Ok(())
    }

    pub fn get_min_allowed_weights( ) -> u16 { MinAllowedWeights::<T>::get() }
    pub fn set_min_allowed_weights( min_allowed_weights: u16 ) { MinAllowedWeights::<T>::insert(min_allowed_weights ); }
    pub fn do_sudo_set_min_allowed_weights( origin:T::RuntimeOrigin, min_allowed_weights: u16 ) -> DispatchResult {
        ensure_root( origin )?;
        Self::set_min_allowed_weights(min_allowed_weights );
        log::info!("MinAllowedWeightSet(  min_allowed_weights: {:?} ) ",min_allowed_weights);
        Self::deposit_event( Event::MinAllowedWeightSet(min_allowed_weights) );
        Ok(())
    }

    pub fn get_max_allowed_uids( ) -> u16  { MaxAllowedUids::<T>::get() }
    pub fn set_max_allowed_uids( max_allowed: u16) { MaxAllowedUids::<T>::insert(max_allowed ); }
    pub fn do_sudo_set_max_allowed_uids( origin:T::RuntimeOrigin,max_allowed_uids: u16 ) -> DispatchResult {
        ensure_root( origin )?;
        ensure!(Self::get_max_allowed_uids()< max_allowed_uids, Error::<T>::MaxAllowedUIdsNotAllowed);
        Self::set_max_allowed_uids( max_allowed_uids );
        log::info!("MaxAllowedUidsSet( max_allowed_uids: {:?} ) ", max_allowed_uids);
        Self::deposit_event( Event::MaxAllowedUidsSet(max_allowed_uids) );
        Ok(())
    }

        
    pub fn get_activity_cutoff(  ) -> u16  { ActivityCutoff::<T>::get( ) }
    pub fn do_sudo_set_activity_cutoff( origin:T::RuntimeOrigin, activity_cutoff: u16 ) -> DispatchResult {
        let key = ensure_signed( origin )?;
        ActivityCutoff::<T>::insert( activity_cutoff ); 
        log::info!("ActivityCutoffSet( activity_cutoff: {:?} ) ", activity_cutoff);
        Self::deposit_event( Event::ActivityCutoffSet( activity_cutoff) );
        Ok(())
    }
            
    pub fn get_target_registrations_per_interval(  ) -> u16 { TargetRegistrationsPerInterval::<T>::get( ) }
    pub fn set_target_registrations_per_interval( target_registrations_per_interval: u16 ) { TargetRegistrationsPerInterval::<T>::insert( target_registrations_per_interval ); }
    pub fn do_sudo_set_target_registrations_per_interval( origin:T::RuntimeOrigin,  target_registrations_per_interval: u16 ) -> DispatchResult {
        ensure_root( origin )?;
        Self::set_target_registrations_per_interval(target_registrations_per_interval );
        log::info!("RegistrationPerIntervalSet(  target_registrations_per_interval: {:?} ) ", target_registrations_per_interval );
        Self::deposit_event( Event::RegistrationPerIntervalSet(target_registrations_per_interval) );
        Ok(())
    }

    pub fn get_max_registrations_per_block() -> u16 { MaxRegistrationsPerBlock::<T>::get( ) }
    pub fn set_max_registrations_per_block( max_registrations_per_block: u16 ) { MaxRegistrationsPerBlock::<T>::put( max_registrations_per_block ); }
    pub fn do_sudo_set_max_registrations_per_block(
        origin: T::RuntimeOrigin, 
        max_registrations_per_block: u16
    ) -> DispatchResult {
        ensure_root( origin )?;
        Self::set_max_registrations_per_block( max_registrations_per_block );
        log::info!("MaxRegistrationsPerBlock(  max_registrations_per_block: {:?} ) ",max_registrations_per_block );
        Self::deposit_event( Event::MaxRegistrationsPerBlockSet( max_registrations_per_block) );
        Ok(())
    }

}


