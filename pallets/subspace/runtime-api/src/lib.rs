#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
use alloc::vec::Vec;

// Here we declare the runtime API. It is implemented it the `impl` block in
// src/neuron_info.rs, src/subnet_info.rs, and src/delegate_info.rs
sp_api::decl_runtime_apis! {

	pub trait NeuronInfoRuntimeApi {
		fn get_neurons(netuid: u16) -> Vec<u8>;
		fn get_neuron(netuid: u16, uid: u16) -> Vec<u8>;
	}

	pub trait NetInfoRuntimeApi {
		fn get_net_info(netuid: u16) -> Vec<u8>;
		fn get_nets_info() -> Vec<u8>;
	}
}