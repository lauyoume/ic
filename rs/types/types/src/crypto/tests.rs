use super::*;
use crate::NodeId;
use strum::IntoEnumIterator;

#[test]
fn should_correctly_convert_i32_to_algorithm_id() {
    ensure_all_algorithm_ids_are_compared(&(0..=16).collect::<Vec<_>>());

    assert_eq!(AlgorithmId::from(0), AlgorithmId::Placeholder);
    assert_eq!(AlgorithmId::from(1), AlgorithmId::MultiBls12_381);
    assert_eq!(AlgorithmId::from(2), AlgorithmId::ThresBls12_381);
    assert_eq!(AlgorithmId::from(3), AlgorithmId::SchnorrSecp256k1);
    assert_eq!(AlgorithmId::from(4), AlgorithmId::StaticDhSecp256k1);
    assert_eq!(AlgorithmId::from(5), AlgorithmId::HashSha256);
    assert_eq!(AlgorithmId::from(6), AlgorithmId::Tls);
    assert_eq!(AlgorithmId::from(7), AlgorithmId::Ed25519);
    assert_eq!(AlgorithmId::from(8), AlgorithmId::Secp256k1);
    assert_eq!(AlgorithmId::from(9), AlgorithmId::Groth20_Bls12_381);
    assert_eq!(AlgorithmId::from(10), AlgorithmId::NiDkg_Groth20_Bls12_381);
    assert_eq!(AlgorithmId::from(11), AlgorithmId::EcdsaP256);
    assert_eq!(AlgorithmId::from(12), AlgorithmId::EcdsaSecp256k1);
    assert_eq!(AlgorithmId::from(13), AlgorithmId::IcCanisterSignature);
    assert_eq!(AlgorithmId::from(14), AlgorithmId::RsaSha256);
    assert_eq!(AlgorithmId::from(15), AlgorithmId::ThresholdEcdsaSecp256k1);
    assert_eq!(AlgorithmId::from(16), AlgorithmId::MegaSecp256k1);

    // Verify that an unknown i32 maps onto Placeholder
    assert_eq!(AlgorithmId::from(42), AlgorithmId::Placeholder);
}

#[test]
fn should_correctly_convert_algorithm_id_to_i32() {
    ensure_all_algorithm_ids_are_compared(&(0..=16).collect::<Vec<_>>());

    assert_eq!(AlgorithmId::Placeholder as i32, 0);
    assert_eq!(AlgorithmId::MultiBls12_381 as i32, 1);
    assert_eq!(AlgorithmId::ThresBls12_381 as i32, 2);
    assert_eq!(AlgorithmId::SchnorrSecp256k1 as i32, 3);
    assert_eq!(AlgorithmId::StaticDhSecp256k1 as i32, 4);
    assert_eq!(AlgorithmId::HashSha256 as i32, 5);
    assert_eq!(AlgorithmId::Tls as i32, 6);
    assert_eq!(AlgorithmId::Ed25519 as i32, 7);
    assert_eq!(AlgorithmId::Secp256k1 as i32, 8);
    assert_eq!(AlgorithmId::Groth20_Bls12_381 as i32, 9);
    assert_eq!(AlgorithmId::NiDkg_Groth20_Bls12_381 as i32, 10);
    assert_eq!(AlgorithmId::EcdsaP256 as i32, 11);
    assert_eq!(AlgorithmId::EcdsaSecp256k1 as i32, 12);
    assert_eq!(AlgorithmId::IcCanisterSignature as i32, 13);
    assert_eq!(AlgorithmId::RsaSha256 as i32, 14);
    assert_eq!(AlgorithmId::ThresholdEcdsaSecp256k1 as i32, 15);
    assert_eq!(AlgorithmId::MegaSecp256k1 as i32, 16)
}

#[test]
fn should_correctly_convert_algorithm_id_to_u8() {
    ensure_all_algorithm_ids_are_compared(&(0..=16).collect::<Vec<_>>());

    let tests: Vec<(AlgorithmId, u8)> = vec![
        (AlgorithmId::Placeholder, 0),
        (AlgorithmId::MultiBls12_381, 1),
        (AlgorithmId::ThresBls12_381, 2),
        (AlgorithmId::SchnorrSecp256k1, 3),
        (AlgorithmId::StaticDhSecp256k1, 4),
        (AlgorithmId::HashSha256, 5),
        (AlgorithmId::Tls, 6),
        (AlgorithmId::Ed25519, 7),
        (AlgorithmId::Secp256k1, 8),
        (AlgorithmId::Groth20_Bls12_381, 9),
        (AlgorithmId::NiDkg_Groth20_Bls12_381, 10),
        (AlgorithmId::EcdsaP256, 11),
        (AlgorithmId::EcdsaSecp256k1, 12),
        (AlgorithmId::IcCanisterSignature, 13),
        (AlgorithmId::RsaSha256, 14),
        (AlgorithmId::ThresholdEcdsaSecp256k1, 15),
        (AlgorithmId::MegaSecp256k1, 16),
    ];

    for (algorithm_id, expected_discriminant) in tests {
        assert_eq!(algorithm_id.as_u8(), expected_discriminant);
    }
}

#[test]
fn should_correctly_convert_usize_to_key_purpose() {
    // ensure _all_ key purposes are compared (i.e., no key purpose was forgotten)
    assert_eq!(KeyPurpose::iter().count(), 6);

    assert_eq!(KeyPurpose::from(0), KeyPurpose::Placeholder);
    assert_eq!(KeyPurpose::from(1), KeyPurpose::NodeSigning);
    assert_eq!(KeyPurpose::from(2), KeyPurpose::QueryResponseSigning);
    assert_eq!(KeyPurpose::from(3), KeyPurpose::DkgDealingEncryption);
    assert_eq!(KeyPurpose::from(4), KeyPurpose::CommitteeSigning);
    assert_eq!(KeyPurpose::from(5), KeyPurpose::IDkgMEGaEncryption);

    // Verify that an unknown usize maps onto Placeholder
    assert_eq!(AlgorithmId::from(42), AlgorithmId::Placeholder);
}

#[cfg(test)]
impl KeyPurpose {
    fn as_str(&self) -> &'static str {
        match self {
            KeyPurpose::Placeholder => "",
            KeyPurpose::NodeSigning => "node_signing",
            KeyPurpose::QueryResponseSigning => "query_response_signing",
            KeyPurpose::DkgDealingEncryption => "dkg_dealing_encryption",
            KeyPurpose::CommitteeSigning => "committee_signing",
            KeyPurpose::IDkgMEGaEncryption => "idkg_mega_encryption",
        }
    }
}

#[test]
fn should_correctly_convert_between_enum_and_string() {
    for i in 0..KeyPurpose::iter().count() {
        if i == 0 {
            continue;
        }
        let key_purpose = KeyPurpose::from(i);
        let converted_key_purpose = key_purpose.as_str();
        assert_eq!(
            KeyPurpose::from_str(converted_key_purpose).unwrap(),
            key_purpose
        );
    }
}

pub fn set_of(node_ids: &[NodeId]) -> BTreeSet<NodeId> {
    let mut dealers = BTreeSet::new();
    node_ids.iter().for_each(|node_id| {
        dealers.insert(*node_id);
    });
    dealers
}

fn ensure_all_algorithm_ids_are_compared(tested_algorithm_ids: &[isize]) {
    let all_algorithm_ids: Vec<isize> = (0..=16).collect();
    assert_eq!(tested_algorithm_ids, all_algorithm_ids);
}

mod current_node_public_keys {
    use super::*;

    const SOME_PUBLIC_KEY: Option<PublicKey> = Some(PublicKey {
        version: 0,
        algorithm: 0,
        key_value: vec![],
        proof_data: None,
        timestamp: None,
    });
    const SOME_X509_CERT: Option<X509PublicKeyCert> = Some(X509PublicKeyCert {
        certificate_der: vec![],
    });

    #[test]
    fn should_count_correctly_empty_node_public_keys() {
        let node_public_keys = CurrentNodePublicKeys {
            node_signing_public_key: None,
            committee_signing_public_key: None,
            tls_certificate: None,
            dkg_dealing_encryption_public_key: None,
            idkg_dealing_encryption_public_key: None,
        };
        assert_eq!(0, node_public_keys.get_pub_keys_and_cert_count());
    }

    #[test]
    fn should_count_correctly_full_node_public_keys() {
        let node_public_keys = CurrentNodePublicKeys {
            node_signing_public_key: SOME_PUBLIC_KEY,
            committee_signing_public_key: SOME_PUBLIC_KEY,
            tls_certificate: SOME_X509_CERT,
            dkg_dealing_encryption_public_key: SOME_PUBLIC_KEY,
            idkg_dealing_encryption_public_key: SOME_PUBLIC_KEY,
        };
        assert_eq!(5, node_public_keys.get_pub_keys_and_cert_count());
    }

    #[test]
    fn should_count_correctly_partial_node_public_keys() {
        let node_public_keys = CurrentNodePublicKeys {
            node_signing_public_key: SOME_PUBLIC_KEY,
            committee_signing_public_key: None,
            tls_certificate: SOME_X509_CERT,
            dkg_dealing_encryption_public_key: None,
            idkg_dealing_encryption_public_key: SOME_PUBLIC_KEY,
        };
        assert_eq!(3, node_public_keys.get_pub_keys_and_cert_count());
    }
}
