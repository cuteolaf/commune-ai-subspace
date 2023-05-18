use super::*;
use frame_support::{sp_std::vec};
use sp_std::vec::Vec;
use frame_system::ensure_root;

impl<T: Config> Pallet<T> { 


    // ---- The implementation for the extrinsic add_network.
    //
    // # Args:
    // 	* 'origin': (<T as frame_system::Config>RuntimeOrigin):
    // 		- Must be sudo.
    //
    // 	* 'netuid' (u16):
    // 		- The u16 network identifier.
    //
    // 	* 'tempo' ( u16 ):
    // 		- Number of blocks between epoch step.
    //
    // 	* 'modality' ( u16 ):
    // 		- Network modality specifier.
    //
    // # Event:
    // 	* NetworkAdded;
    // 		- On successfully creation of a network.
    //
    // # Raises:
    // 	* 'NetworkExist':
    // 		- Attempting to register an already existing.
    //
    // 	* 'InvalidTempo':
    // 		- Attempting to register a network with an invalid tempo.
    //


    pub fn do_set_emission_values( 
        origin: T::RuntimeOrigin, 
        emission: Vec<u64>
    ) -> dispatch::DispatchResult {

        // --- 1. Ensure caller is sudo.
        let key = ensure_signed( origin )?;

        // --- 6. check if sum of emission rates is equal to 1.
        ensure!( emission.iter().sum::<u64>() as u64 == Self::get_block_emission(), Error::<T>::InvalidEmissionValues);

        // --- 7. Add emission values for each network
        Self::set_emission_values( &emission );

        // --- 8. Add emission values for each network
        log::info!("EmissionValuesSet()");
        Self::deposit_event( Event::EmissionValuesSet() );

        // --- 9. Ok and return.
        Ok(())
    }



    // Returns true if the passed tempo is allowed.
    //
    pub fn if_tempo_is_valid(tempo: u16) -> bool {
        tempo < u16::MAX
    }
}