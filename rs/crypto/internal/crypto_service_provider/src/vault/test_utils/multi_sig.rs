use crate::api::CspSigner;
use crate::secret_key_store::test_utils::TempSecretKeyStore;
use crate::types::CspPublicKey;
use crate::vault::api::{CspMultiSignatureError, CspMultiSignatureKeygenError, CspVault};
use crate::Csp;
use crate::KeyId;
use ic_types::crypto::AlgorithmId;
use rand::{thread_rng, Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use std::sync::Arc;
use strum::IntoEnumIterator;

fn multi_sig_verifier() -> impl CspSigner {
    let dummy_key_store = TempSecretKeyStore::new();
    let csprng = ChaChaRng::from_seed(thread_rng().gen::<[u8; 32]>());
    Csp::of(csprng, dummy_key_store)
}

pub fn should_generate_multi_bls12_381_key_pair(csp_vault: Arc<dyn CspVault>) {
    let (pk, _pop) = csp_vault
        .gen_key_pair_with_pop(AlgorithmId::MultiBls12_381)
        .expect("Failure generating key pair with pop");

    assert!(matches!(pk, CspPublicKey::MultiBls12_381(_)));
}

pub fn should_fail_to_generate_multi_sig_key_for_wrong_algorithm_id(csp_vault: Arc<dyn CspVault>) {
    for algorithm_id in AlgorithmId::iter() {
        if algorithm_id != AlgorithmId::MultiBls12_381 {
            assert_eq!(
                csp_vault.gen_key_pair_with_pop(algorithm_id).unwrap_err(),
                CspMultiSignatureKeygenError::UnsupportedAlgorithm {
                    algorithm: algorithm_id,
                }
            );
        }
    }
}

pub fn should_generate_verifiable_pop(csp_vault: Arc<dyn CspVault>) {
    let (public_key, pop) = csp_vault
        .gen_key_pair_with_pop(AlgorithmId::MultiBls12_381)
        .expect("Failed to generate key pair with PoP");

    let verifier = multi_sig_verifier();
    assert!(verifier
        .verify_pop(&pop, AlgorithmId::MultiBls12_381, public_key)
        .is_ok());
}

pub fn should_multi_sign_and_verify_with_generated_key(csp_vault: Arc<dyn CspVault>) {
    let (csp_pub_key, csp_pop) = csp_vault
        .gen_key_pair_with_pop(AlgorithmId::MultiBls12_381)
        .expect("failed to generate keys");
    let key_id = KeyId::from(&csp_pub_key);

    let mut rng = thread_rng();
    let msg_len: usize = rng.gen_range(0..1024);
    let msg: Vec<u8> = (0..msg_len).map(|_| rng.gen::<u8>()).collect();

    let verifier = multi_sig_verifier();
    let sig = csp_vault
        .multi_sign(AlgorithmId::MultiBls12_381, &msg, key_id)
        .expect("failed to generate signature");

    assert!(verifier
        .verify(&sig, &msg, AlgorithmId::MultiBls12_381, csp_pub_key.clone())
        .is_ok());

    assert!(verifier
        .verify_pop(&csp_pop, AlgorithmId::MultiBls12_381, csp_pub_key)
        .is_ok());
}

pub fn should_not_multi_sign_with_unsupported_algorithm_id(csp_vault: Arc<dyn CspVault>) {
    let (csp_pub_key, _csp_pop) = csp_vault
        .gen_key_pair_with_pop(AlgorithmId::MultiBls12_381)
        .expect("failed to generate keys");
    let key_id = KeyId::from(&csp_pub_key);

    let msg = [31; 41];

    for algorithm_id in AlgorithmId::iter() {
        if algorithm_id != AlgorithmId::MultiBls12_381 {
            assert_eq!(
                csp_vault
                    .multi_sign(algorithm_id, &msg, key_id)
                    .unwrap_err(),
                CspMultiSignatureError::UnsupportedAlgorithm {
                    algorithm: algorithm_id,
                }
            );
        }
    }
}

pub fn should_not_multi_sign_if_secret_key_in_store_has_wrong_type(csp_vault: Arc<dyn CspVault>) {
    let wrong_csp_pub_key = csp_vault
        .gen_key_pair(AlgorithmId::Ed25519)
        .expect("failed to generate keys");

    let msg = [31; 41];
    let result = csp_vault.multi_sign(
        AlgorithmId::MultiBls12_381,
        &msg,
        KeyId::from(&wrong_csp_pub_key),
    );

    assert_eq!(
        result.unwrap_err(),
        CspMultiSignatureError::WrongSecretKeyType {
            algorithm: AlgorithmId::MultiBls12_381,
            secret_key_variant: "Ed25519".to_string()
        }
    );
}
