use crypto;
use util;

const INPUT_A: &'static str = "";
const INPUT_B: &'static str = "abc";
const INPUT_C: &'static str = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";

#[test]
fn test_sha256()
{
    assert!(util::to_hex_string(&crypto::digest_sha256(INPUT_A.as_bytes())) == "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    assert!(util::to_hex_string(&crypto::digest_sha256(INPUT_B.as_bytes())) == "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    assert!(util::to_hex_string(&crypto::digest_sha256(INPUT_C.as_bytes())) == "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1");
}

#[test]
fn test_ed25519()
{
    let mut public_key: [u8; 32] = [0; 32];
    let mut private_key: [u8; 32] = [0; 32];
    crypto::gen_ed25519keypair(&mut public_key, &mut private_key);

    let sig_a = crypto::sign_ed25519(&INPUT_A.as_bytes(), &public_key, &private_key);
    assert!(crypto::verify_ed25519(&INPUT_A.as_bytes(), &sig_a, &public_key));

    let sig_b = crypto::sign_ed25519(&INPUT_B.as_bytes(), &public_key, &private_key);
    assert!(crypto::verify_ed25519(&INPUT_B.as_bytes(), &sig_b, &public_key));

    let sig_c = crypto::sign_ed25519(&INPUT_C.as_bytes(), &public_key, &private_key);
    assert!(crypto::verify_ed25519(&INPUT_C.as_bytes(), &sig_c, &public_key));
}
