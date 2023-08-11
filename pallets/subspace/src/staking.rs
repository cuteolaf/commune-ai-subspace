use super::*;
use substrate_fixed::types::{I64F64, I32F32};
// import vec
use sp_std::vec::Vec;

impl<T: Config> Pallet<T> { 

`
    // pub fn do_add_controller(
    //     origin: T::RuntimeOrigin,
    //     controller: T::AccountId,
    // ) -> dispatch::DispatchResult {
    //     let key = ensure_signed( origin.clone() )?;
    //     ensure!(Self::is_registered( netuid, &key.clone() ), Error::<T>::NotRegistered);
    //     ensure!(!Self::is_registered( netuid, &controller.clone() ), Error::<T>::AlreadyRegistered);
    //     Controller2Key::<T>::mutate(&key);
    //     Key2Controller::<T>::mutate(&key);
    //     Ok(())
    
    // }


    pub fn do_add_stake_multiple(
        origin: T::RuntimeOrigin,
        netuid: u16,
        module_keys: Vec<T::AccountId>,
        amounts: Vec<u64>
    ) -> dispatch::DispatchResult {
        let key = ensure_signed( origin.clone() )?;
        let amounts_sum: u64 = amounts.iter().sum();
        ensure!(Self::has_enough_balance( &key, amounts_sum), Error::<T>::NotEnoughStaketoWithdraw);
        ensure!(amounts.len() == module_keys.len(), Error::<T>::DifferentLengths);

        for (i,m_key) in module_keys.iter().enumerate() {
            Self::do_add_stake(origin.clone(),netuid, m_key.clone(), amounts[i as usize])?; 
        }
        Ok(())
    
    }
    //
	pub fn do_add_stake(
        origin: T::RuntimeOrigin, 
        netuid: u16,
        module_key: T::AccountId,
        amount: u64
    ) -> dispatch::DispatchResult {
        // --- 1. We check that the transaction is signed by the caller and retrieve the T::AccountId key information.
        let key = ensure_signed( origin )?;
        

		// --- 1. Ensure we don't exceed tx rate limit
		// ensure!( !Self::exceeds_tx_rate_limit(&key), Error::<T>::TxRateLimitExceeded);
        
        ensure!( Self::is_registered( netuid, &module_key.clone() ), Error::<T>::NotRegistered );  

        log::info!("do_add_stake( origin:{:?} stake_to_be_added:{:?} )", key, amount );
        
        ensure!( Self::can_remove_balance_from_account( &key, amount ), Error::<T>::NotEnoughBalanceToStake );

        Self::add_stake_to_module(netuid, &key, &module_key, amount );
 
        // --- 5. Emit the staking event.
        log::info!("StakeAdded( key:{:?}, stake_to_be_added:{:?} )", key, amount );
        Self::deposit_event( Event::StakeAdded( key, module_key, amount) );

        // --- 6. Ok and return.
        Ok(())
    }

    pub fn do_remove_stake(
        origin: T::RuntimeOrigin, 
        netuid: u16,
        module_key: T::AccountId,
        amount: u64
    ) -> dispatch::DispatchResult {

        // --- 1. We check the transaction is signed by the caller and retrieve the T::AccountId key information.
        let key = ensure_signed( origin )?;
        log::info!("do_remove_stake( origin:{:?} stake_to_be_removed:{:?} )", key, amount );


        ensure!( Self::is_registered( netuid, &module_key.clone() ), Error::<T>::NotRegistered );  

		// --- 6. Ensure we don't exceed tx rate limit
		// ensure!( !Self::exceeds_tx_rate_limit(&key), Error::<T>::TxRateLimitExceeded );

        // --- 5. Ensure that we can conver this u64 to a balance.
        ensure!( Self::has_enough_stake(netuid, &key , &module_key, amount ), Error::<T>::NotEnoughStaketoWithdraw );
        let stake_to_be_added_as_currency = Self::u64_to_balance( amount );
        ensure!( stake_to_be_added_as_currency.is_some(), Error::<T>::CouldNotConvertToBalance );

        // --- 7. We remove the balance from the key.
        Self::remove_stake_from_module(netuid,  &key, &module_key, amount );

        // --- 9. Emit the unstaking event.
        log::info!("StakeRemoved( key:{:?}, stake_to_be_removed:{:?} )", key, amount );
        Self::deposit_event( Event::StakeRemoved( key, module_key, amount ) );

        // --- 10. Done and ok.
        Ok(())
    }

    // Returns the total amount of stake in the staking table.
    //
    pub fn get_total_subnet_stake(netuid:u16) -> u64 { 
        return SubnetTotalStake::<T>::get(netuid);
    }

    // Returns the total amount of stake in the staking table.
    pub fn get_total_stake() -> u64 { 
        return TotalStake::<T>::get();
    }

    // Returns the stake under the cold - hot pairing in the staking table.
    //
    pub fn get_stake(netuid:u16, key: &T::AccountId ) -> u64 { 
        return Stake::<T>::get(netuid,  key );
    }
    
    // Returns the stake under the cold - hot pairing in the staking table.
    pub fn key_account_exists(netuid:u16, key : &T::AccountId) -> bool {
        return Uids::<T>::contains_key(netuid, &key) ; 
    }

    // Returns true if the cold-hot staking account has enough balance to fufil the amount.
    //
    pub fn has_enough_stake(netuid: u16, key: &T::AccountId,  module_key: &T::AccountId, amount: u64 ) -> bool {
        return Self::get_stake_to_module(netuid , key, module_key ) >= amount;
    }

    pub fn get_stake_to_module(netuid:u16, key: &T::AccountId, module_key: &T::AccountId ) -> u64 { 
        
        let mut state_to : u64 = 0;
        for (k, v) in Self::get_stake_to_vector(netuid, key) {
            if k == module_key.clone() {
                state_to = v;
            }
        }

        return state_to;
    }



    pub fn get_stake_to_vector(netuid:u16, key:&T::AccountId, ) -> Vec<(T::AccountId, u64)> { 
        return StakeTo::<T>::get(netuid, key);
    }

    pub fn set_stake_to_vector(netuid:u16, key:&T::AccountId, stake_to_vector: Vec<(T::AccountId, u64)>) { 
        
        // we want to remove any keys that have a stake of 0, as these are from outside the subnet and can bloat the chain
        if stake_to_vector.len() == 0 {
            StakeTo::<T>::remove(netuid, key);
            return;
        }
        StakeTo::<T>::insert(netuid, key, stake_to_vector);
    }


    pub fn set_stake_from_vector(netuid:u16, module_key: &T::AccountId, stake_from_vector: Vec<(T::AccountId, u64)>) { 
        StakeFrom::<T>::insert(netuid, module_key, stake_from_vector);
    }

    pub fn get_stake_from_vector(netuid:u16, module_key: &T::AccountId ) -> Vec<(T::AccountId, u64)> { 
        return StakeFrom::<T>::get(netuid, module_key).into_iter().collect::<Vec<(T::AccountId, u64)>>();
    }
    pub fn get_total_stake_from(netuid:u16, module_key : &T::AccountId ) ->  u64 { 
        let stake_from_vector: Vec<(T::AccountId, u64)> = Self::get_stake_from_vector(netuid, module_key);
        let mut total_stake_from: u64 = 0;
        for (k, v) in stake_from_vector {
            total_stake_from += v;
        }
        return total_stake_from;
    }
    pub fn get_total_stake_to(netuid:u16, key:&T::AccountId, ) -> u64 { 
        let mut stake_to_vector: Vec<(T::AccountId, u64)> = Self::get_stake_to_vector(netuid, key);
        let mut total_stake_to: u64 = 0;
        for (k, v) in stake_to_vector {
            total_stake_to += v;
        }
        let module_stake: u64 = Self::get_stake(netuid, key);
        return total_stake_to;
    }

    // INCREASE   

    pub fn increase_stake_to_module(netuid: u16, key: &T::AccountId,  module_key: &T::AccountId, amount: u64 ) -> bool{


        let mut stake_from_vector: Vec<(T::AccountId, u64)> = Self::get_stake_from_vector(netuid, module_key);
        let mut found_key_in_vector:bool= false;
        for (i, (k, v)) in stake_from_vector.clone().iter().enumerate() {
            if k == key {
                stake_from_vector[i] = (k.clone(), *v + amount);
                found_key_in_vector = true;
            }
        }
        if !found_key_in_vector {
            stake_from_vector.push( (key.clone(), amount) );
        }

        let mut found_key_in_vector:bool= false;
        let mut stake_to_vector: Vec<(T::AccountId, u64)> = Self::get_stake_to_vector(netuid, key);

        for (i, (k, v)) in stake_to_vector.clone().iter().enumerate() {
            if k == module_key {
                stake_to_vector[i] = (k.clone(), *v + amount);
                found_key_in_vector = true;
            }
        }

        if !found_key_in_vector {
            stake_to_vector.push( (module_key.clone(), amount) );
        }

        Self::set_stake_to_vector(netuid, key, stake_to_vector);
        Self::set_stake_from_vector(netuid, module_key, stake_from_vector);
        Self::increase_stake_on_account(netuid, module_key, amount);
        
        return true;

    }

    pub fn add_stake_to_module(netuid: u16, key: &T::AccountId, module_key: &T::AccountId, amount: u64 ) -> bool{
        Self::increase_stake_to_module(netuid, key, module_key, amount);
        Self::remove_balance_from_account( key, Self::u64_to_balance( amount ).unwrap() );
        
        return true;

    }






    pub fn remove_stake_from_module(netuid: u16, key: &T::AccountId, module_key: &T::AccountId, amount: u64 ) -> bool{
        Self::decrease_stake_from_module(netuid, key, module_key, amount);
        Self::add_balance_to_account( key, Self::u64_to_balance( amount ).unwrap() );
        
        return true;

    }



    pub fn decrease_stake_from_module(netuid: u16, key: &T::AccountId, module_key: &T::AccountId, amount: u64 ) -> bool{

        // FROM DELEGATE STAKE
        let mut stake_to_vector: Vec<(T::AccountId, u64)> = Self::get_stake_to_vector(netuid, key);
        let mut stake_from_vector: Vec<(T::AccountId, u64)> = Self::get_stake_from_vector(netuid, module_key).clone();

        let mut idx_to_replace:usize = usize::MAX;

        let mut end_idx:usize = stake_from_vector.len() - 1;
        for (i, (m_key, m_stake_amount)) in stake_from_vector.clone().iter().enumerate() {
            if *m_key == *key {
                let remaining_stake: u64 = *m_stake_amount - amount;
                stake_from_vector[i] = (m_key.clone(), remaining_stake);
                if remaining_stake == 0 {
                    // we need to remove this entry
                    idx_to_replace = i;
                }

            }
        }
        if idx_to_replace != usize::MAX {
            stake_from_vector.remove(idx_to_replace);
        }



        // TO STAKE 
        idx_to_replace = usize::MAX;
        end_idx = stake_to_vector.len() - 1;

        for (i, (k, v)) in stake_to_vector.clone().iter().enumerate() {
            if k == module_key {
                let remaining_stake: u64 = *v - amount;
                stake_to_vector[i] = (k.clone(), remaining_stake);
                if remaining_stake == 0 {
                    idx_to_replace = i;
                }
            }
        }

        if idx_to_replace != usize::MAX {
            stake_to_vector.remove(idx_to_replace);
        }

        
        Self::set_stake_to_vector(netuid, key, stake_to_vector);
        Self::set_stake_from_vector(netuid, module_key, stake_from_vector);
        Self::decrease_stake_on_account( netuid, module_key, amount );
        
        return true;

    }

    pub fn increase_stake_on_account(netuid:u16, key: &T::AccountId, amount: u64 ){
        Stake::<T>::insert(netuid, key, Stake::<T>::get(netuid, key).saturating_add( amount ) );
        SubnetTotalStake::<T>::insert(netuid , SubnetTotalStake::<T>::get(netuid).saturating_add( amount ) );
        TotalStake::<T>::put(TotalStake::<T>::get().saturating_add( amount ) );
        

    }

    // Decreases the stake on the cold - hot pairing by the amount while decreasing other counters.
    //
    pub fn decrease_stake_on_account(netuid:u16, key: &T::AccountId, amount: u64 ) {
        // --- 8. We add the balancer to the key.  If the above fails we will not credit this key.
        Stake::<T>::insert( netuid, key, Stake::<T>::get(netuid,  key).saturating_sub( amount ) );
        TotalStake::<T>::put(TotalStake::<T>::get().saturating_sub( amount ) );
        SubnetTotalStake::<T>::insert(netuid, SubnetTotalStake::<T>::get(netuid).saturating_sub( amount ) );
    }

    // Decreases the stake on the cold - hot pairing by the amount while decreasing other counters.
    //
    pub fn remove_stake_from_storage(netuid:u16, key: &T::AccountId ) {

        let stake_from_vector: Vec<(T::AccountId, u64)> = Self::get_stake_from_vector(netuid, key);
        for (i, (m_key, m_stake_amount)) in stake_from_vector.iter().enumerate() {
            Self::remove_stake_from_module(netuid, m_key, key, *m_stake_amount);
        }

        StakeFrom::<T>::remove(netuid, key);
        Stake::<T>::remove(netuid, &key);
    }

	pub fn u64_to_balance( input: u64 ) -> Option<<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance> { input.try_into().ok() }

    pub fn add_balance_to_account(key: &T::AccountId, amount: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance) {
        T::Currency::deposit_creating(&key, amount); // Infallibe
    }

    pub fn set_balance_on_account(key: &T::AccountId, amount: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance) {
        T::Currency::make_free_balance_be(&key, amount); 
    }

    pub fn can_remove_balance_from_account(key: &T::AccountId, amount_64: u64) -> bool {
        let amount_as_balance = Self::u64_to_balance( amount_64 );
        if amount_as_balance.is_none() {
            return false;
        }
        let amount = amount_as_balance.unwrap();
        let current_balance = Self::get_balance(key);
        if amount > current_balance {
            return false;
        }
        // This bit is currently untested. @todo
        let new_potential_balance = current_balance - amount;
        let can_withdraw : bool = T::Currency::ensure_can_withdraw(&key, amount, WithdrawReasons::except(WithdrawReasons::TIP), new_potential_balance).is_ok();
        can_withdraw
    }

    pub fn get_balance(key: &T::AccountId) -> <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance {
        return T::Currency::free_balance(&key);
    }

    pub fn balance_to_u64( input: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance) -> u64 { input.try_into().ok().unwrap() }

    pub fn get_balance_as_u64(key: &T::AccountId) -> u64 {
        return Self::balance_to_u64( Self::get_balance(key) );
    }

    pub fn has_enough_balance(key: &T::AccountId, amount: u64 ) -> bool {
        return Self::get_balance_as_u64(key) >= amount;
    }

    pub fn resolve_stake_amount(key: &T::AccountId, stake: u64 ) -> u64 {
        let balance = Self::get_balance_as_u64(key);
        if balance <= stake {
            return balance;
        } else {
            return stake;
        }
    }


    pub fn remove_balance_from_account(key: &T::AccountId, amount: <<T as Config>::Currency as Currency<<T as system::Config>::AccountId>>::Balance) -> bool {
        return match T::Currency::withdraw(&key, amount, WithdrawReasons::except(WithdrawReasons::TIP), ExistenceRequirement::KeepAlive) {
            Ok(_result) => {
                true
            }
            Err(_error) => {
                false
            }
        };
    }

}