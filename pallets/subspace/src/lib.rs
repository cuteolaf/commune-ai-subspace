#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "512"]
// Edit this file to define custom logic or remove it if it is not needed.
// Learn more about FRAME and the core library of Substrate FRAME pallets:
// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

use frame_system::{
	self as system,
	ensure_signed
};

use frame_support::{
	dispatch,
	dispatch::{
		DispatchInfo,
		PostDispatchInfo
	}, ensure, 
	traits::{
		Currency, 
		ExistenceRequirement,
		tokens::{
			WithdrawReasons
		},
		IsSubType,
		}
};

use sp_std::marker::PhantomData;
use codec::{Decode, Encode};
use sp_runtime::{
	traits::{
		Dispatchable,
		DispatchInfoOf,
		SignedExtension,
		PostDispatchInfoOf
	},
	transaction_validity::{
		TransactionValidity,
		TransactionValidityError
	}
};
use scale_info::TypeInfo;
use frame_support::sp_runtime::transaction_validity::ValidTransaction;

// ============================
//	==== Benchmark Imports =====
// ============================
#[cfg(feature = "runtime-benchmarks")]
mod benchmarks;

// =========================
//	==== Pallet Imports =====
// =========================

mod epoch;
mod math;
mod network;
mod neuron;
mod registration;
mod staking;
mod weights;

pub mod neuron_info;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use frame_support::traits::Currency;
	use frame_support::sp_std::vec;
	use serde::{Serialize, Deserialize};
	use serde_with::{serde_as, DisplayFromStr};
	use frame_support::inherent::Vec;
	use scale_info::prelude::string::String;


	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		// --- Currency type that will be used to place deposits on neurons
		type Currency: Currency<Self::AccountId> + Send + Sync;

		// =================================
		// ==== Initial Value Constants ====
		// =================================
		#[pallet::constant] // Initial currency issuance.
		type InitialIssuance: Get<u64>;
		#[pallet::constant] // Initial min allowed weights setting.
		type InitialMinAllowedWeights: Get<u16>;
		#[pallet::constant] // Initial Emission Ratio
		type InitialEmissionValue: Get<u16>;
		#[pallet::constant] // Initial max weight limit.
		type InitialMaxWeightsLimit: Get<u16>;
		#[pallet::constant] // Tempo for each network
		type InitialTempo: Get<u16>;
		#[pallet::constant] // Initial adjustment interval.
		type InitialAdjustmentInterval: Get<u16>;
		#[pallet::constant] // Initial target registrations per interval.
		type InitialTargetRegistrationsPerInterval: Get<u16>;
		#[pallet::constant] // Max UID constant.
		type InitialMaxAllowedUids: Get<u16>;
		#[pallet::constant] // Immunity Period Constant.
		type InitialImmunityPeriod: Get<u16>;
		#[pallet::constant] // Activity constant
		type InitialActivityCutoff: Get<u16>;
		#[pallet::constant] // Initial max registrations per block.
		type InitialMaxRegistrationsPerBlock: Get<u16>;
		#[pallet::constant] // Initial pruning score for each neuron
		type InitialPruningScore: Get<u16>;	
		#[pallet::constant] // Initial serving rate limit.
		type InitialServingRateLimit: Get<u64>;
		#[pallet::constant] // Initial transaction rate limit.
		type InitialTxRateLimit: Get<u64>;
	}

	pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

	// ============================
	// ==== Staking + Accounts ====
	// ============================
	#[pallet::type_value] 
	pub fn DefaultAccountTake<T: Config>() -> u64 { 0 }
	#[pallet::type_value]
	pub fn DefaultBlockEmission<T: Config>() -> u64 {1_000_000_000}
	#[pallet::type_value] 
	pub fn DefaultAllowsDelegation<T: Config>() -> bool { false }
	#[pallet::type_value] 
	pub fn DefaultTotalIssuance<T: Config>() -> u64 { T::InitialIssuance::get() }
	#[pallet::type_value] 
	pub fn DefaultAccount<T: Config>() -> T::AccountId { T::AccountId::decode(&mut sp_runtime::traits::TrailingZeroInput::zeroes()).unwrap()}

	#[pallet::storage] // --- ITEM ( total_stake )
	pub type TotalStake<T> = StorageValue<_, u64, ValueQuery>;
	#[pallet::storage] // --- ITEM ( global_block_emission )
	pub type BlockEmission<T> = StorageValue<_, u64, ValueQuery, DefaultBlockEmission<T>>;
	#[pallet::storage] // --- ITEM ( total_issuance )
	pub type TotalIssuance<T> = StorageValue<_, u64, ValueQuery, DefaultTotalIssuance<T>>;
	#[pallet::storage] // --- VALUE --> stake | Returns the total amount of stake under a key.
    pub type TotalKeyStake<T:Config> = StorageValue<_, u64, ValueQuery, DefaultAccountTake<T>>;
	#[pallet::storage] // --- MAP ( key ) --> stake | Returns the stake under a key prefixed by key.
	pub type Stake<T:Config> = StorageValue<_, u64, ValueQuery, DefaultAccountTake<T>>;

	#[pallet::type_value] 
	pub fn DefaultLastAdjustmentBlock<T: Config>() -> u64 { 0 }
	#[pallet::type_value]
	pub fn DefaultRegistrationsThisBlock<T: Config>() ->  u16 { 0}
	#[pallet::type_value] 
	pub fn DefaultMaxRegistrationsPerBlock<T: Config>() -> u16 { T::InitialMaxRegistrationsPerBlock::get() }
	#[pallet::storage] // -->  Block at last adjustment.
	pub type LastAdjustmentBlock<T> = StorageValue<_, u64, ValueQuery, DefaultLastAdjustmentBlock<T> >;
	#[pallet::storage] // ) --> Registration this Block.
	pub type RegistrationsThisBlock<T> = StorageValue<_,  u16, ValueQuery, DefaultRegistrationsThisBlock<T>>;
	#[pallet::storage] // --- ITEM( global_max_registrations_per_block ) 
	pub type MaxRegistrationsPerBlock<T> = StorageValue<_,  u16, ValueQuery, DefaultMaxRegistrationsPerBlock<T> >;

	// ==============================
	// ==== Subnetworks Storage =====
	// ==============================
	#[pallet::type_value] 
	pub fn DefaultN<T:Config>() -> u16 { 0 }
	#[pallet::type_value] 
	pub fn DefaultKeys<T:Config>() -> Vec<u16> { vec![ ] }
	#[pallet::type_value]


	#[pallet::storage] // --- ITEM( tota_number_of_existing_networks )
	pub type N<T:Config> = StorageValue<_, u16, ValueQuery>;
	#[pallet::storage] // --- MAP () --> subnetwork_n (Number of UIDs in the network).
	pub type SubnetworkN<T:Config> = StorageMap< _, Identity, u16, u16, ValueQuery, DefaultN<T> >;
	#[pallet::storage] // --- MAP () --> network_is_added
	pub type NetworksAdded<T:Config> = StorageMap<_, Identity, u16, bool, ValueQuery, DefaultNeworksAdded<T>>;	
	#[pallet::storage] // --- DMAP () -> registration_requirement
	pub type NetworkConnect<T:Config> = StorageDoubleMap<_, Identity, u16, Identity, u16, u16, OptionQuery>;

	// ==============================
	// ==== Subnetwork Features =====
	// ==============================
	#[pallet::type_value]
	pub fn DefaultEmissionValues<T: Config>() ->  u64 { 0 }
	#[pallet::type_value]
	pub fn DefaultPendingEmission<T: Config>() ->  u64 { 0 }
	#[pallet::type_value] 
	pub fn DefaultBlocksSinceLastStep<T: Config>() -> u64 { 0 }
	#[pallet::type_value] 
	pub fn DefaultLastMechansimStepBlock<T: Config>() -> u64 { 0 }
	#[pallet::type_value]
	pub fn DefaultTempo<T: Config>() -> u16 { T::InitialTempo::get() }

	#[pallet::storage] // --- MAP () --> tempo
	pub type Tempo<T> = StorageMap<_, Identity, u16, u16, ValueQuery, DefaultTempo<T> >;
	#[pallet::storage] // --- MAP () --> emission_values
	pub type EmissionValues<T> = StorageMap<_, Identity, u16, u64, ValueQuery, DefaultEmissionValues<T>>;
	#[pallet::storage] // --- MAP () --> pending_emission
	pub type PendingEmission<T> = StorageMap<_, Identity, u16, u64, ValueQuery, DefaultPendingEmission<T>>;
	#[pallet::storage] // --- MAP () --> blocks_since_last_step.
	pub type BlocksSinceLastStep<T> = StorageMap<_, Identity, u16, u64, ValueQuery, DefaultBlocksSinceLastStep<T>>;
	#[pallet::storage] // --- MAP () --> last_mechanism_step_block
	pub type LastMechansimStepBlock<T> = StorageMap<_, Identity, u16, u64, ValueQuery, DefaultLastMechansimStepBlock<T> >;

	// =================================
	// ==== Neuron Endpoints =====
	// =================================
	
	// --- Struct for Neuron.
	
	#[derive(Encode, Decode, Default, TypeInfo, Clone, PartialEq, Eq, Debug)]
    pub struct NeuronInfo {
		pub block: u64, // --- Neuron serving block.
        pub ip: u128, // --- Neuron u128 encoded ip address of type v6 or v4.
        pub port: u16, // --- Neuron u16 encoded port.
        pub name: Vec<u8>, // --- Neuron ip type, 4 for ipv4 and 6 for ipv6.
		pub context: Vec<u8>, // --- Neuron context.
	}

	#[derive(Encode, Decode, Default, TypeInfo, Clone, PartialEq, Eq, Debug)]
	pub struct SubnetInfo<T: Config> {
		pub port: u16, // --- Neuron u16 encoded port.
		pub name: Vec<u8>, // --- Neuron ip type, 4 for ipv4 and 6 for ipv6.
		pub context: Vec<u8>, // --- Neuron context.
		pub keys: Vec<T::AccountId>, // --- Neuron context.
		// pub key: T::AccountId, // --- Neuron context.
	}



	// Rate limiting
	#[pallet::type_value]
	pub fn DefaultTxRateLimit<T: Config>() -> u64 { T::InitialTxRateLimit::get() }
	#[pallet::type_value]
	pub fn DefaultLastTxBlock<T: Config>() -> u64 { 0 }

	#[pallet::storage] // --- ITEM ( tx_rate_limit )
	pub(super) type TxRateLimit<T> = StorageValue<_, u64, ValueQuery, DefaultTxRateLimit<T>>;
	#[pallet::storage] // --- MAP ( key ) --> last_block
	pub(super) type LastTxBlock<T:Config> = StorageMap<_, Identity, T::AccountId, u64, ValueQuery, DefaultLastTxBlock<T>>;


	#[pallet::type_value] 
	pub fn DefaultServingRateLimit<T: Config>() -> u64 { T::InitialServingRateLimit::get() }

	#[pallet::storage] // --- MAP () --> serving_rate_limit
	pub type ServingRateLimit<T> = StorageMap<_, Identity, u16, u64, ValueQuery, DefaultServingRateLimit<T>> ;
	#[pallet::storage] // --- MAP ( key ) --> neuron_info
	pub(super) type Neurons<T:Config> = StorageDoubleMap<_, Identity, u16, Blake2_128Concat, T::AccountId, NeuronInfo, OptionQuery>;
	
	// =======================================
	// ==== Subnetwork Hyperparam storage ====
	// =======================================	
	#[pallet::type_value] 
	pub fn DefaultWeightsSetRateLimit<T: Config>() -> u64 { 0 }
	#[pallet::type_value] 
	pub fn DefaultBlockAtRegistration<T: Config>() -> u64 { 0 }
	#[pallet::type_value]
	pub fn DefaultMaxAllowedUids<T: Config>() -> u16 { T::InitialMaxAllowedUids::get() }
	#[pallet::type_value] 
	pub fn DefaultImmunityPeriod<T: Config>() -> u16 { T::InitialImmunityPeriod::get() }
	#[pallet::type_value] 
	pub fn DefaultActivityCutoff<T: Config>() -> u16 { T::InitialActivityCutoff::get() }
	#[pallet::type_value] 
	pub fn DefaultMaxWeightsLimit<T: Config>() -> u16 { T::InitialMaxWeightsLimit::get() }
	#[pallet::type_value] 
	pub fn DefaultMinAllowedWeights<T: Config>() -> u16 { T::InitialMinAllowedWeights::get() }
	#[pallet::type_value]
	pub fn DefaultAdjustmentInterval<T: Config>() -> u16 { T::InitialAdjustmentInterval::get() }
	#[pallet::type_value] 
	pub fn DefaultTargetRegistrationsPerInterval<T: Config>() -> u16 { T::InitialTargetRegistrationsPerInterval::get() }

	#[pallet::storage]
	pub type NeuronNamespace<T: Config> = StorageMap<_,  Twox64Concat, Vec<u8>, T::AccountId, ValueQuery, DefaultKey<T>>;
		
	#[pallet::storage] // --- MAP () --> uid, we use to record uids to prune at next epoch.
    pub type NeuronsToPruneAtNextEpoch<T:Config> = StorageValue<_, u16, ValueQuery>;
	#[pallet::storage] // --- MAP () --> registrations_this_interval
	pub type RegistrationsThisInterval<T:Config> =   StorageValue<_, u16, ValueQuery>;
	#[pallet::storage] // --- MAP () --> max_allowed_uids
	pub type MaxAllowedUids<T> =  StorageValue<_, u16, ValueQuery, DefaultMaxAllowedUids<T> >;
	#[pallet::storage] // --- MAP () --> immunity_period
	pub type ImmunityPeriod<T> =  StorageValue<_, u16, ValueQuery, DefaultImmunityPeriod<T> >;
	#[pallet::storage] // --- MAP () --> activity_cutoff
	pub type ActivityCutoff<T> =  StorageValue<_, u16, ValueQuery, DefaultActivityCutoff<T> >;
	#[pallet::storage] // --- MAP () --> max_weight_limit
	pub type MaxWeightsLimit<T> = StorageValue<_, u16, ValueQuery,DefaultMaxWeightsLimit<T> >;
	#[pallet::storage] // --- MAP () --> min_allowed_weights
	pub type MinAllowedWeights<T> = StorageValue<_, u16, ValueQuery, DefaultMinAllowedWeights<T> >;
	#[pallet::storage] // --- MAP () --> adjustment_interval
	pub type AdjustmentInterval<T> =  StorageValue<_, u16, ValueQuery, DefaultAdjustmentInterval<T> >;
	#[pallet::storage] // --- MAP () --> weights_set_rate_limit
	pub type WeightsSetRateLimit<T> = StorageValue<_, u16, ValueQuery, DefaultWeightsSetRateLimit<T> >;
	#[pallet::storage] // --- MAP () --> target_registrations_this_interval
	pub type TargetRegistrationsPerInterval<T> = StorageValue<_, u16 , ValueQuery, DefaultTargetRegistrationsPerInterval<T> >;
	#[pallet::storage] // --- DMAP ( uid ) --> block_at_registration
	pub type BlockAtRegistration<T:Config> = StorageDoubleMap<_, Identity, u16, Identity, u16, u64, ValueQuery, DefaultBlockAtRegistration<T> >;

	// =======================================
	// ==== Subnetwork Storage  ====
	// =======================================
	#[pallet::type_value] 
	pub fn EmptyU16Vec<T:Config>() -> Vec<u16> { vec![] }
	#[pallet::type_value] 
	pub fn EmptyU64Vec<T:Config>() -> Vec<u64> { vec![] }
	#[pallet::type_value] 
	pub fn EmptyBoolVec<T:Config>() -> Vec<bool> { vec![] }
	#[pallet::type_value] 
	pub fn DefaultBonds<T:Config>() -> Vec<(u16, u16)> { vec![] }
	#[pallet::type_value] 
	pub fn DefaultWeights<T:Config>() -> Vec<(u16, u16)> { vec![] }
	#[pallet::type_value] 
	pub fn DefaultKey<T:Config>() -> T::AccountId { T::AccountId::decode(&mut sp_runtime::traits::TrailingZeroInput::zeroes()).unwrap() }

	#[pallet::storage] // --- DMAP ( key ) --> uid
	pub(super) type Uids<T:Config> = StorageDoubleMap<_, Identity, u16, Blake2_128Concat, T::AccountId, u16, OptionQuery>;
	#[pallet::storage] // --- DMAP ( uid ) --> key
	pub(super) type Keys<T:Config> = StorageDoubleMap<_, Identity, u16, Identity, u16, T::AccountId, ValueQuery, DefaultKey<T> >;

	#[pallet::storage] // --- DMAP () --> emission
	pub(super) type LoadedEmission<T:Config> = StorageMap< _, Identity, u16, Vec<(T::AccountId, u64)>, OptionQuery >;

	#[pallet::storage] // --- DMAP () --> active
	pub(super) type Active<T:Config> = StorageMap< _, Identity, u16, Vec<bool>, ValueQuery, EmptyBoolVec<T> >;
	#[pallet::storage] // --- DMAP () --> incentive
	pub(super) type Incentive<T:Config> = StorageMap< _, Identity, u16, Vec<u16>, ValueQuery, EmptyU16Vec<T>>;
	#[pallet::storage] // --- DMAP () --> dividends
	pub(super) type Dividends<T:Config> = StorageMap< _, Identity, u16, Vec<u16>, ValueQuery, EmptyU16Vec<T>>;
	#[pallet::storage] // --- DMAP () --> dividends
	pub(super) type Emission<T:Config> = StorageMap< _, Identity, u16, Vec<u64>, ValueQuery, EmptyU64Vec<T>>;
	#[pallet::storage] // --- DMAP () --> last_update
	pub(super) type LastUpdate<T:Config> = StorageMap< _, Identity, u16, Vec<u64>, ValueQuery, EmptyU64Vec<T>>;
	#[pallet::storage] // --- DMAP () --> pruning_scores
	

	#[pallet::storage] // --- DMAP ( uid ) --> weights
    pub(super) type Weights<T:Config> = StorageDoubleMap<_, Identity, u16, Identity, u16, Vec<(u16, u16)>, ValueQuery, DefaultWeights<T> >;
	#[pallet::storage] // --- DMAP ( uid ) --> bonds
    pub(super) type Bonds<T:Config> = StorageDoubleMap<_, Identity, u16, Identity, u16, Vec<(u16, u16)>, ValueQuery, DefaultBonds<T> >;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		// Event documentation should end with an array that provides descriptive names for event
		// parameters. [something, who]
		NetworkAdded( u16, Vec<u8> ),	// --- Event created when a new network is added.
		NetworkRemoved( u16 ), // --- Event created when a network is removed.
		StakeAdded( T::AccountId, u64 ), // --- Event created when stake has been transfered from the a coldkey account onto the key staking account.
		StakeRemoved( T::AccountId, u64 ), // --- Event created when stake has been removed from the key staking account onto the coldkey account.
		WeightsSet( u16, u16 ), // ---- Event created when a caller successfully set's their weights on a subnetwork.
		NeuronRegistered( u16, u16, T::AccountId ), // --- Event created when a new neuron account has been registered to the chain.
		BulkNeuronsRegistered( u16, u16 ), // --- Event created when multiple uids have been concurrently registered.
		BulkBalancesSet(u16, u16),
		MaxAllowedUidsSet( u16, u16 ), // --- Event created when max allowed uids has been set for a subnetwor.
		MaxWeightLimitSet( u16, u16 ), // --- Event created when the max weight limit has been set.
		AdjustmentIntervalSet( u16, u16 ), // --- Event created when the adjustment interval is set for a subnet.
		RegistrationPerIntervalSet( u16, u16 ), // --- Event created when registeration per interval is set for a subnet.
		MaxRegistrationsPerBlockSet( u16, u16), // --- Event created when we set max registrations per block
		ActivityCutoffSet( u16, u16 ), // --- Event created when an activity cutoff is set for a subnet.
		MinAllowedWeightSet( u16, u16 ), // --- Event created when minimun allowed weight is set for a subnet.
		WeightsSetRateLimitSet( u16, u64 ), // --- Event create when weights set rate limit has been set for a subnet.
		ImmunityPeriodSet( u16, u16), // --- Event created when immunity period is set for a subnet.
		NeuronServed( u16, T::AccountId ), // --- Event created when the neuron server information is added to the network.
		EmissionValuesSet(), // --- Event created when emission ratios fr all networks is set.
		DelegateAdded( T::AccountId, T::AccountId, u16 ), // --- Event created to signal a key has become a delegate.
		ServingRateLimitSet( u16, u64 ), // --- Event created when setting the prometheus serving rate limit.
		TxRateLimitSet( u64 ), // --- Event created when setting the transaction rate limit.
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		NeuronNameAlreadyExists, // --- Thrown when a neuron name already exists.
		NetworkDoesNotExist, // --- Thrown when the network does not exist.
		NetworkExist, // --- Thrown when the network already exist.
		InvalidIpType, // ---- Thrown when the user tries to serve an neuron which is not of type	4 (IPv4) or 6 (IPv6).
		InvalidIpAddress, // --- Thrown when an invalid IP address is passed to the serve function.
		NotRegistered, // ---- Thrown when the caller requests setting or removing data from a neuron which does not exist in the active set.
		NonAssociatedColdKey, // ---- Thrown when a stake, unstake or subscribe request is made by a coldkey which is not associated with the key account. 
		NotEnoughStaketoWithdraw, // ---- Thrown when the caller requests removing more stake then there exists in the staking account. See: fn remove_stake.
		NotEnoughBalanceToStake, //  ---- Thrown when the caller requests adding more stake than there exists in the cold key account. See: fn add_stake
		BalanceWithdrawalError, // ---- Thrown when the caller tries to add stake, but for some reason the requested amount could not be withdrawn from the coldkey account
		WeightVecNotEqualSize, // ---- Thrown when the caller attempts to set the weight keys and values but these vectors have different size.
		DuplicateUids, // ---- Thrown when the caller attempts to set weights with duplicate uids in the weight matrix.
		InvalidUid, // ---- Thrown when a caller attempts to set weight to at least one uid that does not exist in the metagraph.
		NotSettingEnoughWeights, // ---- Thrown when the dispatch attempts to set weights on chain with fewer elements than are allowed.
		TooManyRegistrationsThisBlock, // ---- Thrown when registrations this block exceeds allowed number.
		AlreadyRegistered, // ---- Thrown when the caller requests registering a neuron which already exists in the active set.
		MaxAllowedUIdsNotAllowed, // ---  Thrown if the vaule is invalid for MaxAllowedUids
		CouldNotConvertToBalance, // ---- Thrown when the dispatch attempts to convert between a u64 and T::balance but the call fails.
		StakeAlreadyAdded, // --- Thrown when the caller requests adding stake for a key to the total stake which already added
		MaxWeightExceeded, // --- Thrown when the dispatch attempts to set weights on chain with where any normalized weight is more than MaxWeightLimit.
		StorageValueOutOfRange, // --- Thrown when the caller attempts to set a storage value outside of its allowed range.
		TempoHasNotSet, // --- Thrown when tempo has not set
		InvalidTempo, // --- Thrown when tempo is not valid
		EmissionValuesDoesNotMatchNetworks, // --- Thrown when number or recieved emission rates does not match number of networks
		InvalidEmissionValues, // --- Thrown when emission ratios are not valid (did not sum up to 10^9)
		SettingWeightsTooFast, // --- Thrown if the key attempts to set weights twice withing net_tempo/2 blocks.
		ServingRateLimitExceeded, // --- Thrown when an neuron or prometheus serving exceeds the rate limit for a registered neuron.
		BalanceSetError, // --- Thrown when an error occurs setting a balance
		MaxAllowedUidsExceeded, // --- Thrown when number of accounts going to be registered exceed MaxAllowedUids for the network.
		TooManyUids, // ---- Thrown when the caller attempts to set weights with more uids than allowed.
		TxRateLimitExceeded, // --- Thrown when a transactor exceeds the rate limit for transactions.
		InvalidMaxAllowedUids, // --- Thrown when the user tries to set max allowed uids to a value less than the current number of registered uids.
	}

	// ==================
	// ==== Genesis =====
	// ==================

	#[pallet::genesis_config]
	#[cfg(feature = "std")]
	pub struct GenesisConfig<T: Config> {
		pub stakes: Vec<(T::AccountId, Vec<(T::AccountId, (u64, u16))>)>,
		pub balances_issuance: u64
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { 
				stakes: Default::default(),
				balances_issuance: 0
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			// Set initial total issuance from balances
			TotalIssuance::<T>::put(self.balances_issuance);

			// Subnet config values
			let name = "commune".as_bytes().to_vec();
			let tempo = 99;
			let n = 4096;

			let netuid = TotalNetworks::<T>::get();
			TotalNetworks::<T>::mutate( |n| *n += 1 );
	

			// --- 4. Fill tempo memory item.
			Tempo::<T>::put( tempo );
	
		
			MaxAllowedUids::<T>::put( n );
	

			
			// --- Fill tempo memory item.
			Tempo::<T>::put( tempo);

			// Set max allowed uids
			MaxAllowedUids::<T>::put( n);

			let mut next_uid = 0;

			for (coldkey, keys) in self.stakes.iter() {
				for (key, stake_uid) in keys.iter() {
					let (stake, uid) = stake_uid;

					// Expand Yuma with new position.
					Rank::<T>::mutate(|v| v.push(0));
					Active::<T>::mutate(|v| v.push(true));
					Emission::<T>::mutate(|v| v.push(0));
					Incentive::<T>::mutate(|v| v.push(0));
					Dividends::<T>::mutate(|v| v.push(0));
					LastUpdate::<T>::mutate(|v| v.push(0));
			
					// Insert account information.
					Keys::<T>::insert(uid, key.clone()); // Make key - uid association.
					Uids::<T>::insert(key.clone(), uid); // Make uid - key association.
					BlockAtRegistration::<T>::insert(uid, 0); // Fill block at registration.
	
					// Fill stake information.
	
					TotalKeyStake::<T>::insert(key.clone(), stake);

					// Update total issuance value
					TotalIssuance::<T>::put(TotalIssuance::<T>::get().saturating_add(*stake));
	
					Stake::<T>::insert(key.clone(),  stake);
	
					next_uid += 1;
				}
			}

		}
	}

	// ================
	// ==== Hooks =====
	// ================
  
	#[pallet::hooks] 
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> { 
		// ---- Called on the initialization of this pallet. (the order of on_finalize calls is determined in the runtime)
		//
		// # Args:
		// 	* 'n': (T::BlockNumber):
		// 		- The number of the block we are initializing.
		fn on_initialize( _block_number: BlockNumberFor<T> ) -> Weight {
			Self::block_step();
			
			return Weight::from_ref_time(110_634_229_000 as u64)
						.saturating_add(T::DbWeight::get().reads(8304 as u64))
						.saturating_add(T::DbWeight::get().writes(110 as u64));
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {

		// --- Sets the caller weights for the incentive mechanism. The call can be
		// made from the key account so is potentially insecure, however, the damage
		// of changing weights is minimal if caught early. This function includes all the
		// checks that the passed weights meet the requirements. Stored as u16s they represent
		// rational values in the range [0,1] which sum to 1 and can be interpreted as
		// probabilities. The specific weights determine how inflation propagates outward
		// from this peer. 
		// 
		// Note: The 16 bit integers weights should represent 1.0 as the max u16.
		// However, the function normalizes all integers to u16_max anyway. This means that if the sum of all
		// elements is larger or smaller than the amount of elements * u16_max, all elements
		// will be corrected for this deviation. 
		// 
		// # Args:
		// 	* `origin`: (<T as frame_system::Config>Origin):
		// 		- The caller, a key who wishes to set their weights.
		//

		// 	* `dests` (Vec<u16>):
		// 		- The edge endpoint for the weight, i.e. j for w_ij.
		//
		// 	* 'weights' (Vec<u16>):
		// 		- The u16 integer encoded weights. Interpreted as rational
		// 		values in the range [0,1]. They must sum to in32::MAX.
		//
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
        #[pallet::weight((Weight::from_ref_time(10_151_000_000)
		.saturating_add(T::DbWeight::get().reads(4104))
		.saturating_add(T::DbWeight::get().writes(2)), DispatchClass::Normal, Pays::No))]
		pub fn set_weights(
			origin:OriginFor<T>, 
			dests: Vec<u16>, 
			weights: Vec<u16>,
		) -> DispatchResult {
			Self::do_set_weights( origin, dests, weights )
		}



		// --- Adds stake to a key. The call is made from the
		// coldkey account linked in the key.
		// Only the associated coldkey is allowed to make staking and
		// unstaking requests. This protects the neuron against
		// attacks on its key running in production code.
		//
		// # Args:
		// 	* 'origin': (<T as frame_system::Config>Origin):
		// 		- The signature of the caller's coldkey.
		//
		// 	* 'key' (T::AccountId):
		// 		- The associated key account.
		//
		// 	* 'amount_staked' (u64):
		// 		- The amount of stake to be added to the key staking account.
		//
		// # Event:
		// 	* StakeAdded;
		// 		- On the successfully adding stake to a global account.
		//
		// # Raises:
		// 	* 'CouldNotConvertToBalance':
		// 		- Unable to convert the passed stake value to a balance.
		//
		// 	* 'NotEnoughBalanceToStake':
		// 		- Not enough balance on the coldkey to add onto the global account.
		//
		// 	* 'NonAssociatedColdKey':
		// 		- The calling coldkey is not associated with this key.
		//
		// 	* 'BalanceWithdrawalError':
		// 		- Errors stemming from transaction pallet.
		//
		//
		#[pallet::weight((Weight::from_ref_time(65_000_000)
		.saturating_add(T::DbWeight::get().reads(8))
		.saturating_add(T::DbWeight::get().writes(6)), DispatchClass::Normal, Pays::No))]
		pub fn add_stake(
			origin: OriginFor<T>, 
			amount_staked: u64
		) -> DispatchResult {
			Self::do_add_stake(origin, amount_staked)
		}

		// ---- Remove stake from the staking account. The call must be made
		// from the coldkey account attached to the neuron metadata. Only this key
		// has permission to make staking and unstaking requests.
		//
		// # Args:
		// 	* 'origin': (<T as frame_system::Config>Origin):
		// 		- The signature of the caller's coldkey.
		//
		// 	* 'key' (T::AccountId):
		// 		- The associated key account.
		//
		// 	* 'amount_unstaked' (u64):
		// 		- The amount of stake to be added to the key staking account.
		//
		// # Event:
		// 	* StakeRemoved;
		// 		- On the successfully removing stake from the key account.
		//
		// # Raises:
		// 	* 'NotRegistered':
		// 		- Thrown if the account we are attempting to unstake from is non existent.
		//
		// 	* 'NonAssociatedColdKey':
		// 		- Thrown if the coldkey does not own the key we are unstaking from.
		//
		// 	* 'NotEnoughStaketoWithdraw':
		// 		- Thrown if there is not enough stake on the key to withdwraw this amount. 
		//
		// 	* 'CouldNotConvertToBalance':
		// 		- Thrown if we could not convert this amount to a balance.
		//
		//
		#[pallet::weight((Weight::from_ref_time(66_000_000)
		.saturating_add(T::DbWeight::get().reads(8))
		.saturating_add(T::DbWeight::get().writes(6)), DispatchClass::Normal, Pays::No))]
		pub fn remove_stake(
			origin: OriginFor<T>, 
			amount_unstaked: u64
		) -> DispatchResult {
			Self::do_remove_stake(origin, amount_unstaked)
		}

		// ---- Serves or updates neuron /promethteus information for the neuron associated with the caller. If the caller is
		// already registered the metadata is updated. If the caller is not registered this call throws NotRegistered.
		//
		// # Args:
		// 	* 'origin': (<T as frame_system::Config>Origin):
		// 		- The signature of the caller.
		//

		// 	* 'ip' (u64):
		// 		- The endpoint ip information as a u128 encoded integer.
		//
		// 	* 'port' (u16):
		// 		- The endpoint port information as a u16 encoded integer.

		// 	* 'protocol' (u8):
		// 		- UDP:1 or TCP:0 
		//
		// # Event:
		// 	* NeuronServed;
		// 		- On successfully serving the neuron info.
		//
		// # Raises:
		// 	* 'NetworkDoesNotExist':
		// 		- Attempting to set weights on a non-existent network.
		//
		// 	* 'NotRegistered':
		// 		- Attempting to set weights from a non registered account.
		//
		// 	* 'InvalidIpType':
		// 		- The ip type is not 4 or 6.
		//
		// 	* 'InvalidIpAddress':
		// 		- The numerically encoded ip address does not resolve to a proper ip.
		//
		// 	* 'ServingRateLimitExceeded':
		// 		- Attempting to set prometheus information withing the rate limit min.
		//
		#[pallet::weight((Weight::from_ref_time(19_000_000)
		.saturating_add(T::DbWeight::get().reads(2))
		.saturating_add(T::DbWeight::get().writes(1)), DispatchClass::Normal, Pays::No))]
		pub fn serve_neuron(
			origin:OriginFor<T>, 
			ip: u128, 
			port: u16,
			name : Vec<u8>,
			context: Vec<u8>
		) -> DispatchResult {
			Self::do_serve_neuron( origin, ip, port, name, context ) 
		}


		#[pallet::weight((Weight::from_ref_time(19_000_000)
		.saturating_add(T::DbWeight::get().reads(2))
		.saturating_add(T::DbWeight::get().writes(1)), DispatchClass::Normal, Pays::No))]
		pub fn update_neuron(
			origin:OriginFor<T>, 
			ip: u128, 
			port: u16,
			name : Vec<u8>,
			context: Vec<u8>
		) -> DispatchResult {
			Self::do_update_neuron( origin, ip, port, name, context ) 
		}



		// ---- Registers a new neuron to the subnetwork. 
		//
		// # Args:
		// 	* 'origin': (<T as frame_system::Config>Origin):
		// 		- The signature of the calling key.

		// 	* 'block_number' ( u64 ):
		// 		- Block hash used to prove work done.
		//
		// 	* 'nonce' ( u64 ):
		// 		- Positive integer nonce used in POW.
		//

		// 	* 'key' ( T::AccountId ):
		// 		- Key to be registered to the network.
		//
		// 	* 'coldkey' ( T::AccountId ):
		// 		- Associated coldkey account.
		//
		// # Event:
		// 	* NeuronRegistered;
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

		#[pallet::weight((Weight::from_ref_time(91_000_000)
		.saturating_add(T::DbWeight::get().reads(27))
		.saturating_add(T::DbWeight::get().writes(22)), DispatchClass::Normal, Pays::No))]
		pub fn register( 
				origin:OriginFor<T>, 
				ip: u128, 
				port: u16, 
				name: Vec<u8>,
				context: Vec<u8>
		) -> DispatchResult { 
			Self::do_registration(origin, ip, port, name, context)
		}



		// ---- Sudo set emission values for all networks.
		// Args:
		// 	* 'origin': (<T as frame_system::Config>Origin):
		// 		- The caller, must be sudo.
		//
		// 	* `netuids` (Vec<u16>):
		// 		- A vector of network uids values. This must include all netuids.
		//
		// 	* `emission` (Vec<u64>):
		// 		- The emission values associated with passed netuids in order.
		// 
		#[pallet::weight((Weight::from_ref_time(28_000_000)
		.saturating_add(T::DbWeight::get().reads(12))
		.saturating_add(T::DbWeight::get().writes(10)), DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_emission_values(
			origin: OriginFor<T>,
			emission: Vec<u64>,
		) -> DispatchResult {
			Self::do_set_emission_values( 
				origin,
				emission
			)
		}



		// ==================================
		// ==== Parameter Sudo calls ========
		// ==================================


		#[pallet::weight((Weight::from_ref_time(10_000_000)
		.saturating_add(T::DbWeight::get().writes(1)), DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_serving_rate_limit( origin:OriginFor<T>: u16, serving_rate_limit: u64 ) -> DispatchResult {  
			Self::do_sudo_set_serving_rate_limit( origin, serving_rate_limit )
		}

		// Sudo call for setting tx rate limit
		#[pallet::weight((0, DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_tx_rate_limit( origin:OriginFor<T>, tx_rate_limit: u64 ) -> DispatchResult {  
			Self::do_sudo_set_tx_rate_limit( origin, tx_rate_limit )
		}

		#[pallet::weight((Weight::from_ref_time(15_000_000)
		.saturating_add(T::DbWeight::get().reads(1))
		.saturating_add(T::DbWeight::get().writes(1)), DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_weights_set_rate_limit( origin:OriginFor<T>: u16, weights_set_rate_limit: u64 ) -> DispatchResult {  
			Self::do_sudo_set_weights_set_rate_limit( origin, weights_set_rate_limit )
		}


		#[pallet::weight((Weight::from_ref_time(14_000_000)
		.saturating_add(T::DbWeight::get().reads(1))
		.saturating_add(T::DbWeight::get().writes(1)), DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_adjustment_interval( origin:OriginFor<T>: u16, adjustment_interval: u16 ) -> DispatchResult { 
			Self::do_set_adjustment_interval( origin, adjustment_interval )
		}
		#[pallet::weight((Weight::from_ref_time(14_000_000)
		.saturating_add(T::DbWeight::get().reads(1))
		.saturating_add(T::DbWeight::get().writes(1)), DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_target_registrations_per_interval( origin:OriginFor<T>: u16, target_registrations_per_interval: u16 ) -> DispatchResult {
			Self::do_sudo_set_target_registrations_per_interval( origin, target_registrations_per_interval )
		}
		#[pallet::weight((Weight::from_ref_time(13_000_000)
		.saturating_add(T::DbWeight::get().reads(1))
		.saturating_add(T::DbWeight::get().writes(1)), DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_activity_cutoff( origin:OriginFor<T>: u16, activity_cutoff: u16 ) -> DispatchResult {
			Self::do_sudo_set_activity_cutoff( origin, activity_cutoff )
		}

		#[pallet::weight((Weight::from_ref_time(18_000_000)
		.saturating_add(T::DbWeight::get().reads(2))
		.saturating_add(T::DbWeight::get().writes(1)), DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_max_allowed_uids( origin:OriginFor<T>: u16, max_allowed_uids: u16 ) -> DispatchResult {
			Self::do_sudo_set_max_allowed_uids(origin, max_allowed_uids )
		}
		#[pallet::weight((Weight::from_ref_time(13_000_000)
		.saturating_add(T::DbWeight::get().reads(1))
		.saturating_add(T::DbWeight::get().writes(1)), DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_min_allowed_weights( origin:OriginFor<T>: u16, min_allowed_weights: u16 ) -> DispatchResult {
			Self::do_sudo_set_min_allowed_weights( origin, min_allowed_weights )
		}


		#[pallet::weight((Weight::from_ref_time(13_000_000)
		.saturating_add(T::DbWeight::get().reads(1))
		.saturating_add(T::DbWeight::get().writes(1)), DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_immunity_period( origin:OriginFor<T>: u16, immunity_period: u16 ) -> DispatchResult {
			Self::do_sudo_set_immunity_period( origin, immunity_period )
		}
		#[pallet::weight((Weight::from_ref_time(13_000_000)
		.saturating_add(T::DbWeight::get().reads(1))
		.saturating_add(T::DbWeight::get().writes(1)), DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_max_weight_limit( origin:OriginFor<T>: u16, max_weight_limit: u16 ) -> DispatchResult {
			Self::do_sudo_set_max_weight_limit( origin, max_weight_limit )
		}
		#[pallet::weight((Weight::from_ref_time(15_000_000)
		.saturating_add(T::DbWeight::get().reads(1))
		.saturating_add(T::DbWeight::get().writes(1)), DispatchClass::Operational, Pays::No))]
		pub fn sudo_set_max_registrations_per_block(origin: OriginFor<T>: u16, max_registrations_per_block: u16 ) -> DispatchResult {
			Self::do_sudo_set_max_registrations_per_block(origin, max_registrations_per_block )
		}



		#[pallet::weight((Weight::from_ref_time(49_882_000_000)
		.saturating_add(T::DbWeight::get().reads(8303))
		.saturating_add(T::DbWeight::get().writes(110)), DispatchClass::Normal, Pays::No))]
		pub fn benchmark_epoch_with_weights( _:OriginFor<T> ) -> DispatchResult {
			Self::epoch( 11, 1_000_000_000 );
			Ok(())
		} 
		#[pallet::weight((Weight::from_ref_time(117_586_465_000 as u64)
		.saturating_add(T::DbWeight::get().reads(12299 as u64))
		.saturating_add(T::DbWeight::get().writes(110 as u64)), DispatchClass::Normal, Pays::No))]
		pub fn benchmark_epoch_without_weights( _:OriginFor<T> ) -> DispatchResult {
			let _: Vec<(T::AccountId, u64)> = Self::epoch( 11, 1_000_000_000 );
			Ok(())
		} 
		#[pallet::weight((0, DispatchClass::Normal, Pays::No))]
		pub fn benchmark_drain_emission( _:OriginFor<T> ) -> DispatchResult {
			Self::drain_emission( 11 );
			Ok(())
		} 
	}	

	// ---- Subspace helper functions.
	impl<T: Config> Pallet<T> {
		// --- Returns the transaction priority for setting weights.
		pub fn get_priority_set_weights( key: &T::AccountId: u16 ) -> u64 {
			if Uids::<T>::contains_key( &key ) {
				let uid = Self::get_uid_for_key( &key.clone()).unwrap();
				let current_block_number: u64 = Self::get_current_block_as_u64();
				return current_block_number - Self::get_last_update_for_uid( uid as u16);
			}
			return 0;
		}
	}
}


/************************************************************
	CallType definition
************************************************************/
#[derive(Debug, PartialEq)]
pub enum CallType {
    SetWeights,
    AddStake,
    RemoveStake,
	AddDelegate,
    Register,
	AddNetwork,
    Serve,
	Other,
}
impl Default for CallType {
    fn default() -> Self {
        CallType::Other
    }
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, TypeInfo)]
pub struct SubspaceSignedExtension<T: Config + Send + Sync + TypeInfo>(pub PhantomData<T>);

impl<T: Config + Send + Sync + TypeInfo> SubspaceSignedExtension<T> where
	T::RuntimeCall: Dispatchable<Info=DispatchInfo, PostInfo=PostDispatchInfo>,
	<T as frame_system::Config>::RuntimeCall: IsSubType<Call<T>>,
{
	pub fn new() -> Self {
		Self(Default::default())
	}

	pub fn get_priority_vanilla() -> u64 {
		// Return high priority so that every extrinsic except set_weights function will 
		// have a higher priority than the set_weights call
		return u64::max_value();
	}

	pub fn get_priority_set_weights( who: &T::AccountId: u16 ) -> u64 {
		// Return the non vanilla priority for a set weights call.

		return Pallet::<T>::get_priority_set_weights( who );
	}

	pub fn u64_to_balance( input: u64 ) -> Option<<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance> { input.try_into().ok() }

}

impl <T:Config + Send + Sync + TypeInfo> sp_std::fmt::Debug for SubspaceSignedExtension<T> {
	fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		write!(f, "SubspaceSignedExtension")
	}
}

impl<T: Config + Send + Sync + TypeInfo> SignedExtension for SubspaceSignedExtension<T>
    where
        T::RuntimeCall: Dispatchable<Info=DispatchInfo, PostInfo=PostDispatchInfo>,
        <T as frame_system::Config>::RuntimeCall: IsSubType<Call<T>>,
{
	const IDENTIFIER: &'static str = "SubspaceSignedExtension";

	type AccountId = T::AccountId;
	type Call = T::RuntimeCall;
	type AdditionalSigned = ();
	type Pre = (CallType, u64, Self::AccountId);
	
	fn additional_signed( &self ) -> Result<Self::AdditionalSigned, TransactionValidityError> { 
		Ok(())
	}


	fn validate(
		&self,
		who: &Self::AccountId,
		call: &Self::Call,
		_info: &DispatchInfoOf<Self::Call>,
		_len: usize,
	) -> TransactionValidity {
		match call.is_sub_type() {
			Some(Call::set_weights{..}) => {
				let priority: u64 = Self::get_priority_set_weights(who, *netuid);
                Ok(ValidTransaction {
                    priority: priority,
                    longevity: 1,
                    ..Default::default()
                })
            }
			Some(Call::add_stake{..}) => {
                Ok(ValidTransaction {
                    priority: Self::get_priority_vanilla(),
                    ..Default::default()
                })
            }
            Some(Call::remove_stake{..}) => {
                Ok(ValidTransaction {
                    priority: Self::get_priority_vanilla(),
                    ..Default::default()
                })
            }
            Some(Call::add_network{..}) => {
                Ok(ValidTransaction {
                    priority: Self::get_priority_vanilla(),
                    ..Default::default()
                })
            }
	
			Some(Call::register{..}) => {
                Ok(ValidTransaction {
                    priority: Self::get_priority_vanilla(),
                    ..Default::default()
                })
            }
			_ => {
                Ok(ValidTransaction {
                    priority: Self::get_priority_vanilla(),
                    ..Default::default()
                })
            }
		}
	}

	// NOTE: Add later when we put in a pre and post dispatch step.
    fn pre_dispatch(
        self,
        who: &Self::AccountId,
        call: &Self::Call,
        _info: &DispatchInfoOf<Self::Call>,
        _len: usize,
    ) -> Result<Self::Pre, TransactionValidityError> {

        match call.is_sub_type() {
            Some(Call::add_stake{..}) => {
				let transaction_fee = 0;
                Ok((CallType::AddStake, transaction_fee, who.clone()))
            }
            Some(Call::remove_stake{..}) => {
				let transaction_fee = 0;
                Ok((CallType::RemoveStake, transaction_fee, who.clone()))
            }
			Some(Call::set_weights{..}) => {
				let transaction_fee = 0;
                Ok((CallType::SetWeights, transaction_fee, who.clone())) 
            }
			Some(Call::register{..}) => {
                let transaction_fee = 0;
                Ok((CallType::Register, transaction_fee, who.clone()))
            }

            Some(Call::serve_neuron{..}) => {
                let transaction_fee = 0;
                Ok((CallType::Serve, transaction_fee, who.clone()))
            }
            Some(Call::update_neuron{..}) => {
                let transaction_fee = 0;
                Ok((CallType::Serve, transaction_fee, who.clone()))
            }
            _ => {
				let transaction_fee = 0;
                Ok((CallType::Other, transaction_fee, who.clone()))
            }
        }
    }

	fn post_dispatch(
        maybe_pre: Option<Self::Pre>,
        _info: &DispatchInfoOf<Self::Call>,
        _post_info: &PostDispatchInfoOf<Self::Call>,
        _len: usize,
        _result: &dispatch::DispatchResult,
    ) -> Result<(), TransactionValidityError> {

		if let Some((call_type, _transaction_fee, _who)) = maybe_pre {
			match call_type {
				CallType::SetWeights => {
					log::debug!("Not Implemented!");
				}
				CallType::AddStake => {
					log::debug!("Not Implemented! Need to add potential transaction fees here.");
				}
				CallType::RemoveStake => {
					log::debug!("Not Implemented! Need to add potential transaction fees here.");
				}
				CallType::AddNetwork => {
					log::debug!("Not Implemented! Need to add potential transaction fees here.");
				}
				CallType::Register => {
					log::debug!("Not Implemented!");
				}
				_ => {
					log::debug!("Not Implemented!");
				}
			}
		} 
		Ok(())
    }

}
