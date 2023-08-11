use node_subspace_runtime::{
	AccountId, AuraConfig, BalancesConfig, GenesisConfig, GrandpaConfig, Signature, SudoConfig,
	SystemConfig, WASM_BINARY, SubspaceModuleConfig
};
use sc_service::ChainType;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};
use sp_core::crypto::Ss58Codec;

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}



/// Generate an Aura authority key.
pub fn authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
	(get_from_seed::<AuraId>(s), get_from_seed::<GrandpaId>(s))
}

pub fn authority_keys_from_ss58(s_aura :&str, s_grandpa : &str) -> (AuraId, GrandpaId) {
	(
		get_aura_from_ss58_addr(s_aura),
		get_grandpa_from_ss58_addr(s_grandpa),
	)
}

pub fn get_aura_from_ss58_addr(s: &str) -> AuraId {
	Ss58Codec::from_ss58check(s).unwrap()
}

pub fn get_grandpa_from_ss58_addr(s: &str) -> GrandpaId {
	Ss58Codec::from_ss58check(s).unwrap()
}


// Includes for nakamoto genesis
use std::{fs::File, path::PathBuf};
use serde::{Deserialize};
use serde_json as json;






// Configure storage from nakamoto data
#[derive(Deserialize, Debug)]
struct SubspaceJSONState {
	balances: std::collections::HashMap<String, u64>,
	// subnet -> (name, tempo, immunity_period, min_allowed_weights, max_allowed_weights, max_allowed_uids, founder)
	subnets: Vec<(String, u16, u16, u16, u16, u16, String )>,
	// module -> (key, name, address, stake, profit_ratio, weights)
	modules : Vec<Vec<(String, String, String, u64, u16 , Vec<(u16, u16)>)>>,

	block: u32,

	version : u16, //

}


pub fn generate_config(network:String) -> Result<ChainSpec, String> {
	let path: PathBuf = std::path::PathBuf::from(format!("./snapshots/{}.json", network));
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	// We mmap the file into memory first, as this is *a lot* faster than using
	// `serde_json::from_reader`. See https://github.com/serde-rs/json/issues/160
	let file = File::open(&path)
		.map_err(|e| format!("Error opening genesis file `{}`: {}", path.display(), e))?;

	// SAFETY: `mmap` is fundamentally unsafe since technically the file can change
	//         underneath us while it is mapped; in practice it's unlikely to be a problem
	let bytes = unsafe {
		memmap2::Mmap::map(&file)
			.map_err(|e| format!("Error mmaping genesis file `{}`: {}", path.display(), e))?
	};

	let state: SubspaceJSONState =
		json::from_slice(&bytes).map_err(|e| format!("Error parsing genesis file: {}", e))?;

	let block : u32 = state.block;
	// (name, tempo, immunity_period, min_allowed_weights, max_allowed_weights, max_allowed_uids, founder)
	let mut subnets: Vec<( Vec<u8>, u16, u16, u16 , u16, u16, sp_runtime::AccountId32)> = Vec::new();
	let mut modules: Vec<Vec<(sp_runtime::AccountId32, Vec<u8>, Vec<u8>, u64, Vec<(u16,u16)>)>> = Vec::new();

	for (netuid, subnet) in state.subnets.iter().enumerate() {

		subnets.push((subnet.0.as_bytes().to_vec(),  // name
					 subnet.1, // tempo
					 subnet.2, // immunity_period
					 subnet.3, // min_allowed_weights
					 subnet.4, // max_allowed_weights
					 subnet.5, //  max_allowed_uids
					 sp_runtime::AccountId32::from(<sr25519::Public as Ss58Codec>::from_ss58check(&subnet.6).unwrap()),
					));

		// Add  modules
		modules.push(Vec::new());
		for (uid, module) in state.modules[netuid].iter().enumerate() {
			modules[netuid].push((
				sp_runtime::AccountId32::from(<sr25519::Public as Ss58Codec>::from_ss58check(&module.0).unwrap()),
				module.1.as_bytes().to_vec(), // key
				module.2.as_bytes().to_vec(), // name
				module.3, // stake 
				module.4.iter().map(|(a,b)| (*a,*b)).collect(), // Convert to tuples
			));
		}

	}

	let mut balances_issuance: u64 = 0;
	let mut processed_balances: Vec<(sp_runtime::AccountId32, u64)> = Vec::new();
	for (key_str, amount) in state.balances.iter() {
		let key = <sr25519::Public as Ss58Codec>::from_ss58check(&key_str).unwrap();
		let key_account = sp_runtime::AccountId32::from(key);

		processed_balances.push((key_account, *amount));
		balances_issuance += *amount;
	}

	// Give front-ends necessary data to present to users
	let mut properties = sc_service::Properties::new();
	properties.insert("tokenSymbol".into(), "C".into());
	properties.insert("tokenDecimals".into(), 9.into());
	properties.insert("ss58Format".into(), 13116.into());



	Ok(ChainSpec::from_genesis(
		// Name
		"commune",
		// ID
		"commune",
		ChainType::Development,
		move || {
			network_genesis(
				wasm_binary,
				// Initial PoA authorities (Validators)
				// aura | grandpa
				vec![
					// Keys for debug
					authority_keys_from_seed("Alice"), authority_keys_from_seed("Bob"),
					], 
				// Sudo account
				Ss58Codec::from_ss58check("5GYs4kBRGo3VH1wgzYEs8UeP2ABSotNNmvaeXs9vJUiGEThJ").unwrap(), 
				// Pre-funded a
				processed_balances.clone(), // balances
				modules.clone(), // modules,
				subnets.clone(), // subnets,
				block,
				
			)
		},
		// Bootnodes
		vec![
		],
		// Telemetry
		None,
		// Protocol ID
		Some("commune"),
		None,
		// Properties
		Some(properties),
		// Extensions
		None,
	))

}

pub fn mainnet_config() -> Result<ChainSpec, String> {
	return generate_config("main".to_string());
}

pub fn devnet_config() -> Result<ChainSpec, String> {
	return generate_config("dev".to_string());
}
pub fn testnet_config() -> Result<ChainSpec, String> {
	return generate_config("dev".to_string());
}


// Configure initial storage state for FRAME modules.
fn network_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(AuraId, GrandpaId)>,
	root_key: AccountId,
	balances: Vec<(AccountId, u64)>,
	modules: Vec<Vec<(AccountId,Vec<u8>, Vec<u8>, u64, Vec<(u16, u16)>)>>,
	subnets: Vec<(Vec<u8>, u16, u16, u16, u16, u16 ,AccountId)>,
	block: u32,



) -> GenesisConfig {
	GenesisConfig {
		system: SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
		},
		balances: BalancesConfig {
			// Configure endowed accounts with initial balance of 1 << 60.
			//balances: balances.iter().cloned().map(|k| k).collect(),
			balances: balances.iter().cloned().map(|(k, balance)| (k, balance )).collect(),
		},
		aura: AuraConfig {
			authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
		},
		grandpa: GrandpaConfig {
			authorities: initial_authorities.iter().map(|x| (x.1.clone(), 1)).collect(),
		},
		sudo: SudoConfig {
			// Assign network admin rights.
			key: Some(root_key),
		},
		transaction_payment: Default::default(),
		subspace_module: SubspaceModuleConfig {
			// Add names to storage.
			modules: modules,
			subnets: subnets,
			block: block,
		},
	}
}
