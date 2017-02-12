extern crate untrusted;

extern crate ring;
use self::ring::{digest, rand, signature};

pub fn digest_sha256(buf: &[u8]) -> Vec<u8>
{
    digest::digest(&digest::SHA256, buf).as_ref().to_vec()
}

pub fn gen_ed25519keypair(public_key: &mut [u8; 32], private_key: &mut [u8; 32])
{
    let rng = rand::SystemRandom::new();
    let (_, generated_bytes) = signature::Ed25519KeyPair::generate_serializable(&rng).expect("Failed generating key pair");
    *public_key = generated_bytes.public_key;
    *private_key = generated_bytes.private_key;
}

pub fn sign_ed25519(bytes: &[u8], public_key: &[u8], private_key: &[u8]) -> Vec<u8>
{
    let mut sig = [0; 64];
    let sigbytes = &signature::Ed25519KeyPair::from_bytes(&private_key, &public_key).unwrap().sign(bytes);
    sig.clone_from_slice(sigbytes.as_slice());
    sig.to_vec()
}

pub fn verify_ed25519(bytes: &[u8], sig: &[u8], public_key: &[u8]) -> bool
{
    let key = untrusted::Input::from(&public_key);
    let msg = untrusted::Input::from(bytes);
    let sig = untrusted::Input::from(sig);
    match signature::verify(&signature::ED25519, key, msg, sig)
    {
        Ok(_) => { true }
        Err(_) => { false }
    }
}
