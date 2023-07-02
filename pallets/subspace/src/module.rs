use super::*;
use frame_support::storage::IterableStorageMap;
use frame_support::pallet_prelude::{Decode, Encode};
extern crate alloc;
use alloc::vec::Vec;
use codec::Compact;


impl<T: Config> Pallet<T> {


        // Replace the module under this uid.
        pub fn replace_module( netuid: u16, uid: u16, new_key: &T::AccountId, name: Vec<u8>, address: Vec<u8>, stake: u64 ) {

            log::debug!("remove_network_for_netuid( netuid: {:?} | uid : {:?} | new_key: {:?} ) ", netuid, uid, new_key );
            
            let block_number:u64 = Self::get_current_block_as_u64();
            let old_key: T::AccountId = Keys::<T>::get( netuid, uid );
            // 2. Remove previous set memberships.
            Uids::<T>::remove( netuid, old_key.clone() );  // Remove old key - uid association.
            Uids::<T>::insert( netuid, new_key.clone(), uid ); // Make uid - key association.
            Keys::<T>::insert( netuid, uid, new_key.clone() ); // Make key - uid association.

            BlockAtRegistration::<T>::insert( netuid, uid, block_number ); // Fill block at registration.
            Address::<T>::insert( netuid, uid, address ); // Fill module info.
            Namespace::<T>::insert( netuid, name.clone(), uid ); // Fill module namespace.
            Names::<T>::insert( netuid, uid, name.clone() ); // Fill module namespace.

            // 3. Remove the stake from the old account
            Self::decrease_all_stake_on_account( netuid, &old_key.clone() );
            Stake::<T>::remove( netuid, &old_key.clone() ); // Make uid - key association.
            
            
            // 4. Add the stake to the new account (self)
            Self::increase_stake_on_account( netuid, &new_key.clone(), &new_key.clone(), stake );
            // 4. Emit the event.
            
        }




    

        // Replace the module under this uid.
        pub fn remove_module( netuid: u16, uid: u16 ) {
            // 1. Get the old key under this position.
            let key: T::AccountId = Keys::<T>::get( netuid, uid );
            // 2. Remove previous set memberships.
            Uids::<T>::remove( netuid, key.clone() ); 
            Keys::<T>::remove( netuid, uid ); // Make key - uid association.
            Address::<T>::remove(netuid, uid ); // Make uid - key association.
            BlockAtRegistration::<T>::remove( netuid, uid ); // Fill block at registration.
            Weights::<T>::remove( netuid, uid ); // Make uid - key association.
            Self::remove_all_stake_on_account( netuid, &key.clone() ); // Make uid - key association.

            N::<T>::mutate( netuid, |v| *v -= 1 ); // Decrease the number of modules in the network.
            // 3. Remove the network if it is empty.
            if N::<T>::get( netuid ) == 0 {
                Self::remove_network_for_netuid( netuid );
            }

    
            
            // 4. Emit the event.
            
        }
    

        // Appends the uid to the network.
        pub fn append_module( netuid: u16, key: &T::AccountId , name: Vec<u8>, address: Vec<u8>, stake: u64) -> u16{
    
            // 1. Get the next uid. This is always equal to subnetwork_n.
            let uid: u16 = Self::get_subnetwork_n( netuid );
            let block_number = Self::get_current_block_as_u64();
            log::debug!("append_module( netuid: {:?} | uid: {:?} | new_key: {:?} ) ", netuid, key, uid );
    
            // 2. Get and increase the uid count.
            N::<T>::insert( netuid, uid + 1 );
    
            // 3. Expand Yuma with new position.
            Emission::<T>::mutate(netuid, |v| v.push(0) );
            Incentive::<T>::mutate(netuid, |v| v.push(0) );
            Dividends::<T>::mutate(netuid, |v| v.push(0) );
            LastUpdate::<T>::mutate(netuid, |v| v.push( block_number ) );
        
            // 4. Insert new account information.
            Keys::<T>::insert( netuid, uid, key.clone() ); // Make key - uid association.
            Uids::<T>::insert( netuid, key.clone(), uid ); // Make uid - key association.
            BlockAtRegistration::<T>::insert( netuid, uid, block_number ); // Fill block at registration.
            Namespace::<T>::insert( netuid, name.clone(), uid ); // Fill module namespace.
            Names::<T>::insert( netuid, uid, name.clone() ); // Fill module namespace.
            Address::<T>::insert( netuid, uid, address ); // Fill module info.

            Self::increase_stake_on_account( netuid, &key.clone(), &key.clone(), stake );

            return uid;
    
        }   
    




}

