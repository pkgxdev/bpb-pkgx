use std::time::SystemTime;

use ed25519_dalek as ed25519;
use failure::Error;

use crate::config::Config;

pub struct KeyData {
    keypair: ed25519::SigningKey,
    user_id: String,
    timestamp: u64,
}

impl KeyData {
    pub fn create(keypair: ed25519::SigningKey, user_id: String, timestamp: u64) -> KeyData {
        KeyData {
            keypair,
            user_id,
            timestamp,
        }
    }

    pub fn load(config: &Config) -> Result<KeyData, Error> {
        let keypair = ed25519::SigningKey::from_bytes(&config.secret()?);
        Ok(KeyData::create(
            keypair,
            config.user_id().to_owned(),
            config.timestamp(),
        ))
    }

    pub fn sign(&self, data: &[u8]) -> Result<pbp_pkgx::PgpSig, Error> {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();
        Ok(pbp_pkgx::PgpSig::from_dalek::<sha2::Sha256, sha2::Sha512>(
            &self.keypair,
            data,
            self.fingerprint(),
            pbp_pkgx::SigType::BinaryDocument,
            timestamp as u32,
        ))
    }

    pub fn keypair(&self) -> &ed25519::SigningKey {
        &self.keypair
    }

    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    pub fn fingerprint(&self) -> pbp_pkgx::Fingerprint {
        self.public().fingerprint()
    }

    pub fn public(&self) -> pbp_pkgx::PgpKey {
        pbp_pkgx::PgpKey::from_dalek::<sha2::Sha256, sha2::Sha512>(
            &self.keypair,
            pbp_pkgx::KeyFlags::SIGN | pbp_pkgx::KeyFlags::CERTIFY,
            self.timestamp as u32,
            &self.user_id,
        )
    }
}
