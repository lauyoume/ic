#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]
//#![deny(missing_docs)]

//! Interface for the cryptographic service provider

pub mod api;
pub mod canister_threshold;
pub mod imported_test_utils;
pub mod imported_utilities;
pub mod key_id;
pub mod keygen;
pub mod public_key_store;
pub mod secret_key_store;
mod signer;
pub mod threshold;
pub mod tls;
pub mod types;
pub mod vault;

pub use crate::vault::api::TlsHandshakeCspVault;
pub use crate::vault::local_csp_vault::LocalCspVault;
pub use crate::vault::remote_csp_vault::run_csp_vault_server;
use crate::vault::remote_csp_vault::RemoteCspVault;

use crate::api::{
    CspIDkgProtocol, CspKeyGenerator, CspSecretKeyStoreChecker, CspSigVerifier, CspSigner,
    CspThresholdEcdsaSigVerifier, CspThresholdEcdsaSigner, CspTlsHandshakeSignerProvider,
    NiDkgCspClient, NodePublicKeyData, ThresholdSignatureCspClient,
};
use crate::public_key_store::read_node_public_keys;
use crate::secret_key_store::SecretKeyStore;
use crate::types::CspPublicKey;
use crate::vault::api::CspVault;
use ic_config::crypto::{CryptoConfig, CspVaultType};
use ic_crypto_internal_logmon::metrics::CryptoMetrics;
use ic_crypto_internal_types::encrypt::forward_secure::CspFsEncryptionPublicKey;
use ic_logger::{info, new_logger, replica_logger::no_op_logger, ReplicaLogger};
use ic_protobuf::crypto::v1::NodePublicKeys;
use ic_types::crypto::CurrentNodePublicKeys;
use key_id::KeyId;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use rand::{CryptoRng, Rng};
use secret_key_store::proto_store::ProtoSecretKeyStore;
use std::convert::TryFrom;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

#[cfg(test)]
mod tests;

const SKS_DATA_FILENAME: &str = "sks_data.pb";
const CANISTER_SKS_DATA_FILENAME: &str = "canister_sks_data.pb";

/// Describes the interface of the crypto service provider (CSP), e.g. for
/// signing and key generation. The Csp struct implements this trait.
pub trait CryptoServiceProvider:
    CspSigner
    + CspSigVerifier
    + CspKeyGenerator
    + ThresholdSignatureCspClient
    + NiDkgCspClient
    + CspIDkgProtocol
    + CspThresholdEcdsaSigner
    + CspThresholdEcdsaSigVerifier
    + CspSecretKeyStoreChecker
    + CspTlsHandshakeSignerProvider
    + NodePublicKeyData
{
}

impl<T> CryptoServiceProvider for T where
    T: CspSigner
        + CspSigVerifier
        + CspKeyGenerator
        + ThresholdSignatureCspClient
        + CspIDkgProtocol
        + CspThresholdEcdsaSigner
        + CspThresholdEcdsaSigVerifier
        + NiDkgCspClient
        + CspSecretKeyStoreChecker
        + CspTlsHandshakeSignerProvider
        + NodePublicKeyData
{
}

struct PublicKeyData {
    node_public_keys: NodePublicKeys,
}

impl TryFrom<NodePublicKeys> for PublicKeyData {
    type Error = String;

    fn try_from(node_public_keys: NodePublicKeys) -> Result<Self, Self::Error> {
        let signing_pk = node_public_keys
            .node_signing_pk
            .to_owned()
            .map(|pk| CspPublicKey::try_from(pk));
        if let Some(Err(e)) = signing_pk {
            return Err(format!(
                "Unsupported public key proto as node signing public key: {:?}",
                e
            ));
        }

        let dkg_pk = node_public_keys
            .dkg_dealing_encryption_pk
            .to_owned()
            .map(|pk| CspFsEncryptionPublicKey::try_from(pk));
        if let Some(Err(e)) = dkg_pk {
            return Err(format!(
                "Unsupported public key proto as dkg dealing encryption public key: {:?}",
                e
            ));
        }

        Ok(PublicKeyData { node_public_keys })
    }
}

/// Implements `CryptoServiceProvider` that uses a `CspVault` for
/// storing and managing secret keys.
pub struct Csp {
    csp_vault: Arc<dyn CspVault>,
    public_key_data: PublicKeyData,
    logger: ReplicaLogger,
}

/// This lock provides the option to add metrics about lock acquisition times.
struct CspRwLock<T> {
    name: String,
    rw_lock: RwLock<T>,
    metrics: Arc<CryptoMetrics>,
}

impl<T> CspRwLock<T> {
    pub fn new_for_rng(content: T, metrics: Arc<CryptoMetrics>) -> Self {
        // Note: The name will appear on metric dashboards and may be used in alerts, do
        // not change this unless you are also updating the monitoring.
        Self::new(content, "csprng".to_string(), metrics)
    }

    pub fn new_for_sks(content: T, metrics: Arc<CryptoMetrics>) -> Self {
        // Note: The name will appear on metric dashboards and may be used in alerts, do
        // not change this unless you are also updating the monitoring.
        Self::new(content, "secret_key_store".to_string(), metrics)
    }

    pub fn new_for_csks(content: T, metrics: Arc<CryptoMetrics>) -> Self {
        // Note: The name will appear on metric dashboards and may be used in alerts, do
        // not change this unless you are also updating the monitoring.
        Self::new(content, "canister_secret_key_store".to_string(), metrics)
    }

    fn new(content: T, lock_name: String, metrics: Arc<CryptoMetrics>) -> Self {
        Self {
            name: lock_name,
            rw_lock: RwLock::new(content),
            metrics,
        }
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        let start_time = self.metrics.now();
        let write_guard = self.rw_lock.write();
        self.observe(&self.metrics, "write", start_time);
        write_guard
    }

    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        let start_time = self.metrics.now();
        let read_guard = self.rw_lock.read();
        self.observe(&self.metrics, "read", start_time);
        read_guard
    }

    fn observe(&self, metrics: &CryptoMetrics, access: &str, start_time: Option<Instant>) {
        metrics.observe_lock_acquisition_duration_seconds(&self.name, access, start_time);
    }
}

impl Csp {
    /// Creates a production-grade crypto service provider.
    ///
    /// If the `config`'s vault type is `UnixSocket`, a `tokio_runtime_handle`
    /// must be provided, which is then used for the `async`hronous
    /// communication with the vault via RPC.
    ///
    /// # Panics
    /// Panics if the `config`'s vault type is `UnixSocket` and
    /// `tokio_runtime_handle` is `None`.
    pub fn new(
        config: &CryptoConfig,
        tokio_runtime_handle: Option<tokio::runtime::Handle>,
        logger: Option<ReplicaLogger>,
        metrics: Arc<CryptoMetrics>,
    ) -> Self {
        match &config.csp_vault_type {
            CspVaultType::InReplica => Self::new_with_in_replica_vault(config, logger, metrics),
            CspVaultType::UnixSocket(socket_path) => Self::new_with_unix_socket_vault(
                socket_path,
                tokio_runtime_handle.expect("missing tokio runtime handle"),
                config,
                logger,
            ),
        }
    }

    fn new_with_in_replica_vault(
        config: &CryptoConfig,
        logger: Option<ReplicaLogger>,
        metrics: Arc<CryptoMetrics>,
    ) -> Self {
        let logger = logger.unwrap_or_else(no_op_logger);
        info!(
            logger,
            "Proceeding with an in-replica csp_vault, CryptoConfig: {:?}", config
        );
        let secret_key_store = ProtoSecretKeyStore::open(
            &config.crypto_root,
            SKS_DATA_FILENAME,
            Some(new_logger!(&logger)),
        );
        let canister_key_store = ProtoSecretKeyStore::open(
            &config.crypto_root,
            CANISTER_SKS_DATA_FILENAME,
            Some(new_logger!(&logger)),
        );
        let csp_vault = Arc::new(LocalCspVault::new(
            secret_key_store,
            canister_key_store,
            metrics,
            new_logger!(&logger),
        ));
        Self::csp_with(&config.crypto_root, logger, csp_vault)
    }

    fn new_with_unix_socket_vault(
        socket_path: &Path,
        rt_handle: tokio::runtime::Handle,
        config: &CryptoConfig,
        logger: Option<ReplicaLogger>,
    ) -> Self {
        let logger = logger.unwrap_or_else(no_op_logger);
        info!(
            logger,
            "Proceeding with a remote csp_vault, CryptoConfig: {:?}", config
        );
        let csp_vault = RemoteCspVault::new(socket_path, rt_handle).unwrap_or_else(|e| {
            panic!(
                "Could not connect to CspVault at socket {:?}: {:?}",
                socket_path, e
            )
        });
        Self::csp_with(&config.crypto_root, logger, Arc::new(csp_vault))
    }

    fn csp_with(pk_path: &Path, logger: ReplicaLogger, csp_vault: Arc<dyn CspVault>) -> Self {
        let node_public_keys = read_node_public_keys(pk_path).unwrap_or_default();
        let public_key_data =
            PublicKeyData::try_from(node_public_keys).expect("invalid node public keys");
        Csp {
            public_key_data,
            csp_vault,
            logger,
        }
    }
}

impl Csp {
    /// Creates a crypto service provider for testing.
    ///
    /// Note: This MUST NOT be used in production as the secrecy of the random
    /// number generator, hence the keys, is not guaranteed.
    pub fn new_with_rng<R: Rng + CryptoRng + Send + Sync + 'static>(
        csprng: R,
        config: &CryptoConfig,
    ) -> Self {
        let node_public_keys = read_node_public_keys(&config.crypto_root).unwrap_or_default();
        let public_key_data =
            PublicKeyData::try_from(node_public_keys).expect("invalid node public keys");
        Csp {
            public_key_data,
            csp_vault: Arc::new(LocalCspVault::new_for_test(
                csprng,
                ProtoSecretKeyStore::open(&config.crypto_root, SKS_DATA_FILENAME, None),
            )),
            logger: no_op_logger(),
        }
    }
}

impl Csp {
    /// Resets public key data according to the given `NodePublicKeys`.
    ///
    /// Note: This is for testing only and MUST NOT be used in production.
    pub fn reset_public_key_data(&mut self, node_public_keys: NodePublicKeys) {
        self.public_key_data =
            PublicKeyData::try_from(node_public_keys).expect("invalid node public keys");
    }
}

impl NodePublicKeyData for Csp {
    fn current_node_public_keys(&self) -> CurrentNodePublicKeys {
        let public_keys = self.public_key_data.node_public_keys.clone();
        CurrentNodePublicKeys {
            node_signing_public_key: public_keys.node_signing_pk,
            committee_signing_public_key: public_keys.committee_signing_pk,
            tls_certificate: public_keys.tls_certificate,
            dkg_dealing_encryption_public_key: public_keys.dkg_dealing_encryption_pk,
            idkg_dealing_encryption_public_key: public_keys.idkg_dealing_encryption_pk,
        }
    }

    fn dkg_dealing_encryption_key_id(&self) -> KeyId {
        CspFsEncryptionPublicKey::try_from(
            self.public_key_data
                .node_public_keys
                .dkg_dealing_encryption_pk
                .to_owned()
                .expect("Missing dkg dealing encryption key id"),
        )
        .map(|pk| KeyId::from(&pk))
        .expect("Unsupported public key proto as dkg dealing encryption public key.")
    }
}

impl Csp {
    /// Creates a crypto service provider for testing.
    ///
    /// Note: This MUST NOT be used in production as the secrecy of the secret
    /// key store and the canister secret key store is not guaranteed.
    pub fn of<R: Rng + CryptoRng + Send + Sync + 'static, S: SecretKeyStore + 'static>(
        csprng: R,
        secret_key_store: S,
    ) -> Self {
        let node_public_keys: NodePublicKeys = Default::default();
        let public_key_data =
            PublicKeyData::try_from(node_public_keys).expect("invalid node public keys");
        Csp {
            public_key_data,
            csp_vault: Arc::new(LocalCspVault::new_for_test(csprng, secret_key_store)),
            logger: no_op_logger(),
        }
    }
}
