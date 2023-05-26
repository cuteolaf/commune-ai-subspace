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
    // 		- On successfully registereing a uid to a module slot on a subnetwork.
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
        ensure!(!Self::is_key_registered(&key),Error::<T>::AlreadyRegistered);  
        ensure!(!Self::name_exists( name.clone()) , Error::<T>::ModuleNameAlreadyExists); 
        ensure!( Self::is_valid_ip_address(ip), Error::<T>::InvalidIpType );
        
        log::info!("do_registration( key:{:?} )", key );

        // --- 4. Ensure that the key is not already registered.
        let already_registered: bool  =  Self::is_key_registered(&key); 

        let current_block_number: u64 = Self::get_current_block_as_u64();
        let mut uid: u16;
        let n: u16 = Self::get_n();


        // If the network account does not exist we will create it here.
        Self::create_account_if_non_existent( &key);         
    
        // Possibly there is no module slots at all.
        ensure!( Self::get_max_allowed_uids() != 0, Error::<T>::NetworkDoesNotExist );
        
        if n < Self::get_max_allowed_uids() {

            // --- 12.1.1 No replacement required, the uid appends the subnetwork.
            // We increment the subnetwork count here but not below.
            uid = n;

            // --- 12.1.2 Expand subnetwork with new account.
            Self::append_module(  &key );
            log::info!("add new module account");
        } else {
            // --- 12.1.1 Replacement required.
            // We take the module with the lowest pruning score here.
            uid = Self::get_module_to_prune();

            // --- 12.1.1 Replace the module account with the new info.
            Self::replace_module( uid, &key );
            log::info!("prune module");
        }

        let current_block: u64 = Self::get_current_block_as_u64(); 

        let mut module = Self::get_module_info( &key );  
        module.name = name.clone();
        module.ip = ip;
        module.port = port;
        module.context = context.clone();
        module.serve_block = current_block;
        module.register_block = current_block;


        Modules::<T>::insert( key.clone(), module.clone() );
        ModuleNamespace::<T>::insert( name.clone(), uid );

        // --- 8. Return is successful dispatch. 

        // --- Record the registration and increment block and interval counters.
        RegistrationsThisInterval::<T>::mutate( |val| *val += 1 );
        RegistrationsThisBlock::<T>::mutate(|val| *val += 1 );
        // ---Deposit successful event.
        log::info!("ModuleRegistered(  uid:{:?} key:{:?}  ) ",  uid, key );
        Self::deposit_event( Event::ModuleRegistered( uid, key.clone() ) );

        // --- 16. Ok and done.
        Ok(())
    }


    pub fn vec_to_hash( vec_hash: Vec<u8> ) -> H256 {
        let de_ref_hash = &vec_hash; // b: &Vec<u8>
        let de_de_ref_hash: &[u8] = &de_ref_hash; // c: &[u8]
        let real_hash: H256 = H256::from_slice( de_de_ref_hash );
        return real_hash
    }

    // Determine which peer to prune from the network by finding the element with the lowest pruning score out of
    // immunity period. If all modules are in immunity period, return node with lowest prunning score.
    // This function will always return an element to prune.

    pub fn get_prune_score_for_uid() {
        let pruning_score:u16 = Self::get_emission_for_uid( module_uid_i );
        return pruning_score;
    }

    pub fn get_module_to_prune() -> u16 {
        let mut min_score : u16 = u16::MAX;
        let mut min_score_in_immunity_period = u16::MAX;
        let mut uid_with_min_score = 0;
        let mut uid_with_min_score_in_immunity_period: u16 =  0;
        if Self::get_n() == 0 { return 0 } // If there are no modules in this network.
        for module_uid_i in 0..Self::get_n() {
            // we set the pruning score to the lowest emmisions
            let pruning_score:u16 = Self::get_emission_for_uid( module_uid_i );
            let block_at_registration: u64 = Self::get_module_block_at_registration( module_uid_i );
            let current_block :u64 = Self::get_current_block_as_u64();
            let immunity_period: u64 = Self::get_immunity_period() as u64;
            if min_score == pruning_score {
                if current_block - block_at_registration <  immunity_period { //module is in immunity period
                    if min_score_in_immunity_period > pruning_score {
                        min_score_in_immunity_period = pruning_score; 
                        uid_with_min_score_in_immunity_period = module_uid_i;
                    }
                }
                else {
                    min_score = pruning_score; 
                    uid_with_min_score = module_uid_i;
                }
            }
            // Find min pruning score.
            else if min_score > pruning_score { 
                if current_block - block_at_registration <  immunity_period { //module is in immunity period
                    if min_score_in_immunity_period > pruning_score {
                         min_score_in_immunity_period = pruning_score; 
                        uid_with_min_score_in_immunity_period = module_uid_i;
                    }
                }
                else {
                    min_score = pruning_score; 
                    uid_with_min_score = module_uid_i;
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

    // ---- The implementation for the extrinsic update_module which sets the ip endpoint information for a uid on a network.
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
    // 		- the name of the module.
    // 
    // 	* 'context' (Vec[u8]):
    // 		- Any context that can be put as a string (json serializable objects count too)
    // 

    // # Event:
    // 	* ModuleServed;
    // 		- On successfully serving the module info.
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
    pub fn do_update_module( 
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

        // --- 4. Get the previous module information.
        let mut prev_module = Self::get_module_info( &key );  

            
        if (name.len() > 0) {
            ensure!(!Self::name_exists( name.clone()) , Error::<T>::ModuleNameAlreadyExists); 
            prev_module.name = name.clone();
            let uid = ModuleNamespace::<T>::get(prev_name);
            ModuleNamespace::<T>::insert( name.clone(), uid );
        }

        if (ip.len() > 0) {
            ensure!( Self::is_valid_ip_address(ip), Error::<T>::InvalidIpType );
            prev_module.ip = ip;
        }
        if (port > 0) {
            prev_module.port = port;

        }
        if (name.len() > 0) {
            prev_module.context = context.clone();

        }
        
        // set the serve block
        let current_block: u64 = Self::get_current_block_as_u64(); 
        prev_module.serve_block = current_block;

        Modules::<T>::insert( key.clone(), prev_module.clone() );
        let prev_name  = prev_module.name.clone();
        
        // --- 7. We deposit module served event.
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

    pub fn module_passes_rate_limit( prev_module_info: &ModuleInfo, current_block: u64 ) -> bool {
        let rate_limit: u64 = Self::get_serving_rate_limit(netuid);
        let last_serve = prev_module_info.block;
        return rate_limit == 0 || last_serve == 0 || current_block - last_serve >= rate_limit;
    }



    pub fn has_module_info( key: &T::AccountId ) -> bool {
        return Modules::<T>::contains_key( key );
    }


    pub fn get_module_info( key: &T::AccountId ) -> ModuleInfo {
        if Self::has_module_info( key ) {
            return Modules::<T>::get( key ).unwrap();
        } else{
            return ModuleInfo { 
                serve_block: 0,
                register_block: 0,
                ip: 0,
                port: 0,
                name: vec![],

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