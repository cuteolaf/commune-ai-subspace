use super::*;
use frame_support::{sp_std::vec};
use sp_std::vec::Vec;
use frame_support::storage::IterableStorageMap;
use frame_support::pallet_prelude::DispatchError;
use frame_support::storage::IterableStorageDoubleMap;

impl<T: Config> Pallet<T> { 


    // Replace the neuron under this uid.
    pub fn replace_neuron( uid_to_replace: u16, new_key: &T::AccountId ) {

        log::debug!("replace_neuron( | uid_to_replace: {:?} | new_key: {:?} ) ", uid_to_replace, new_key );

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

    // Appends the uid to the network.
    pub fn append_neuron(new_key: &T::AccountId ) {

        // 1. Get the next uid. This is always equal to subnetwork_n.
        let next_uid: u16 = Self::get_n();
        let block_number = Self::get_current_block_as_u64();
        log::debug!("append_neuron(next_uid: {:?} | new_key: {:?} ) ",new_key, next_uid );

        // 2. Get and increase the uid count.

        // 3. Expand Yuma with new position.
        Rank::<T>::mutate( |v| v.push(0) );
        Active::<T>::mutate(|v| v.push( true ) );
        Emission::<T>::mutate(|v| v.push(0) );
        Incentive::<T>::mutate(|v| v.push(0) );
        Dividends::<T>::mutate(|v| v.push(0) );
        LastUpdate::<T>::mutate(|v| v.push( block_number ) );
        PruningScores::<T>::mutate(|v| v.push(0) );
 
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

    // Returs the key under the network uid as a Result. Ok if the uid is taken.
    //
    pub fn get_key_for_uid( neuron_uid: u16) ->  T::AccountId {
        Keys::<T>::try_get(neuron_uid).unwrap() 
    }
    

    // Returns the uid of the key in the network as a Result. Ok if the key has a slot.
    //
    pub fn get_uid_for_key( key: &T::AccountId) -> Result<u16, DispatchError> { 
        return Uids::<T>::try_get(&key).map_err(|_err| Error::<T>::NotRegistered.into()) 
    }

    // Returns the stake of the uid on network or 0 if it doesnt exist.
    //
    pub fn get_stake_for_uid( neuron_uid: u16) -> u64 { 
        if Self::is_uid_exist( neuron_uid) {
            return Self::get_total_stake_for_key( &Self::get_key_for_uid( neuron_uid ) ) 
        } else {
            return 0;
        }
    }

}
