use std::env;

use clap::{App, Arg};

use rustica_keys::yubikey::ssh::convert_to_ssh_pubkey;
use rustica_keys::yubikey::{RetiredSlotId, SlotId, provision};

use yubikey_piv::key::AlgorithmId;
use yubikey_piv::policy::TouchPolicy;

use std::convert::TryFrom;

fn provision_new_key(slot: SlotId, pin: &str, mgm_key: &[u8], alg: &str, secure: bool) {
    let alg = match alg {
        "eccp256" => AlgorithmId::EccP256,
        _ => AlgorithmId::EccP384,
    };

    println!("Provisioning new {:?} key in slot: {:?}", alg, slot);

    let policy = if secure {
        println!("You're creating a secure key that will require touch to use. Touch key to continue...");
        TouchPolicy::Cached
    } else {
        TouchPolicy::Never
    };

    match provision(pin.as_bytes(), mgm_key, slot, alg, policy) {
        Ok(pk) => {
            convert_to_ssh_pubkey(&pk).unwrap();
        },
        Err(e) => panic!("Could not provision device with new key: {:?}", e),
    }
}

fn slot_parser(slot: &str) -> Option<SlotId> {
    // If first character is R, then we need to parse the nice
    // notation
    if (slot.len() == 2 || slot.len() == 3) && slot.starts_with('R') {
        let slot_value = slot[1..].parse::<u8>();
        match slot_value {
            Ok(v) if v <= 20 => Some(SlotId::try_from(0x81_u8 + v).unwrap()),
            _ => None,
        }
    } else if let Ok(s) = SlotId::try_from(slot.to_owned()) {
        Some(s)
    } else {
        None
    }
}

fn slot_validator(slot: &str) -> Result<(), String> {
    match slot_parser(slot) {
        Some(_) => Ok(()),
        None => Err(String::from("Provided slot was not valid. Should be R1 - R20 or a raw hex identifier")),
    }
}


fn main() {
    let matches = App::new("yk-provision")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Mitchell Grenier <mitchell@confurious.io>")
        .about("A tool to provision a new key on a yubikey")
        .arg(
            Arg::new("slot")
                .about("Numerical value for the slot on the yubikey to use for your private key")
                .long("slot")
                .short('s')
                .required(true)
                .validator(slot_validator)
                .takes_value(true),
        )
        .arg(
            Arg::new("pin")
                .about("Provision this slot with a new private key. The pin number must be passed as parameter here")
                .long("pin")
                .short('p')
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::new("management-key")
                .about("Provision this slot with a new private key. The pin number must be passed as parameter here")
                .default_value("010203040506070801020304050607080102030405060708")
                .long("mgmkey")
                .short('m')
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::new("type")
                .about("Specify the type of key you want to provision (eccp256, eccp384)")
                .long("type")
                .short('t')
                .possible_value("eccp256")
                .possible_value("eccp384")
                .takes_value(true),
        )
        .arg(
            Arg::new("require-touch")
                .about("Newly provisioned key requires touch for signing operations (touch cached for 15 seconds)")
                .long("require-touch")
                .short('r')
        )
        .get_matches();

        let slot = match matches.value_of("slot") {
            // We unwrap here because we have already run the validator above
            Some(x) => slot_parser(x).unwrap(),
            None => SlotId::Retired(RetiredSlotId::R17),
        };
    
        let secure = matches.is_present("require-touch");

        provision_new_key(
            slot,
            matches.value_of("pin").unwrap(),
            &hex::decode(matches.value_of("management-key").unwrap()).unwrap(),
            matches.value_of("type").unwrap_or("eccp384"),
            secure
        );
}