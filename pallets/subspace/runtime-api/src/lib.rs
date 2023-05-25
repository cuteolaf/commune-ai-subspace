#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
use alloc::vec::Vec;

// Here we declare the runtime API. It is implemented it the `impl` block in
// src/neuron_info.rs, src/subnet_info.rs, and src/delegate_info.rs
sp_api::decl_runtime_apis! {

	pub trait ModuleInfoRuntimeApi {
		fn get_neurons() -> Vec<u8>;
		fn get_neuron(uid: u16) -> Vec<u8>;
	}

}