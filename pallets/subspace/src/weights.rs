use super::*;
use frame_support::sp_std::vec;
use sp_std::vec::Vec;

impl<T: Config> Pallet<T> {


    // ---- The implementation for the extrinsic set_weights.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- The signature of the calling key.
    //
    // 	* 'netuid' (u16):
    // 		- The u16 network identifier.
    //
    // 	* 'uids' ( Vec<u16> ):
    // 		- The uids of the weights to be set on the chain.
    //
    // 	* 'values' ( Vec<u16> ):
    // 		- The values of the weights to set on the chain.

    // # Event:
    // 	* WeightsSet;
    // 		- On successfully setting the weights on chain.
    //
    // # Raises:
    // 	* 'NetworkDoesNotExist':
    // 		- Attempting to set weights on a non-existent network.
    //
    // 	* 'NotRegistered':
    // 		- Attempting to set weights from a non registered account.
    //

    // 	* 'SettingWeightsTooFast':
    // 		- Attempting to set weights faster than the weights_set_rate_limit.
    //
    // 	* 'NoValidatorPermit':
    // 		- Attempting to set non-self weights without a validator permit.
    //
    // 	* 'WeightVecNotEqualSize':
    // 		- Attempting to set weights with uids not of same length.
    //
    // 	* 'DuplicateUids':
    // 		- Attempting to set weights with duplicate uids.
    // 
    //     * 'TooManyUids':
    // 		- Attempting to set weights above the max allowed uids.
    //
    // 	* 'InvalidUid':
    // 		- Attempting to set weights with invalid uids.
    //
    // 	* 'NotSettingEnoughWeights':
    // 		- Attempting to set weights with fewer weights than min.
    //
    // 	* 'MaxWeightExceeded':
    // 		- Attempting to set weights with max value exceeding limit.
    //
    pub fn do_set_weights( origin: T::RuntimeOrigin, uids: Vec<u16>, values: Vec<u16> ) -> dispatch::DispatchResult{

        // --- 1. Check the caller's signature. This is the key of a registered account.
        let key = ensure_signed( origin )?;
        log::info!("do_set_weights( origin:{:?} uids:{:?}, values:{:?})", key, uids, values );

        // --- 2. Check that the length of uid list and value list are equal for this network.
        ensure!( Self::uids_match_values( &uids, &values ), Error::<T>::WeightVecNotEqualSize );
        
        // --- 4. Check to see if the number of uids is within the max allowed uids for this network.
        ensure!( Self::check_len_uids_within_allowed( &uids ), Error::<T>::TooManyUids);

        // --- 5. Check to see if the key is registered to the passed network.
        ensure!( Self::is_key_registered( &key ), Error::<T>::NotRegistered );


        // --- 7. Get the module uid of associated key on network netuid.
        let module_uid;
        match Self::get_uid_for_key( &key ) { Ok(k) => module_uid = k, Err(e) => panic!("Error: {:?}", e) } 

        // --- 8. Ensure the uid is not setting weights faster than the weights_set_rate_limit.
        let current_block: u64 = Self::get_current_block_as_u64();
        ensure!( Self::check_rate_limit( module_uid, current_block ), Error::<T>::SettingWeightsTooFast );

 
        // --- 10. Ensure the passed uids contain no duplicates.
        ensure!( !Self::has_duplicate_uids( &uids ), Error::<T>::DuplicateUids );

        // --- 11. Ensure that the passed uids are valid for the network.
        ensure!( !Self::contains_invalid_uids( &uids ), Error::<T>::InvalidUid );

        // --- 12. Ensure that the weights have the required length.
        ensure!( Self::check_length( module_uid, &uids, &values ), Error::<T>::NotSettingEnoughWeights );

        // --- 13. Normalize the weights.
        let normalized_values = Self::normalize_weights( values );

        // --- 14. Ensure the weights are max weight limited 
        ensure!( Self::max_weight_limited(module_uid, &uids, &normalized_values ), Error::<T>::MaxWeightExceeded );

        // --- 15. Zip weights for sinking to storage map.
        let mut zipped_weights: Vec<( u16, u16 )> = vec![];
        for ( uid, val ) in uids.iter().zip(normalized_values.iter()) { zipped_weights.push((*uid, *val)) }

        // --- 16. Set weights under  uid double map entry.
        Weights::<T>::insert( module_uid, zipped_weights );

        // --- 17. Set the activity for the weights on this network.
        Self::set_last_update_for_uid(module_uid, current_block );

        // --- 18. Emit the tracking event.
        log::info!("WeightsSet(  module_uid:{:?} )", module_uid );
        Self::deposit_event( Event::WeightsSet( module_uid ) );

        // --- 19. Return ok.
        Ok(())
    }



    // Checks if the module has set weights within the weights_set_rate_limit.
    //
    pub fn check_rate_limit( module_uid: u16, current_block: u64 ) -> bool {
        if Self::is_uid_exist( module_uid ){ 
            // --- 1. Ensure that the diff between current and last_set weights is greater than limit.
            let last_set_weights: u64 = Self::get_last_update_for_uid( module_uid );
            if last_set_weights == 0 { return true; } // (Storage default) Never set weights.
            return current_block - last_set_weights >= Self::get_weights_set_rate_limit( );
        }
        // --- 3. Non registered peers cant pass.
        return false;
    }

    // Checks for any invalid uids on this network.
    pub fn contains_invalid_uids( uids: &Vec<u16> ) -> bool {
        for uid in uids {
            if !Self::is_uid_exist( *uid ) {
                return true;
            }
        }
        return false;
    }

    // Returns true if the passed uids have the same length of the passed values.
    fn uids_match_values(uids: &Vec<u16>, values: &Vec<u16>) -> bool {
        return uids.len() == values.len();
    }

    // Returns true if the items contain duplicates.
    fn has_duplicate_uids(items: &Vec<u16>) -> bool {
        let mut parsed: Vec<u16> = Vec::new();
        for item in items {
            if parsed.contains(&item) { return true; }
            parsed.push(item.clone());
        }
        return false;
    }

    // Returns True if the uids and weights are have a valid length for uid on network.
    pub fn check_length( uid: u16, uids: &Vec<u16>, weights: &Vec<u16> ) -> bool {
        let min_allowed_length: usize = Self::get_min_allowed_weights() as usize;

        // Check self weight. Allowed to set single value for self weight.
        if Self::is_self_weight(uid, uids, weights) {
            return true;
        }
        // Check if number of weights exceeds min.
        if weights.len() >= min_allowed_length {
            return true;
        }
        // To few weights.
        return false;
    }

    // Implace normalizes the passed positive integer weights so that they sum to u16 max value.
    pub fn normalize_weights(mut weights: Vec<u16>) -> Vec<u16> {
        let sum: u64 = weights.iter().map(|x| *x as u64).sum();
        if sum == 0 { return weights; }
        weights.iter_mut().for_each(|x| { *x = (*x as u64 * u16::max_value() as u64 / sum) as u16; });
        return weights;
    }

    // Returns False if the weights exceed the max_weight_limit for this network.
    pub fn max_weight_limited( uid: u16, uids: &Vec<u16>, weights: &Vec<u16> ) -> bool {

        // Allow self weights to exceed max weight limit.
        if Self::is_self_weight( uid, uids, weights ) { return true; }

        // If the max weight limit it u16 max, return true.
        let max_weight_limit: u16 = Self::get_max_weight_limit( );
        if max_weight_limit == u16::MAX { return true; }
    
        // Check if the weights max value is less than or equal to the limit.
        let max: u16 = *weights.iter().max().unwrap();
        if max <= max_weight_limit { return true; }
        
        // The check has failed.
        return false;
    }

    // Returns true if the uids and weights correspond to a self weight on the uid.
    pub fn is_self_weight( uid: u16, uids: &Vec<u16>, weights: &Vec<u16> ) -> bool {
        if weights.len() != 1 { return false; }
        if uid != uids[0] { return false; } 
        return true;
    }

    // Returns False is the number of uids exceeds the allowed number of uids for this network.
    pub fn check_len_uids_within_allowed( uids: &Vec<u16> ) -> bool {
        let subnetwork_n: u16 = Self::get_n();
        // we should expect at most subnetwork_n uids.
        return uids.len() <= subnetwork_n as usize;
    }
    
}