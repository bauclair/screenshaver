use sha2::{
    Digest,
    Sha256,
};


pub fn hash_source(
    source: &[u8],
) -> String {

    let mut hasher =
        Sha256::new();


    hasher.update(
        source
    );


    format!(
        "{:x}",
        hasher.finalize()
    )
}