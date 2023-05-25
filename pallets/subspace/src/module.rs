use super::*;
use frame_support::storage::IterableStorageMap;
use frame_support::{sp_std::vec};
use frame_support::pallet_prelude::{Decode, Encode};
extern crate alloc;
use alloc::vec::Vec;
use codec::Compact;
use frame_support::pallet_prelude::DispatchError;


impl<T: Config> Pallet<T> {
	pub fn get_modules(netuid: u16) -> Vec<ModuleInfo<T>> {

        let mut modules = Vec::new();
        let n = Self::get_subnetwork_n(netuid);
        for uid in 0..n {
            let uid = uid;

            let _module = Self::get_module(uid);
            let module;
            if _module.is_none() {
                break; // No more modules
            } else {
                // No error, key was registered
                module = _module.expect("Module should exist");
            }

            modules.push( module );
        }
        return modules;
	}

    fn get_module( uid: u16) -> Option<ModuleInfo<T>> {
        
        let key = Self::get_key_for_uid(uid);
        let module_info = Self::get_module_info( &key.clone() );    
        let active = Self::get_active_for_uid( uid as u16 );
        let emission = Self::get_emission_for_uid(  uid as u16 );
        let incentive = Self::get_incentive_for_uid(  uid as u16 );
        let dividends = Self::get_dividends_for_uid(  uid as u16 );
        let last_update = Self::get_last_update_for_uid(  uid as u16 );
        let context = Self::get_context_for_uid(  uid as u16 );
        let name = Self::get_name_for_uid(  uid as u16 );


        let module = ModuleNetworkInfo {
            name: name.clone(),
            key: key.clone(),
            uid: uid.into(),
            active: active,
            stake: stake,
            emission: emission.into(),
            incentive: incentive.into(),
            dividends: dividends.into(),
            last_update: last_update.into(),
            weights: weights,
            bonds: bonds,
            context: context.clone(),
        };
        
        return Some(module);
    }


    // Replace the module under this uid.
    pub fn replace_module( uid_to_replace: u16, new_key: &T::AccountId ) {

        log::debug!("replace_module( | uid_to_replace: {:?} | new_key: {:?} ) ", uid_to_replace, new_key );

        // 1. Get the old key under this position.
        let old_key: T::AccountId = Keys::<T>::get( uid_to_replace );

        // 2. Remove previous set memberships.
        Uids::<T>::remove( old_key.clone() ); 
        Keys::<T>::remove( uid_to_replace ); 
        let block_number:u64 = Self::get_current_block_as_u64();

        // 3. Create new set memberships.
        Self::set_active_for_uid(  uid_to_replace, true ); // Set to active by default.
        Keys::<T>::insert( uid_to_replace, new_key.clone() ); // Make key - uid association.
        Uids::<T>::insert( new_key.clone(), uid_to_replace ); // Make uid - key association.
        BlockAtRegistration::<T>::insert(  uid_to_replace, block_number ); // Fill block at registration.
    }
    

    // Replace the module under this uid.
    pub fn remove_module( uid: u16 ) {

        log::debug!("replace_module( | uid_to_replace: {:?} | new_key: {:?} ) ", uid_to_replace, new_key );

        // 1. Get the old key under this position.
        let old_key: T::AccountId = Keys::<T>::get( uid );

        // 2. Remove previous set memberships.
        Uids::<T>::remove( old_key.clone() ); 
        Keys::<T>::remove( uid ); 
        let block_number:u64 = Self::get_current_block_as_u64();

    }

    // Appends the uid to the network.
    pub fn append_module(new_key: &T::AccountId ) {

        // 1. Get the next uid. This is always equal to subnetwork_n.
        let next_uid: u16 = Self::get_n();
        let block_number = Self::get_current_block_as_u64();
        log::debug!("append_module(next_uid: {:?} | new_key: {:?} ) ",new_key, next_uid );

        // 2. Get and increase the uid count.

        // 3. Expand Yuma with new position.
        Rank::<T>::mutate( |v| v.push(0) );
        Active::<T>::mutate(|v| v.push( true ) );
        Emission::<T>::mutate(|v| v.push(0) );
        Incentive::<T>::mutate(|v| v.push(0) );
        Dividends::<T>::mutate(|v| v.push(0) );
        LastUpdate::<T>::mutate(|v| v.push( block_number ) );
 
        // 4. Insert new account information.
        Keys::<T>::insert( next_uid, new_key.clone() ); // Make key - uid association.
        Uids::<T>::insert( new_key.clone(), next_uid ); // Make uid - key association.
        BlockAtRegistration::<T>::insert( block_number ); // Fill block at registration.
    }

    // Returns true if the uid is set on the network.
    //
    pub fn is_uid_exist(uid: u16) -> bool {
        return  Keys::<T>::contains_key( uid);
    }

    // Returns true if the key holds a slot on the network.
    //
    pub fn is_key_registered(key: &T::AccountId ) -> bool { 
        return Uids::<T>::contains_key( key ) 
    }

    pub fn is_uid_registered(uid: u16) -> bool {
        return  Self::<T>::is_uid_exist( uid);
    }

    // Returs the key under the network uid as a Result. Ok if the uid is taken.
    //
    pub fn get_key_for_uid( module_uid: u16) ->  T::AccountId {
        Keys::<T>::try_get(module_uid).unwrap() 
    }
    

    // Returns the uid of the key in the network as a Result. Ok if the key has a slot.
    //
    pub fn get_uid_for_key( key: &T::AccountId) -> Result<u16, DispatchError> { 
        return Uids::<T>::try_get(&key).map_err(|_err| Error::<T>::NotRegistered.into()) 
    }

    // Returns the stake of the uid on network or 0 if it doesnt exist.
    //
    pub fn get_stake_for_uid( module_uid: u16) -> u64 { 
        if Self::is_uid_exist( module_uid) {
            return Self::get_total_stake_for_key( &Self::get_key_for_uid( module_uid ) ) 
        } else {
            return 0;
        }
    }

    // Returns the stake of the uid on network or 0 if it doesnt exist.
    //
    pub fn get_stake_for_key(key: &T::AccountId ) -> u64 { 
        if Self::is_key_registered( &key) {
            return Self::get_total_stake_for_key( &key ) 
        } else {
            return 0;
        }
    }

}

