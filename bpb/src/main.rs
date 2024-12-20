#[macro_use]
extern crate failure;
#[macro_use]
extern crate serde_derive;

mod config;
mod key_data;
mod keychain;
mod legacy_config;
mod tests;

use std::time::SystemTime;

use ed25519_dalek as ed25519;
use failure::Error;
use keychain::{add_keychain_item, get_keychain_item};
use rand::RngCore;

use crate::config::Config;
use crate::key_data::KeyData;
use crate::legacy_config::LegacyConfig;

fn main() -> Result<(), Error> {
    let mut args = std::env::args().skip(1);
    match args.next().as_ref().map(|s| &s[..]) {
        Some("init") => {
            if let Some(userid) = args.next() {
                generate_keypair(userid)
            } else {
                bail!("Must specify a userid argument, e.g.: `bpb init \"username <email>\"`")
            }
        }
        Some("import") => import(),
        Some("upgrade") => upgrade(),
        Some("print") => print_public_key(),
        Some("--help") => print_help_message(),
        Some(arg) if gpg_sign_arg(arg) => verify_commit(),
        _ => {
            if args.any(|arg| gpg_sign_arg(&arg)) {
                verify_commit()
            } else {
                delegate()
            }
        }
    }
}

fn gpg_sign_arg(arg: &str) -> bool {
    arg == "--sign" || (arg.starts_with('-') && !arg.starts_with("--") && arg.contains('s'))
}

fn print_help_message() -> Result<(), Error> {
    println!("bpb: boats's personal barricade\n");
    println!("A program for signing git commits.\n");
    println!("Arguments:");
    println!("    init <userid>:    Generate a keypair and store in the keychain.");
    println!("    print:            Print public key in OpenPGP format.\n");
    println!("See https://github.com/pkgxdev/bpb for more information.");
    Ok(())
}

fn generate_keypair(userid: String) -> Result<(), Error> {
    if let Ok(_config) = Config::load() {
        eprintln!(
            "A keypair already exists. If you (really) want to reinitialize your state\n\
                   run `security delete-generic-password -s {}` first.",
            _config.service()
        );
        return Ok(());
    }

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();

    let mut rng = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut rng[0..32]);
    let keypair = ed25519::SigningKey::from_bytes(&rng);

    let public_key = hex::encode(keypair.verifying_key().as_bytes());
    let config = Config::create(public_key, userid, timestamp)?;
    config.write()?;

    let service = config.service();
    let account = config.user_id();
    let hex = hex::encode(keypair.to_bytes());
    add_keychain_item(service, account, &hex)?;

    let keydata = KeyData::load(&config, keypair.to_bytes())?;
    println!("{}", keydata.public());

    Ok(())
}

fn print_public_key() -> Result<(), Error> {
    let config = Config::load()?;
    let service = config.service();
    let account = config.user_id();
    let secret_str = get_keychain_item(service, account)?;
    let secret = to_32_bytes(&secret_str)?;

    let keypair = KeyData::load(&config, secret)?;
    println!("{}", keypair.public());
    Ok(())
}

fn verify_commit() -> Result<(), Error> {
    use std::io::Read;

    let mut commit = String::new();
    let mut stdin = std::io::stdin();
    stdin.read_to_string(&mut commit)?;

    let config = Config::load()?;
    let service = config.service();
    let account = config.user_id();
    let secret_str = get_keychain_item(service, account)?;
    let secret = to_32_bytes(&secret_str)?;

    let config = Config::load()?;
    let keypair = KeyData::load(&config, secret)?;

    let sig = keypair.sign(commit.as_bytes())?;

    eprintln!("\n[GNUPG:] SIG_CREATED ");
    println!("{}", sig);
    Ok(())
}

fn delegate() -> ! {
    use std::process;

    let mut cmd = process::Command::new("gpg");
    cmd.args(std::env::args().skip(1));
    let status = cmd.status().unwrap().code().unwrap();
    process::exit(status)
}

fn upgrade() -> Result<(), Error> {
    let mut file = std::fs::File::open(legacy_keys_file())?;
    let (config, secret) = LegacyConfig::convert(&mut file)?;
    let service = config.service();
    let account = config.user_id();
    let hex = hex::encode(secret);
    add_keychain_item(service, account, &hex)?;
    config.write()
}

fn import() -> Result<(), Error> {
    let config = Config::load()?;
    let service = config.service();
    let account = config.user_id();

    let key = std::env::args().nth(2).unwrap();
    add_keychain_item(service, account, &key)
}

fn legacy_keys_file() -> String {
    std::env::var("BPB_KEYS")
        .unwrap_or_else(|_| format!("{}/.bpb_keys.toml", std::env::var("HOME").unwrap()))
}

fn to_32_bytes(slice: &String) -> Result<[u8; 32], Error> {
    let vector = hex::decode(slice)?;
    let mut array = [0u8; 32];
    let len = std::cmp::min(vector.len(), 32);
    array[..len].copy_from_slice(&vector[..len]);
    Ok(array)
}
