use std::{path::PathBuf, io::Write};
use utils::term::{Term, style::{Style}};
use super::core::{self, StagingId, StagingTransaction};

/// function to create a new empty transaction
pub fn new( mut term: Term
          , root_dir: PathBuf
          )
{
    let staging = match StagingTransaction::new(root_dir) {
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

pub fn finalize( mut term: Term
               , root_dir: PathBuf
               , id_str: &str
               )
{
    let mut staging = load_staging(&mut term, root_dir, id_str);
    unimplemented!()
}

pub fn status( mut term: Term
             , root_dir: PathBuf
             , id_str: &str
             )
{
    let staging = load_staging(&mut term, root_dir, id_str);

    let export = staging.export();

    ::serde_yaml::to_writer(&mut term, &export).unwrap();
}

pub fn add_input( mut term: Term
                , root_dir: PathBuf
                , id_str: &str
                )
{
    let mut staging = load_staging(&mut term, root_dir, id_str);

    unimplemented!()
}

pub fn add_output( mut term: Term
                 , root_dir: PathBuf
                 , id_str: &str
                 )
{
    let mut staging = load_staging(&mut term, root_dir, id_str);
    unimplemented!()
}

pub fn remove_input( mut term: Term
                   , root_dir: PathBuf
                   , id_str: &str
                   )
{
    let mut staging = load_staging(&mut term, root_dir, id_str);
    unimplemented!()
}

pub fn remove_output( mut term: Term
                    , root_dir: PathBuf
                    , id_str: &str
                    )
{
    let mut staging = load_staging(&mut term, root_dir, id_str);
    unimplemented!()
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
