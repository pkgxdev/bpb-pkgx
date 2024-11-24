use std::io::{Read, Write};

use failure::Error;

use crate::key_data::KeyData;

#[derive(Serialize, Deserialize)]
pub struct Config {
    public: PublicKey,
    secret: SecretKey,
}

impl Config {
    pub fn create(key_data: &KeyData) -> Result<Config, Error> {
        let keypair = key_data.keypair();
        let userid = key_data.user_id().to_owned();
        let timestamp = key_data.timestamp();
        Ok(Config {
            public: PublicKey {
                key: hex::encode(keypair.verifying_key().as_bytes()),
                userid,
                timestamp,
            },
            secret: SecretKey {
                key: Some(hex::encode(keypair.as_bytes())),
                program: None,
            },
        })
    }

    pub fn load(file: &mut impl Read) -> Result<Config, Error> {
        let mut buf = vec![];
        file.read_to_end(&mut buf)?;
        Ok(toml::from_slice(&buf)?)
    }

    pub fn write(&self, file: &mut impl Write) -> Result<(), Error> {
        Ok(file.write_all(&toml::to_vec(self)?)?)
    }

    pub fn timestamp(&self) -> u64 {
        self.public.timestamp
    }

    pub fn user_id(&self) -> &str {
        &self.public.userid
    }

    // pub fn public(&self) -> &str {
    //     &self.public.key
    // }

    pub fn secret(&self) -> Result<[u8; 32], Error> {
        self.secret.secret()
    }
}

#[derive(Serialize, Deserialize)]
struct PublicKey {
    key: String,
    userid: String,
    timestamp: u64,
}

#[derive(Serialize, Deserialize)]
struct SecretKey {
    key: Option<String>,
    program: Option<String>,
}

impl SecretKey {
    fn secret(&self) -> Result<[u8; 32], Error> {
        if let Some(key) = &self.key {
            to_32_bytes(key)
        } else if let Some(cmd) = &self.program {
            let mut args = cmd.split_whitespace();
            let cmd = args.next().ok_or(failure::err_msg("Missing command"))?;
            let output = std::process::Command::new(cmd).args(args).output().unwrap();
            to_32_bytes(&String::from_utf8(output.stdout)?)
        } else {
            bail!("No secret key or program specified")
        }
    }
}

fn to_32_bytes(slice: &String) -> Result<[u8; 32], Error> {
    let vector = hex::decode(slice)?;
    let mut array = [0u8; 32];
    let len = std::cmp::min(vector.len(), 32);
    array[..len].copy_from_slice(&vector[..len]);
    Ok(array)
}
