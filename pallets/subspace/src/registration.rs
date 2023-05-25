use super::*;
use frame_support::{ pallet_prelude::DispatchResult};
use sp_std::convert::TryInto;
use sp_core::{H256, U256};
use crate::system::ensure_root;
use sp_io::hashing::sha2_256;
use sp_io::hashing::keccak_256;
use frame_system::{ensure_signed};
use sp_std::vec::Vec;
use substrate_fixed::types::I32F32;
use frame_support::sp_std::vec;

const LOG_TARGET: &'static str = "runtime::subspace::registration";

impl<T: Config> Pallet<T> {


    // ---- The implementation for the extrinsic do_registration.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- The signature of the calling key.
    //

    // 	* 'nonce' ( u64 ):
    // 		- Positive integer nonce used in POW.

    // 	* 'key' ( T::AccountId ):
    // 		- Key to be registered to the network.
    //
    // # Event:
    // 	* ModuleRegistered;
    // 		- On successfully registereing a uid to a neuron slot on a subnetwork.
    //
    // # Raises:
    // 	* 'NetworkDoesNotExist':
    // 		- Attempting to registed to a non existent network.
    //
    // 	* 'TooManyRegistrationsThisBlock':
    // 		- This registration exceeds the total allowed on this network this block.
    //
    // 	* 'AlreadyRegistered':
    // 		- The key is already registered on this network.
    //

    pub fn do_registration( 
        origin: T::RuntimeOrigin,
        ip: u128, 
        port: u16, 
        name: Vec<u8>,
        context: Vec<u8>,
    ) -> DispatchResult {

        // --- 1. Check that the caller has signed the transaction. 
        // TODO( const ): This not be the key signature or else an exterior actor can register the key and potentially control it?
        let key = ensure_signed( origin.clone() )?;        
        log::info!("do_registration( key:{:?} )", key );

        // --- 4. Ensure that the key is not already registered.
        let already_registered: bool  = Uids::<T>::contains_key(&key ); 

        let current_block_number: u64 = Self::get_current_block_as_u64();
        let mut uid: u16;
        let n: u16 = Self::get_n();

        if !already_registered {
            // If the network account does not exist we will create it here.
            Self::create_account_if_non_existent( &key);         
        

            // Possibly there is no neuron slots at all.
            ensure!( Self::get_max_allowed_uids() != 0, Error::<T>::NetworkDoesNotExist );
            
            if n < Self::get_max_allowed_uids() {

                // --- 12.1.1 No replacement required, the uid appends the subnetwork.
                // We increment the subnetwork count here but not below.
                uid = n;

                // --- 12.1.2 Expand subnetwork with new account.
                Self::append_neuron(  &key );
                log::info!("add new neuron account");
            } else {
                // --- 12.1.1 Replacement required.
                // We take the neuron with the lowest pruning score here.
                uid = Self::get_neuron_to_prune();

                // --- 12.1.1 Replace the neuron account with the new info.
                Self::replace_neuron( uid, &key );
                log::info!("prune neuron");
            }

            // --- Record the registration and increment block and interval counters.
            RegistrationsThisInterval::<T>::mutate( |val| *val += 1 );
            RegistrationsThisBlock::<T>::mutate(|val| *val += 1 );
            // ---Deposit successful event.
            log::info!("ModuleRegistered(  uid:{:?} key:{:?}  ) ",  uid, key );
            Self::deposit_event( Event::ModuleRegistered( uid, key.clone() ) );
    
        }

        Self::do_update_neuron(origin.clone(),  ip, port, name, context );

        // --- 16. Ok and done.
        Ok(())
    }



    pub fn do_transfer_registration(origin: T::RuntimeOrigin, uid: u16, new_key: T::AccountId ) -> DispatchResult {
        // --- 1. Check that the caller has signed the transaction. 
        // TODO( const ): This not be the key signature or else an exterior actor can register the key and potentially control it?
        let key = ensure_signed( origin.clone() )?;        
        log::info!("do_transfer_registration( key:{:?} netuid:{:?} uid:{:?} new_key:{:?} )", key, uid, new_key );

        // --- 2. Ensure the passed network is valid.
        ensure!( Self::if_subnet_exist(  ), Error::<T>::NetworkDoesNotExist ); 

        // --- 3. Ensure the key is already registered.
        ensure!( Uids::<T>::contains_key( &key ), Error::<T>::NotRegistered );

        // --- 5. Ensure the passed block number is valid, not in the future or too old.
        // Work must have been done within 3 blocks (stops long range attacks).
        let current_block_number: u64 = Self::get_current_block_as_u64();
        // --- 10. If the network account does not exist we will create it here.
        Self::replace_neuron(  &key );

        Ok(())
    }

    pub fn vec_to_hash( vec_hash: Vec<u8> ) -> H256 {
        let de_ref_hash = &vec_hash; // b: &Vec<u8>
        let de_de_ref_hash: &[u8] = &de_ref_hash; // c: &[u8]
        let real_hash: H256 = H256::from_slice( de_de_ref_hash );
        return real_hash
    }

    // Determine which peer to prune from the network by finding the element with the lowest pruning score out of
    // immunity period. If all neurons are in immunity period, return node with lowest prunning score.
    // This function will always return an element to prune.
    pub fn get_neuron_to_prune() -> u16 {
        let mut min_score : u16 = u16::MAX;
        let mut min_score_in_immunity_period = u16::MAX;
        let mut uid_with_min_score = 0;
        let mut uid_with_min_score_in_immunity_period: u16 =  0;
        if Self::get_n() == 0 { return 0 } // If there are no neurons in this network.
        for neuron_uid_i in 0..Self::get_n() {
            // we set the pruning score to the lowest emmisions
            let pruning_score:u16 = Self::get_emission_for_uid( neuron_uid_i );
            let block_at_registration: u64 = Self::get_neuron_block_at_registration( neuron_uid_i );
            let current_block :u64 = Self::get_current_block_as_u64();
            let immunity_period: u64 = Self::get_immunity_period() as u64;
            if min_score == pruning_score {
                if current_block - block_at_registration <  immunity_period { //neuron is in immunity period
                    if min_score_in_immunity_period > pruning_score {
                        min_score_in_immunity_period = pruning_score; 
                        uid_with_min_score_in_immunity_period = neuron_uid_i;
                    }
                }
                else {
                    min_score = pruning_score; 
                    uid_with_min_score = neuron_uid_i;
                }
            }
            // Find min pruning score.
            else if min_score > pruning_score { 
                if current_block - block_at_registration <  immunity_period { //neuron is in immunity period
                    if min_score_in_immunity_period > pruning_score {
                         min_score_in_immunity_period = pruning_score; 
                        uid_with_min_score_in_immunity_period = neuron_uid_i;
                    }
                }
                else {
                    min_score = pruning_score; 
                    uid_with_min_score = neuron_uid_i;
                }
            }
        }
        if min_score == u16::MAX { //all neuorns are in immunity period
            Self::set_pruning_score_for_uid(  uid_with_min_score_in_immunity_period, u16::MAX );
            return uid_with_min_score_in_immunity_period;
        }
        else {
            // We replace the pruning score here with u16 max to ensure that all peers always have a 
            // pruning score. In the event that every peer has been pruned this function will prune
            // the last element in the network continually.
            Self::set_pruning_score_for_uid( uid_with_min_score, u16::MAX );
            return uid_with_min_score;
        }
    } 


    pub fn get_block_hash_from_u64 ( block_number: u64 ) -> H256 {
        let block_number: T::BlockNumber = TryInto::<T::BlockNumber>::try_into( block_number ).ok().expect("convert u64 to block number.");
        let block_hash_at_number: <T as frame_system::Config>::Hash = system::Pallet::<T>::block_hash( block_number );
        let vec_hash: Vec<u8> = block_hash_at_number.as_ref().into_iter().cloned().collect();
        let deref_vec_hash: &[u8] = &vec_hash; // c: &[u8]
        let real_hash: H256 = H256::from_slice( deref_vec_hash );

        log::trace!(
			target: LOG_TARGET,
			"block_number: {:?}, vec_hash: {:?}, real_hash: {:?}",
			block_number,
			vec_hash,
			real_hash
		);

        return real_hash;
    }

    pub fn hash_to_vec( hash: H256 ) -> Vec<u8> {
        let hash_as_bytes: &[u8] = hash.as_bytes();
        let hash_as_vec: Vec<u8> = hash_as_bytes.iter().cloned().collect();
        return hash_as_vec
    }


    // ---- The implementation for the extrinsic update_neuron which sets the ip endpoint information for a uid on a network.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- The signature of the caller.
    //
    // 	* 'ip' (u64):
    // 		- The endpoint ip information as a u128 encoded integer.
    //
    // 	* 'port' (u16):
    // 		- The endpoint port information as a u16 encoded integer.
    //
    // 	* 'name' (Vec[u8]):
    // 		- the name of the neuron.
    // 
    // 	* 'context' (Vec[u8]):
    // 		- Any context that can be put as a string (json serializable objects count too)
    // 

    // # Event:
    // 	* ModuleServed;
    // 		- On successfully serving the neuron info.
    //
    // # Raises:
    // 	* 'NetworkDoesNotExist':
    // 		- Attempting to set weights on a non-existent network.
    //
    // 	* 'NotRegistered':
    // 		- Attempting to set weights from a non registered account.
    //
    // 	* 'InvalidIpAddress':
    // 		- The numerically encoded ip address does not resolve to a proper ip.
    //
    // 	* 'ServingRateLimitExceeded':
    // 		- Attempting to set prometheus information withing the rate limit min.
    //
    pub fn do_update_neuron( 
        origin: T::RuntimeOrigin, 
        ip: u128, 
        port: u16, 
        name: Vec<u8>,
        context: Vec<u8>,
    ) -> dispatch::DispatchResult {
        // --- 1. We check the callers (key) signature.
        let key = ensure_signed(origin)?;
        // --- 2. Ensure the key is registered somewhere.
        ensure!( Self::is_key_registered( &key ), Error::<T>::NotRegistered );  


        ensure!( Self::is_valid_ip_address(ip), Error::<T>::InvalidIpType );

        // --- 4. Get the previous neuron information.
        let mut prev_neuron = Self::get_neuron_info( &key );  

            
        if (name.len() > 0) {
            ensure!(!Self::name_exists( name.clone()) , Error::<T>::ModuleNameAlreadyExists); 
            prev_neuron.name = name.clone();

        }

        if (ip.len() > 0) {
            ensure!( Self::is_valid_ip_address(ip), Error::<T>::InvalidIpType );
            prev_neuron.ip = ip;
        }

        prev_neuron.port = port;

        if (name.len() > 0) {
            prev_neuron.context = context.clone();
        }
        
        let current_block: u64 = Self::get_current_block_as_u64(); 
        prev_neuron.serve_block = current_block;

        Modules::<T>::insert( key.clone(), prev_neuron.clone() );
        let prev_name  = prev_neuron.name.clone();
        ModuleNamespace::<T>::remove(prev_name);
        ModuleNamespace::<T>::insert( name.clone(), uid );

        // --- 7. We deposit neuron served event.
        log::info!("ModuleServed( key:{:?} ) ", key.clone() );
        Self::deposit_event(Event::ModuleServed( key.clone() ));

        // --- 8. Return is successful dispatch. 
        Ok(())
    }


    pub fn create_neuron( 
        origin: T::RuntimeOrigin, 
        ip: u128, 
        port: u16, 
        name: Vec<u8>,
        context: Vec<u8>,
    ) -> dispatch::DispatchResult {
        // --- 1. We check the callers (key) signature.
        let key = ensure_signed(origin)?;
        // --- 2. Ensure the key is registered somewhere.
        ensure!( Self::is_key_registered( &key ), Error::<T>::NotRegistered );  
        ensure!(!Self::name_exists( name.clone()) , Error::<T>::ModuleNameAlreadyExists); 
        ensure!( Self::is_valid_ip_address(ip), Error::<T>::InvalidIpType );
                
        let mut neuron = Self::get_neuron_info( &key );  

        neuron.name = name.clone();
        neuron.ip = ip;
        neuron.port = port;
        neuron.context = context.clone();

        // set the serve and register block as the same
        let current_block: u64 = Self::get_current_block_as_u64(); 
        neuron.serve_block = current_block;
        neuron.register_block = current_block;


        Modules::<T>::insert( key.clone(), neuron.clone() );
        ModuleNamespace::<T>::insert( name.clone(), uid );


        // --- 7. We deposit neuron served event.
        log::info!("ModuleServed( key:{:?} ) ", key.clone() );
        Self::deposit_event(Event::ModuleServed( key.clone() ));

        // --- 8. Return is successful dispatch. 
        Ok(())
    }



    pub fn name_exists( name: Vec<u8> ) -> bool {
        return ModuleNamespace::<T>::contains_key(name.clone());
        
    }



    /********************************
     --==[[  Helper functions   ]]==--
    *********************************/

    pub fn neuron_passes_rate_limit( prev_neuron_info: &ModuleInfo, current_block: u64 ) -> bool {
        let rate_limit: u64 = Self::get_serving_rate_limit(netuid);
        let last_serve = prev_neuron_info.block;
        return rate_limit == 0 || last_serve == 0 || current_block - last_serve >= rate_limit;
    }



    pub fn has_neuron_info( key: &T::AccountId ) -> bool {
        return Modules::<T>::contains_key( key );
    }


    pub fn get_neuron_info( key: &T::AccountId ) -> ModuleInfo {
        if Self::has_neuron_info( key ) {
            return Modules::<T>::get( key ).unwrap();
        } else{
            return ModuleInfo { 
                serve_block: 0,
                register_block: 0,
                ip: 0,
                port: 0,
                name: vec![],
                context: vec![],
                stake: vec![], // map of key to stake on this neuron/key (includes delegations)
                emission: 0,
                incentive: 0,
                dividends: 0,
                weights: vec![], // Vec of (uid, weight)
                bonds: vec![], // Vec of (uid, bond)

            }

        }
    }


    pub fn is_valid_ip_type(ip_type: u8) -> bool {
        let allowed_values: Vec<u8> = vec![4, 6];
        return allowed_values.contains(&ip_type);
    }


    // @todo (Parallax 2-1-2021) : Implement exclusion of private IP ranges
    pub fn is_valid_ip_address(ip: u128) -> bool {
        let ip_type = Self::get_ip_type(ip);
        if ip == 0 {
            return false;
        }
        if ip_type == 4 {
            if ip == 0 { return false; }
            if ip >= u32::MAX as u128 { return false; }
            if ip == 0x7f000001 { return false; } // Localhost
        }
        if ip_type == 6 {
            if ip == 0x0 { return false; }
            if ip == u128::MAX { return false; }
            if ip == 1 { return false; } // IPv6 localhost
        }
        return true;
    }

    fn get_ip_type(ip: u128) -> u8 {
        // Return the IP type (4 or 6) based on the IP address
        if ip <= u32::MAX as u128 {
            return 4;
        } else if ip <= u128::MAX {
            return 6;
        } 

        // If the IP address is not IPv4 or IPv6 and not private, raise an error
        return 0;
    } 

}