use std::{path::PathBuf, io::Write, iter, collections::BTreeMap};
use utils::term::{Term, style::{Style}};
use super::core::{self, StagingId, StagingTransaction};
use super::super::blockchain::{Blockchain};
use super::super::wallet::{Wallets, self, WalletName};
use cardano::{tx::{TxId, TxIn, TxInWitness}, coin::{Coin, sum_coins}, address::{ExtendedAddr}, fee::{LinearFee, FeeAlgorithm}};
use cardano::tx;

/// function to create a new empty transaction
pub fn new( mut term: Term
          , root_dir: PathBuf
          , blockchain: String
          )
{
    let blockchain = Blockchain::load(root_dir.clone(), blockchain);

    let staging = match StagingTransaction::new(root_dir, blockchain.config.protocol_magic) {
        Err(err) => {
            // we should not expect errors at this time, but if it happens
            // we need to report it to the user
            error!("Error while creating a staging transaction: {:?}", err);
            term.error("Cannot create a new staging transaction\n").unwrap();
            ::std::process::exit(1);
        },
        Ok(st) => st
    };

    writeln!(term, "Staging file successfully created: {}", style!(staging.id()));
}

pub fn list( mut term: Term
           , root_dir: PathBuf
           )
{
    let transactions_dir = core::config::transaction_directory(root_dir.clone());

    for entry in ::std::fs::read_dir(transactions_dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            term.warn(&format!("unexpected directory in transaction directory: {:?}", entry.path())).unwrap();
            continue;
        }
        let name = entry.file_name().into_string().unwrap_or_else(|err| {
            panic!("invalid utf8... {:?}", err)
        });

        let staging = load_staging(&mut term, root_dir.clone(), name.as_str());

        writeln!(term, "{}", style!(staging.id())).unwrap();
    }
}

pub fn destroy( mut term: Term
              , root_dir: PathBuf
              , id_str: &str
              )
{
    let staging = load_staging(&mut term, root_dir, id_str);

    if let Err(err) = staging.destroy() {
        error!("{:?}", err);
        term.error("cannot delete the sta").unwrap();
    } else {
        term.success("transaction deleted\n").unwrap();
    }
}

pub fn sign( mut term: Term
           , root_dir: PathBuf
           , id_str: &str
           )
{
    let mut signatures = Vec::new();
    let mut staging = load_staging(&mut term, root_dir.clone(), id_str);
    let mut wallets = BTreeMap::new();
    for (name, wallet) in Wallets::load(root_dir.clone()).unwrap() {
        let state = wallet::utils::create_wallet_state_from_logs(&mut term, &wallet, root_dir.clone(), wallet::state::lookup::accum::Accum::default());
        wallets.insert(name, (wallet, state));
    }

    let txid = staging.to_tx_aux().tx.id();
    let protocol_magic = staging.protocol_magic;

    // TODO: ignore already signed inputs
    for input in staging.transaction().inputs() {
        let txin = input.extract_txin();
        let mut signature = None;
        for (name, (wallet, state)) in wallets.iter() {
            if let Some(utxo) = state.utxos.get(&txin) {
                term.info(
                    &format!(
                        "signing input {}.{} ({})\n",
                        style!(input.transaction_id),
                        style!(input.index_in_transaction),
                        style!(name)
                    )
                ).unwrap();

                signature = Some(wallet::utils::wallet_sign_tx(
                    &mut term, wallet, protocol_magic, &txid, &utxo.credited_address
                ));
            }
        }

        if let Some(signature) = signature {
            signatures.push(signature);
        } else {
            panic!("cannot sign input {:#?}", input)
        }
    }

    for signature in signatures {
        staging.add_signature(signature).unwrap();
    }
}

pub fn status( mut term: Term
             , root_dir: PathBuf
             , id_str: &str
             )
{
    let staging = load_staging(&mut term, root_dir, id_str);

    let trans = staging.transaction();
    let inputs = trans.inputs();
    let input_total = sum_coins(inputs.into_iter().map(|x| x.expected_value)).unwrap();
    let txaux = staging.to_tx_aux();
    let output_total = txaux.tx.get_output_total().unwrap();
    let difference = {
        let i : u64 = input_total.into();
        let o : u64 = output_total.into();
        (i as i64) - (o as i64)
    };

    let fee_alg = LinearFee::default();
    let fake_witnesses : Vec<TxInWitness> = iter::repeat(TxInWitness::fake()).take(inputs.len()).collect();
    let fee = fee_alg.calculate_for_txaux_component(&txaux.tx, &fake_witnesses).unwrap();

    let txbytes_length = tx::txaux_serialize_size(&txaux.tx, &fake_witnesses);

    println!("input-total: {}", input_total);
    println!("output-total: {}", output_total);
    println!("actual-fee: {}", difference);
    println!("fee: {}", fee.to_coin());
    println!("tx-bytes: {}", txbytes_length);

    let export = staging.export();

    ::serde_yaml::to_writer(&mut term, &export).unwrap();
}

pub fn add_input( mut term: Term
                , root_dir: PathBuf
                , id_str: &str
                , input: Option<(TxId, u32, Option<Coin>)>
                )
{
    let mut staging = load_staging(&mut term, root_dir.clone(), id_str);

    if staging.is_finalizing_pending() {
        term.error("Cannot add input to a staging transaction with signatures in").unwrap();
        ::std::process::exit(1);
    }

    let input = if let Some(input) = input {
        match input.2 {
            None => {
                find_input_in_all_utxos(&mut term, root_dir.clone(), input.0, input.1)
            },
            Some(v) => {
                core::Input {
                    transaction_id: input.0,
                    index_in_transaction: input.1,
                    expected_value: v,
                }
            },
        }
    } else {
        // TODO, implement interactive mode
        unimplemented!()
    };

    match staging.add_input(input) {
        Err(err) => panic!("{:?}", err),
        Ok(())   => ()
    }
}

pub fn add_output( mut term: Term
                 , root_dir: PathBuf
                 , id_str: &str
                 , output: Option<(ExtendedAddr, Coin)>
                 )
{
    let mut staging = load_staging(&mut term, root_dir, id_str);

    if staging.is_finalizing_pending() {
        term.error("Cannot add output to a staging transaction with signatures in").unwrap();
        ::std::process::exit(1);
    }

    let output = if let Some(output) = output {
        core::Output {
            address: output.0,
            amount:  output.1
        }
    } else {
        // TODO, implement interactive mode
        unimplemented!()
    };

    match staging.add_output(output) {
        Err(err) => panic!("{:?}", err),
        Ok(())   => ()
    }
}

pub fn add_change( mut term: Term
                 , root_dir: PathBuf
                 , id_str: &str
                 , change: ExtendedAddr
                 )
{
    let mut staging = load_staging(&mut term, root_dir, id_str);

    if staging.is_finalizing_pending() {
        term.error("Cannot add change to a staging transaction with signatures in").unwrap();
        ::std::process::exit(1);
    }

    if staging.transaction.has_change() {
        term.error("multiple change address not supported yet").unwrap();
        ::std::process::exit(1);
    }

    match staging.add_change(change.into()) {
        Err(err) => panic!("{:?}", err),
        Ok(())   => ()
    }
}

pub fn remove_input( mut term: Term
                   , root_dir: PathBuf
                   , id_str: &str
                   , input: Option<(TxId, u32)>
                   )
{
    let mut staging = load_staging(&mut term, root_dir, id_str);

    if staging.is_finalizing_pending() {
        term.error("Cannot remove input to a staging transaction with signatures in").unwrap();
        ::std::process::exit(1);
    }

    let txin = if let Some(input) = input {
        TxIn {
            id: input.0,
            index: input.1
        }
    } else {
        // TODO, implement interactive mode
        unimplemented!()
    };

    match staging.remove_input(txin) {
        Err(err) => panic!("{:?}", err),
        Ok(())   => ()
    }
}

pub fn remove_output( mut term: Term
                    , root_dir: PathBuf
                    , id_str: &str
                    , address: Option<ExtendedAddr>
                    )
{
    let mut staging = load_staging(&mut term, root_dir, id_str);

    if staging.is_finalizing_pending() {
        term.error("Cannot remove output to a staging transaction with signatures in").unwrap();
        ::std::process::exit(1);
    }

    if let Some(addr) = address {
        match staging.remove_outputs_for(&addr) {
            Err(err) => panic!("{:?}", err),
            Ok(())   => ()
        }
    } else {
        // TODO, implement interactive mode
        unimplemented!()
    };
}

pub fn remove_change( mut term: Term
                    , root_dir: PathBuf
                    , id_str: &str
                    , change: ExtendedAddr
                    )
{
    let mut staging = load_staging(&mut term, root_dir, id_str);

    if staging.is_finalizing_pending() {
        term.error("Cannot remove change addresses to a staging transaction with signatures in").unwrap();
        ::std::process::exit(1);
    }

    match staging.remove_change(change) {
        Err(err) => panic!("{:?}", err),
        Ok(())   => ()
    }
}

pub fn export( mut term: Term
             , root_dir: PathBuf
             , id_str: &str
             , export_file: Option<&str>
             )
{
    let staging = load_staging(&mut term, root_dir, id_str);

    let export = staging.export();

    if let Some(export_file) = export_file {
        let mut file = ::std::fs::OpenOptions::new().create(true).write(true).open(export_file).unwrap();
        ::serde_yaml::to_writer(&mut file, &export).unwrap();
    } else {
        ::serde_yaml::to_writer(&mut term, &export).unwrap();
    }
}

pub fn import( mut term: Term
             , root_dir: PathBuf
             , import_file: Option<&str>
             )
{
    let import = if let Some(import_file) = import_file {
        let mut file = ::std::fs::OpenOptions::new().read(true).open(import_file).unwrap();
        ::serde_yaml::from_reader(&mut file).unwrap()
    } else {
        let mut stdin = ::std::io::stdin();
        ::serde_yaml::from_reader(&mut stdin).unwrap()
    };

    let staging = StagingTransaction::import(root_dir, import).unwrap();
    writeln!(&mut term, "Staging transaction `{}' successfully imported",
        style!(staging.id())
    );
}

pub fn input_select( mut term: Term
                   , root_dir: PathBuf
                   , id_str: &str
                   , wallets: Vec<WalletName>
                   )
{
    use ::cardano::{fee::{self, SelectionAlgorithm}, tx, txutils};

    let alg = fee::LinearFee::default();
    let selection_policy = fee::SelectionPolicy::default();

    let staging = load_staging(&mut term, root_dir, id_str);

    if ! staging.transaction().has_change() {
        term.error("cannot select inputs if no change").unwrap();
        ::std::process::exit(1);
    }

    let change_address = staging.transaction().changes()[0].address.clone();
    let output_policy = txutils::OutputPolicy::One(change_address.clone());

    let (fee, selected_inputs, change)
        = match alg.compute(selection_policy, inputs, outputs, &output_policy) {
            Err(err) => { panic!("error {:#?}", err) },
            Ok(v) => v
    };

    if change != Coin::zero() {
        term.info(&format!("using the change address: {} with value {}", change_address, change)).unwrap();
        // add/remove the output change
        staging.remove_change(change_address.clone()).unwrap();
        staging.add_output(core::Output { address : change_address, amount: change }).unwrap();
    }

    unimplemented!()
}

/// helper function to load a staging file
fn load_staging(term: &mut Term, root_dir: PathBuf, id_str: &str) -> StagingTransaction {
    let id = match id_str.parse::<StagingId>() {
        Err(err) => {
            debug!("cannot parse staging id: {:?}", err);
            term.error("Invalid StagingId\n").unwrap();
            ::std::process::exit(1);
        },
        Ok(id) => id
    };

    match StagingTransaction::read_from_file(root_dir, id) {
        Err(err) => {
            error!("Error while loading a staging transaction: {:?}", err);
            term.error("Cannot load the staging transaction\n").unwrap();
            ::std::process::exit(1);
        },
        Ok(st) => st
    }
}

// ----------------------------------- helpers ---------------------------------

// find_input_in_all_utxos(&mut term, root_dir.clone(), &input.0, input.1)
fn find_input_in_all_utxos(term: &mut Term, root_dir: PathBuf, txid: TxId, index: u32) -> core::Input {
    let txin = TxIn { id: txid, index: index };
    for (_, wallet) in Wallets::load(root_dir.clone()).unwrap() {
        let state = wallet::utils::create_wallet_state_from_logs(term, &wallet, root_dir.clone(), wallet::state::lookup::accum::Accum::default());

        if let Some(utxo) = state.utxos.get(&txin) {
            let txin = utxo.extract_txin();
            return core::Input {
                transaction_id: txin.id,
                index_in_transaction: txin.index,
                expected_value: utxo.credited_value,
            };
        }
    }

    term.error(&format!("No input found")).unwrap();
    ::std::process::exit(1);
}
