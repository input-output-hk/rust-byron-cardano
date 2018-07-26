use cardano::{address::{ExtendedAddr, StakeDistribution}, util::{base58, hex}};

use utils::term::Term;

pub fn command_address( mut term: Term
                      , address: String
                      )
{
    let bytes = match base58::decode(&address) {
        Err(err) => {
            term.error(&format!("Invalid Address, should be encoded in base58\n")).unwrap();
            term.error(&format!("{}\n", err)).unwrap();
            ::std::process::exit(1)
        },
        Ok(bytes) => bytes,
    };

    let address = match ExtendedAddr::from_bytes(&bytes) {
        Err(err) => {
            term.error(&format!("Invalid Address\n")).unwrap();
            term.error(&format!("{:?}\n", err)).unwrap();
            ::std::process::exit(2)
        },
        Ok(address) => address,
    };

    term.success("Cardano Extended Address\n").unwrap();
    term.info(&format!("  - address hash:       {}\n", address.addr)).unwrap();
    term.info(&format!("  - address type:       {}\n", address.addr_type)).unwrap();
    if let Some(ref payload) = address.attributes.derivation_path {
        term.info(&format!("  - payload:            {}\n", hex::encode(payload.as_ref()))).unwrap();
    }
    match address.attributes.stake_distribution {
        StakeDistribution::BootstrapEraDistr =>
           term.info("  - stake distribution: bootstrap era\n").unwrap(),
        StakeDistribution::SingleKeyDistr(id) =>
           term.info(&format!("  - stake distribution: {}\n", id)).unwrap(),
    }
}
