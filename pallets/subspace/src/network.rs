use super::*;
use frame_support::{sp_std::vec};
use sp_std::vec::Vec;
use frame_system::ensure_root;
use frame_support::IterableStorageDoubleMap;
use frame_support::storage::IterableStorageMap;
use frame_support::pallet_prelude::{Decode, Encode};
use codec::Compact;
use frame_support::pallet_prelude::{DispatchError, DispatchResult};
use substrate_fixed::types::{I64F64};
extern crate alloc;



impl<T: Config> Pallet<T> { 


    // Returns true if the subnetwork exists.
    //
    pub fn if_subnet_exist( netuid: u16 ) -> bool{
        return N::<T>::contains_key( netuid );
    }

    // get the least staked network
    pub fn least_staked_netuid(stake: u64) -> u16 {
        let mut min_stake: u64 = 0;
        let mut min_stake_netuid: u16 = 0;
        if stake > min_stake {
            for ( netuid, net_stake ) in <SubnetTotalStake<T> as IterableStorageMap<u16, u64> >::iter(){
                if net_stake <= min_stake {
                    min_stake = net_stake;
                    min_stake_netuid = netuid;
                }
            }
        }
        return min_stake_netuid;
    }


    pub fn get_network_stake( netuid: u16 ) -> u64 {
        return SubnetTotalStake::<T>::get( netuid );
    }


    pub fn do_remove_network( 
        origin: T::RuntimeOrigin,
        netuid: u16,
    ) -> DispatchResult {

        let key = ensure_signed(origin)?;
        // --- 1. Ensure the network name does not already exist.
            
        ensure!( Self::if_subnet_netuid_exists( netuid ), Error::<T>::SubnetNameAlreadyExists );
        ensure!( Self::is_subnet_founder( netuid, &key ), Error::<T>::NotSubnetFounder );

        Self::remove_network_for_netuid( netuid );
        // --- 16. Ok and done.
        Ok(())
    }

    pub fn do_update_network( 
        origin: T::RuntimeOrigin,
        netuid: u16,
        stake: u64,
        immunity_period: u16,
        min_weights: u16,
        max_n: u16,
        tempo: u16,
        founder: T::AccountId,
    ) -> DispatchResult {

        let key = ensure_signed(origin)?;

        ensure!( Self::if_subnet_netuid_exists( netuid ), Error::<T>::SubnetNameAlreadyExists );
        ensure!( Self::is_subnet_founder( netuid, &key ), Error::<T>::NotSubnetFounder );

        Self::update_network_for_netuid( netuid, stake, immunity_period, min_weights, max_n, tempo, founder);
        // --- 16. Ok and done.
        Ok(())
    }


    pub fn update_network_for_netuid(netuid: u16,
                    stake: u64,
                    immunity_period: u16,
                    min_weights: u16,
                    max_n: u16,
                    tempo: u16,
                    founders: Vec<T::AccountId>,) {
        Tempo::<T>::insert( netuid, tempo);
        MaxN::<T>::insert( netuid, max_n );
        ImmunityPeriod::<T>::insert( netuid, immunity_period );
        MinWeights::<T>::insert( netuid, min_weights );
        Founders::<T>::insert( netuid, founders );
    }



    pub fn default_subnet() -> SubnetInfo {
        let netuid: u16 = 0;
        return SubnetInfo {
            immunity_period: MinWeights::<T>::get( netuid ),
            min_weights: ImmunityPeriod::<T>::get( netuid ),
            max_n:  MaxN::<T>::get( netuid ),
            tempo: Tempo::<T>::get( netuid ),
            n: N::<T>::get( netuid ),
            netuid: netuid,
            stake: SubnetTotalStake::<T>::get( netuid ),
            name : Self::get_name_for_netuid( netuid ),
            emission: SubnetEmission::<T>::get( netuid ),
        
        };

            
        
    }


    pub fn is_subnet_founder( netuid: u16, key: &T::AccountId ) -> bool {
        return Founders::<T>::get( netuid) == *key;
    }





    pub fn add_network_from_registration( 
        name: Vec<u8>,
        stake: u64,
        key : &T::AccountId,
    ) -> u16 {


        let default_subnet  = Self::default_subnet();

        let netuid = Self::add_network( 
                            name.clone(),
                            default_subnet.stake + stake, 
                            default_subnet.max_n, 
                            default_subnet.immunity_period,
                            default_subnet.min_weights,
                            default_subnet.tempo,
                            &key, );

        // --- 16. Ok and done.
        return netuid;
    }
    // Returns the total amount of stake in the staking table.
    //
    pub fn calculate_network_emission(netuid:u16) -> u64 { 

        let subnet_stake: I64F64 =I64F64::from_num( Self::get_total_subnet_stake(netuid) + 1);
        let total_stake: I64F64 = I64F64::from_num(Self::get_total_stake());

        let mut subnet_ratio: I64F64 = I64F64::from_num(0);
        if total_stake > I64F64::from_num(0) {
            subnet_ratio =  subnet_stake/total_stake;
        } else {
            let n = TotalSubnets::<T>::get();
            subnet_ratio = I64F64::from_num(1)/I64F64::from_num(n);
        }
        
        let token_emission: u64 = (subnet_ratio* I64F64::from_num(1_000_000_000)).to_num::<u64>();
        
        SubnetEmission::<T>::insert( netuid, token_emission );

        return token_emission;

    }


    pub fn add_network( 
                       name: Vec<u8>,
                       stake: u64,
                       max_n: u16,
                       immunity_period: u16,
                       min_weights: u16,
                       tempo: u16,
                       founders: Vec<T::AccountId>, 
                    ) -> u16 {

        // --- 1. Enfnsure that the network name does not already exist.
        let total_networks = TotalSubnets::<T>::get();
        let max_networks = MaxAllowedSubnets::<T>::get();
        // if networks exceeds max_networks, remove the least staked network
        let netuid : u16 ; 
        if total_networks > max_networks {
            netuid = Self::least_staked_netuid(stake);
            Self::remove_network_for_netuid( netuid );
        } else {
            netuid = total_networks + 1;

        }

        Tempo::<T>::insert( netuid, tempo);
        MaxN::<T>::insert( netuid, max_n );
        ImmunityPeriod::<T>::insert( netuid, immunity_period );
        MinWeights::<T>::insert( netuid, min_weights );
        SubnetNamespace::<T>::insert( name.clone(), netuid );
        Founders::<T>::insert( netuid,founders);

        // set stat once network is created
        TotalSubnets::<T>::mutate( |n| *n += 1 );
        N::<T>::insert( netuid, 0 );
        
        // --- 6. Emit the new network event.
        log::info!("NetworkAdded( netuid:{:?}, name:{:?} )", netuid, name.clone());
        Self::deposit_event( Event::NetworkAdded( netuid, name.clone()) );
    

        return netuid;

    }



    // Initializes a new subnetwork under netuid with parameters.
    //
    pub fn if_subnet_name_exists(name: Vec<u8>) -> bool {
        return  SubnetNamespace::<T>::contains_key(name.clone()).into();
    }

    pub fn if_subnet_netuid_exists(netuid: u16) -> bool {
       
   
        return  SubnetNamespace::<T>::contains_key(Self::get_name_for_netuid(netuid)).into();
    }


    pub fn get_netuid_for_name( name: Vec<u8> ) -> u16 {
        
        let netuid: u16 = SubnetNamespace::<T>::get(name.clone());
        return netuid;
    }


    pub fn get_name_for_netuid( netuid : u16) -> Vec<u8> {
        for ( name, _netuid ) in <SubnetNamespace<T> as IterableStorageMap<Vec<u8>, u16>>::iter(){
            if _netuid == netuid {
                return name;
            }
        }
        return Vec::new();
    }




    // Removes the network (netuid) and all of its parameters.
    //
    pub fn remove_network_for_netuid( netuid: u16 ) -> u16 {
        let name = Self::get_name_for_netuid( netuid );
        return Self::remove_network_for_name( name );
    }

    // Returns true if the account is the founder of the network.
    pub fn is_network_founder( netuid: u16, key: &T::AccountId ) -> bool {
        let founder = Founder::<T>::get( netuid );
        return founder == key.clone();
    }


    pub fn remove_network_for_name( name: Vec<u8>) -> u16 {
        // --- 2. Ensure the network to be removed exists.
        if !Self::if_subnet_name_exists( name.clone() ) {
            return 0;
        }
        let netuid = Self::get_netuid_for_name( name.clone() );
        SubnetNamespace::<T>::remove( name.clone() );

    
        for ( uid, key ) in <Keys<T> as IterableStorageDoubleMap<u16, u16, T::AccountId>>::iter_prefix( netuid) {
            Stake::<T>::remove(key.clone());
        }

        Uids::<T>::clear_prefix( netuid, u32::max_value(), None );
        Keys::<T>::clear_prefix( netuid, u32::max_value(), None );
        Weights::<T>::clear_prefix( netuid, u32::max_value(), None );
        Emission::<T>::remove( netuid );
        Incentive::<T>::remove( netuid );
        Dividends::<T>::remove( netuid );
        LastUpdate::<T>::remove( netuid );
        Founder::<T>::remove( netuid );

        // --- 2. Erase network parameters.
        Tempo::<T>::remove( netuid );
        MaxN::<T>::remove( netuid );
        ImmunityPeriod::<T>::remove( netuid );
        MinWeights::<T>::remove( netuid );
        N::<T>::remove( netuid );
        // --- 4. Remove all stake.

        Stake::<T>::remove( netuid );
        SubnetTotalStake::<T>::remove( netuid );
        TotalSubnets::<T>::mutate(|val| *val -= 1);
        // --- 4. Emit the event.
        log::info!("NetworkRemoved( netuid:{:?} )", netuid);
        Self::deposit_event( Event::NetworkRemoved( netuid ) );

        return netuid;
        

    }



	pub fn get_subnet_info(netuid: u16) -> Option<SubnetInfo> {
        if !Self::if_subnet_exist(netuid) {
            return None;
        }

        let immunity_period = Self::get_immunity_period(netuid);
        let name = Self::get_name_for_netuid(netuid);
        let min_weights = Self::get_min_weights(netuid);
        let n = Self::get_subnetwork_n(netuid);
        let max_n = Self::get_max_n(netuid);
        let tempo = Self::get_tempo(netuid);



        return Some(SubnetInfo {
            immunity_period: immunity_period.into(),
            name: name,
            netuid: netuid.into(),
            min_weights: min_weights.into(),
            n: n.into(),
            max_n: max_n.into(),
            tempo: tempo.into(),
            emission: SubnetEmission::<T>::get(netuid).into(),
            stake: SubnetTotalStake::<T>::get(netuid).into(),
            
        })
	}

    pub fn get_subnets_info() -> Vec<Option<SubnetInfo>> {
        let mut subnets_info = Vec::<Option<SubnetInfo>>::new();
        for ( netuid, net_n ) in < N<T> as IterableStorageMap<u16, u16> >::iter() {
            subnets_info.push(Self::get_subnet_info(netuid));
        }
        return subnets_info;
	}


    // Returns the number of filled slots on a network.
    ///
    pub fn get_subnetwork_n( netuid:u16 ) -> u16 { 
        return N::<T>::get( netuid ) 
    }
    

  

    // Returns true if the uid is set on the network.
    //
    pub fn is_uid_exist_on_network(netuid: u16, uid: u16) -> bool {
        return  Keys::<T>::contains_key(netuid, uid);
    }

    // Returns true if the key holds a slot on the network.
    //
    pub fn is_key_registered_on_network( netuid:u16, key: &T::AccountId ) -> bool { 
        return Uids::<T>::contains_key( netuid, key ) 
    }

    // Returs the key under the network uid as a Result. Ok if the uid is taken.
    //
    pub fn get_key_for_uid( netuid: u16, module_uid: u16) ->  T::AccountId {
        Keys::<T>::try_get(netuid, module_uid).unwrap() 
    }
    

    // Returns the uid of the key in the network as a Result. Ok if the key has a slot.
    //
    pub fn get_uid_for_key( netuid: u16, key: &T::AccountId) -> Result<u16, DispatchError> { 
        return Uids::<T>::try_get(netuid, &key).map_err(|_err| Error::<T>::NotRegistered.into()) 
    }

    pub fn get_uid_for_name ( netuid: u16, name: Vec<u8> ) -> u16  {
        return Namespace::<T>::get(netuid, name)
    }

    pub fn get_name_for_uid ( netuid: u16, uid: u16 ) -> Vec<u8>  {
        return Names::<T>::get(netuid, uid);
    }


    pub fn if_module_name_exists( netuid: u16, name: Vec<u8> ) -> bool {
        return Namespace::<T>::contains_key( netuid, name.clone() );
        
    }

    // Returns the stake of the uid on network or 0 if it doesnt exist.
    //
    pub fn get_stake_for_uid( netuid: u16, module_uid: u16) -> u64 { 
        return Self::get_stake_for_key( netuid, &Self::get_key_for_uid( netuid, module_uid) )
    }

    pub fn get_stake_for_key( netuid: u16,  key: &T::AccountId) -> u64 { 
        stake: u16 = 0;
        for ( netuid, stated_amount ) in <Stake<T> as IterableStorageDoubleMap<T::AccountId, T::AccountId, u64> >::iter_prefix(key){
            stake += stated_amount ;
        }
        return stake;
    }
    
    
    // Return the total number of subnetworks available on the chain.
    //
    pub fn get_number_of_subnets()-> u16 {
        let mut number_of_subnets : u16 = 0;
        for (_, _)  in <N<T> as IterableStorageMap<u16, u16>>::iter(){
            number_of_subnets = number_of_subnets + 1;
        }
        return number_of_subnets;
    }


    // ========================
	// ==== Global Setters ====
	// ========================
    pub fn set_tempo( netuid: u16, tempo: u16 ) { Tempo::<T>::insert( netuid, tempo ); }

    pub fn set_registrations_this_block( netuid: u16, registrations_this_block: u16 ) { RegistrationsThisBlock::<T>::insert(netuid, registrations_this_block); }

    
    // ========================
	// ==== Global Getters ====
	// ========================
    pub fn get_current_block_as_u64( ) -> u64 { TryInto::try_into( <frame_system::Pallet<T>>::block_number() ).ok().expect("blockchain will not exceed 2^64 blocks; QED.") }

    // ==============================
	// ==== Yomama params ====
	// ==============================
    pub fn get_emission( netuid:u16 ) -> Vec<u64> { Emission::<T>::get( netuid ) }
    pub fn get_incentive( netuid:u16 ) -> Vec<u16> { Incentive::<T>::get( netuid ) }
    pub fn get_dividends( netuid:u16 ) -> Vec<u16> { Dividends::<T>::get( netuid ) }
    pub fn get_last_update( netuid:u16 ) -> Vec<u64> { LastUpdate::<T>::get( netuid ) }
    
    // Emission is the same as the Yomama params 
    
    pub fn set_last_update_for_uid( netuid:u16, uid: u16, last_update: u64 ) { 
        let mut updated_last_update_vec = Self::get_last_update( netuid ); 
        if (uid as usize) < updated_last_update_vec.len() { 
            updated_last_update_vec[uid as usize] = last_update;
            LastUpdate::<T>::insert( netuid, updated_last_update_vec );
        }  
    }

    pub fn get_emission_for_uid( netuid:u16, uid: u16) -> u64 {let vec =  Emission::<T>::get( netuid ); if (uid as usize) < vec.len() { return vec[uid as usize] } else{ return 0 } }
    pub fn get_incentive_for_uid( netuid:u16, uid: u16) -> u16 { let vec = Incentive::<T>::get( netuid ); if (uid as usize) < vec.len() { return vec[uid as usize] } else{ return 0 } }
    pub fn get_dividends_for_uid( netuid:u16, uid: u16) -> u16 { let vec = Dividends::<T>::get( netuid ); if (uid as usize) < vec.len() { return vec[uid as usize] } else{ return 0 } }
    pub fn get_last_update_for_uid( netuid:u16, uid: u16) -> u64 { let vec = LastUpdate::<T>::get( netuid ); if (uid as usize) < vec.len() { return vec[uid as usize] } else{ return 0 } }
    pub fn get_pruning_score_for_uid( netuid:u16, uid: u16) -> u16 { let vec = Emission::<T>::get( netuid ); if (uid as usize) < vec.len() { return vec[uid as usize] as u16 } else{ return u16::MAX } }


    // ============================
	// ==== Subnetwork Getters ====
	// ============================
    pub fn get_tempo( netuid:u16 ) -> u16{ Tempo::<T>::get( netuid ) }
    pub fn get_pending_emission( netuid:u16 ) -> u64{ PendingEmission::<T>::get( netuid ) }
    pub fn get_registrations_this_block( netuid:u16 ) -> u16 { RegistrationsThisBlock::<T>::get( netuid ) }
    pub fn get_module_block_at_registration( netuid: u16, module_uid: u16 ) -> u64 { BlockAtRegistration::<T>::get( netuid, module_uid )}

    // ========================
	// ==== Rate Limiting =====
	// ========================
	pub fn get_last_tx_block( key: &T::AccountId ) -> u64 { LastTxBlock::<T>::get( key ) }
    pub fn set_last_tx_block( key: &T::AccountId, last_tx_block: u64 ) { LastTxBlock::<T>::insert( key, last_tx_block ) }

	// Configure tx rate limiting
	pub fn get_tx_rate_limit() -> u64 { TxRateLimit::<T>::get() }
    pub fn set_tx_rate_limit( tx_rate_limit: u64 ) { TxRateLimit::<T>::put( tx_rate_limit ) }

    pub fn get_immunity_period(netuid: u16 ) -> u16 { ImmunityPeriod::<T>::get( netuid ) }
    pub fn set_immunity_period( netuid: u16, immunity_period: u16 ) { ImmunityPeriod::<T>::insert( netuid, immunity_period ); }

    pub fn get_min_weights( netuid:u16 ) -> u16 {
        let min_weights = MinWeights::<T>::get( netuid ) ; 
        let n = Self::get_subnetwork_n(netuid);
        // if n < min_weights, then return n
        if (n < min_weights) {
            return n;
        } else {
            return min_weights;
        }
        }
    pub fn set_min_weights( netuid: u16, min_weights: u16 ) { MinWeights::<T>::insert( netuid, min_weights ); }

    pub fn get_max_n( netuid: u16 ) -> u16  { MaxN::<T>::get( netuid ) }
    pub fn set_max_n(netuid: u16, max_allowed: u16) { MaxN::<T>::insert( netuid, max_allowed ); }
            


    pub fn get_max_registrations_per_block( netuid: u16 ) -> u16 { MaxRegistrationsPerBlock::<T>::get( netuid ) }
    pub fn set_max_registrations_per_block( netuid: u16, max_registrations_per_block: u16 ) { MaxRegistrationsPerBlock::<T>::insert( netuid, max_registrations_per_block ); }

    pub fn is_registered(netuid: u16, key: &T::AccountId) -> bool {
        return Uids::<T>::contains_key(netuid, &key)
    }

    pub fn get_keys( netuid : u16) ->  Vec< T::AccountId>  {
        let mut keys: Vec< T::AccountId> = vec![];
        for ( uid, key ) in < Keys<T> as IterableStorageDoubleMap<u16, u16, T::AccountId >>::iter_prefix( netuid ) {
            keys.push( key ); 
        }
        return keys
    }
    pub fn get_uids( netuid : u16) ->  Vec<u16> {
        let mut uids: Vec<u16> = vec![];
        for ( uid, key ) in < Keys<T> as IterableStorageDoubleMap<u16, u16, T::AccountId >>::iter_prefix( netuid ) {
            uids.push( uid ); 
        }
        return uids
    }

}


    
