use std;
use std::fmt;
use std::string::String;

use blockchain::genesis;
use blockchain::normal;
use blockchain::{Block, SscProof};
use wallet_crypto;

use ansi_term::Colour;

// Constants for the fmt::Display instance
static DISPLAY_INDENT_SIZE: usize = 4; // spaces
static DISPLAY_INDENT_LEVEL: usize = 0; // beginning starts at zero

type AST = Vec<(Key, Val)>;

type Key = String;

// TODO: extend with blockchain-specific constructors with color
pub enum Val {
    Raw(String),
    Tree(AST),
}

pub trait Pretty {
    fn to_pretty(&self) -> Val;
}

fn longest_key_length(ast: &[(Key, Val)]) -> usize {
    ast.iter()
        .fold(0, |longest, (key, _)| std::cmp::max(longest, key.len()))
}

fn fmt_key(
    key: &Key,
    f: &mut fmt::Formatter,
    indent_size: usize,
    indent_level: usize,
    key_width: usize,
) -> fmt::Result {
    write!(
        f,
        "{:>iw$}- {:<kw$}:",
        "",
        key,
        kw = key_width,
        iw = indent_size * indent_level,
    )
}

fn fmt_val(
    val: &Val,
    f: &mut fmt::Formatter,
    indent_size: usize,
    indent_level: usize,
) -> fmt::Result {
    match val {
        // write inline
        Val::Raw(_) => {
            write!(f, " ")?;
            fmt_pretty(val, f, indent_size, indent_level)?;
            write!(f, "\n")
        }
        // write on the next line
        Val::Tree(_) => {
            write!(f, "\n")?;
            fmt_pretty(val, f, indent_size, indent_level)
        }
    }
    // XXX: DRY up the duplicate calls to `fmt_pretty`?
}

fn fmt_pretty(
    p: &Val,
    f: &mut fmt::Formatter,
    indent_size: usize,
    indent_level: usize,
) -> fmt::Result {
    match p {
        // format pretty-val as a terminal
        Val::Raw(display) => write!(f, "{}", display),
        // format pretty-val as a set of  key-vals
        Val::Tree(ast) => {
            let key_width = longest_key_length(ast);
            ast.iter().fold(Ok(()), |prev_result, (key, val)| {
                prev_result.and_then(|()| {
                    fmt_key(key, f, indent_size, indent_level, key_width)?;
                    fmt_val(val, f, indent_size, indent_level + 1)
                })
            })
        }
    }
}

impl fmt::Display for Val {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt_pretty(self, f, DISPLAY_INDENT_SIZE, DISPLAY_INDENT_LEVEL)
    }
}

// the rest of the file is `impl` and `test`

impl Pretty for str {
    fn to_pretty(&self) -> Val {
        Val::Raw(self.to_string())
    }
}

impl Pretty for Block {
    fn to_pretty(&self) -> Val {
        match self {
            Block::GenesisBlock(b) => Val::Tree(vec![("GenesisBlock".to_string(), b.to_pretty())]),
            Block::MainBlock(b) => Val::Tree(vec![("MainBlock".to_string(), b.to_pretty())]),
        }
    }
}

impl Pretty for normal::Block {
    fn to_pretty(&self) -> Val {
        Val::Tree(vec![
            ("header".to_string(), self.header.to_pretty()),
            ("body".to_string(), self.body.to_pretty()),
            (
                "extra".to_string(),
                Val::Raw(format!("TODO {:?}", self.extra)),
            ),
        ])
    }
}

impl Pretty for normal::BlockHeader {
    fn to_pretty(&self) -> Val {
        Val::Tree(vec![
            (
                "protocol magic".to_string(),
                Val::Raw(format!("{}", self.protocol_magic)),
            ),
            (
                "previous hash".to_string(),
                Val::Raw(
                    Colour::Green
                        .paint(wallet_crypto::util::hex::encode(
                            self.previous_header.as_ref(),
                        ))
                        .to_string(),
                ),
            ),
            ("body proof".to_string(), self.body_proof.to_pretty()),
            ("consensus".to_string(), self.consensus.to_pretty()),
            (
                "extra data".to_string(),
                Val::Raw(format!("TODO {:?}", self.extra_data)),
            ),
        ])
    }
}

impl Pretty for normal::BodyProof {
    fn to_pretty(&self) -> Val {
        Val::Tree(vec![
            ("tx proof".to_string(), self.tx.to_pretty()),
            ("mpc".to_string(), self.mpc.to_pretty()),
            ("proxy sk".to_string(), self.proxy_sk.to_pretty()),
            ("update".to_string(), self.update.to_pretty()),
        ])
    }
}

impl Pretty for wallet_crypto::tx::TxProof {
    fn to_pretty(&self) -> Val {
        Val::Tree(vec![
            ("number".to_string(), Val::Raw(format!("{}", self.number))),
            ("root".to_string(), self.root.to_pretty()),
            ("witness hash".to_string(), self.witnesses_hash.to_pretty()),
        ])
    }
}

impl Pretty for wallet_crypto::hash::Blake2b256 {
    fn to_pretty(&self) -> Val {
        Val::Raw(format!("{}", self))
    }
}

impl Pretty for SscProof {
    fn to_pretty(&self) -> Val {
        Val::Raw(format!("{:?}", self))
    }
}

impl Pretty for normal::Consensus {
    fn to_pretty(&self) -> Val {
        Val::Tree(vec![
            (
                "slot id".to_string(),
                Val::Raw(format!("{:?}", self.slot_id)),
            ),
            (
                "leader key".to_string(),
                Val::Raw(wallet_crypto::util::hex::encode(self.leader_key.as_ref())),
            ),
            (
                "chain difficulty".to_string(),
                Val::Raw(format!("{}", self.chain_difficulty)),
            ),
            (
                "block signature".to_string(),
                Val::Raw(
                    Colour::Cyan
                        .paint(format!("{:?}", self.block_signature))
                        .to_string(),
                ),
            ),
        ])
    }
}

impl Pretty for normal::Body {
    fn to_pretty(&self) -> Val {
        Val::Tree(vec![
            (
                "tx-payload".to_string(),
                Val::Raw(format!("TODO {}", self.tx)),
            ),
            ("scc".to_string(), Val::Raw(format!("TODO {:?}", self.scc))),
            (
                "delegation".to_string(),
                Val::Raw(format!("TODO {:?}", self.delegation)),
            ),
            (
                "update".to_string(),
                Val::Raw(format!("TODO {:?}", self.update)),
            ),
        ])
    }
}

impl Pretty for genesis::Block {
    fn to_pretty(&self) -> Val {
        Val::Tree(vec![
            (
                "header".to_string(),
                Val::Raw(format!("TODO {}", self.header)),
            ),
            (
                "body".to_string(),
                Val::Raw(format!("TODO {:?}", self.body)),
            ),
            (
                "extra".to_string(),
                Val::Raw(format!("TODO {:?}", self.extra)),
            ),
        ])
    }
}

#[cfg(test)]
mod tests {
    use std::vec::Vec;

    use command::pretty::Val::*;
    use command::pretty::*;

    #[test]
    fn test_display_single() {
        assert_eq!(format!("{}", Raw(format!("{}", 123))), "123");
    }
    #[test]
    fn longest_key_length_works() {
        let mut input = Vec::new();
        input.push(("name".to_string(), Raw("zaphod".to_string())));
        input.push(("age".to_string(), Raw(format!("{}", 42))));
        assert_eq!(longest_key_length(&input), 4);
    }
    #[test]
    fn test_display_flat_pairs() {
        let mut input = Vec::new();
        input.push(("name".to_string(), Raw("zaphod".to_string())));
        input.push(("age".to_string(), Raw(format!("{}", 42))));
        assert_eq!(
            format!("{}", Tree(input)),
            "\
- name: zaphod
- age : 42
"
        );
    }
    #[test]
    fn test_display_nested_pairs() {
        let mut nested = Vec::new();
        nested.push(("name".to_string(), Raw("zaphod".to_string())));
        nested.push(("age".to_string(), Raw(format!("{}", 42))));
        let mut input = Vec::new();
        input.push(("character".to_string(), Tree(nested)));
        input.push(("crook".to_string(), Raw("yes".to_string())));
        assert_eq!(
            format!("{}", Tree(input)),
            "\
- character:
    - name: zaphod
    - age : 42
- crook    : yes
"
        );
    }
}
