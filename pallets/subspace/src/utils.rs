use frame_support::inherent::Vec;

pub fn vec_to_hash( vec_hash: Vec<u8> ) -> H256 {
    let de_ref_hash = &vec_hash; // b: &Vec<u8>
    let de_de_ref_hash: &[u8] = &de_ref_hash; // c: &[u8]
    let real_hash: H256 = H256::from_slice( de_de_ref_hash );
    return real_hash
}
