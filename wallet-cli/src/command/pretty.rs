use std;
use std::fmt;
use std::string::String;

use blockchain::{genesis, normal, types, Block, SscProof};
use wallet_crypto::{hash, tx, util::hex};

use ansi_term::Colour;

// Constants for the fmt::Display instance
static DISPLAY_INDENT_SIZE: usize = 4; // spaces
static DISPLAY_INDENT_LEVEL: usize = 0; // beginning starts at zero

type AST = Vec<(Key, Val)>;

type Key = String;

// XXX: consider splitting into two mutually-recursive types (one with only terminals, one with only nonterminals)
// TODO: extend with blockchain-specific constructors with color
pub enum Val {
    // terminals
    Raw(String),
    Hash(types::HeaderHash),
    Epoch(u32),
    SlotId(u32),
    BlockSig(normal::BlockSignature),

    // recursive
    List(Vec<Val>),
    Tree(AST),
}

pub trait Pretty {
    fn to_pretty(&self) -> Val;
}

fn longest_key_length(ast: &[(Key, Val)]) -> usize {
    ast.iter()
        .fold(0, |longest, (key, _)| std::cmp::max(longest, key.len()))
}

fn fmt_indent(f: &mut fmt::Formatter, indent_size: usize, indent_level: usize) -> fmt::Result {
    write!(f, "{:>iw$}", "", iw = indent_size * indent_level,)
}

fn fmt_key(key: &Key, f: &mut fmt::Formatter, key_width: usize) -> fmt::Result {
    write!(f, "- {:<kw$}:", key, kw = key_width,)
}

// XXX: DRY up the duplicate calls to `fmt_pretty`?
fn fmt_val(
    val: &Val,
    f: &mut fmt::Formatter,
    indent_size: usize,
    indent_level: usize,
) -> fmt::Result {
    match val {
        // write terminals inline
        Val::Raw(_) | Val::Hash(_) | Val::Epoch(_) | Val::SlotId(_) | Val::BlockSig(_) => {
            write!(f, " ")?;
            fmt_pretty(val, f, indent_size, indent_level)?;
            write!(f, "\n")
        }

        // write nonterminals on the next line
        Val::List(_) | Val::Tree(_) => {
            write!(f, "\n")?;
            fmt_pretty(val, f, indent_size, indent_level)
        }
    }
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
        Val::Hash(hash) => write!(f, "{}", Colour::Green.paint(hex::encode(hash.as_ref()))),
        Val::Epoch(epoch) => write!(f, "{}", Colour::Blue.paint(format!("{}", epoch))),
        Val::SlotId(slotid) => write!(f, "{}", Colour::Purple.paint(format!("{}", slotid))),
        Val::BlockSig(block_signature) => write!(
            f,
            "{}",
            Colour::Cyan.paint(format!("{:?}", block_signature))
        ),

        // format pretty-val as a set of key-vals
        Val::Tree(ast) => {
            let key_width = longest_key_length(ast);
            ast.iter().fold(Ok(()), |prev_result, (key, val)| {
                prev_result.and_then(|()| {
                    fmt_indent(f, indent_size, indent_level)?;
                    fmt_key(key, f, key_width)?;
                    fmt_val(val, f, indent_size, indent_level + 1)
                })
            })
        }

        // format pretty-val as a sequence of vals
        Val::List(vals) => vals.iter().fold(Ok(()), |prev_result, val| {
            prev_result.and_then(|()| {
                fmt_indent(f, indent_size, indent_level)?;
                write!(f, "*")?;
                fmt_val(val, f, indent_size, indent_level + 1)
            })
        }),
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
                self.previous_header.to_pretty(),
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

impl Pretty for types::HeaderHash {
    fn to_pretty(&self) -> Val {
        Val::Hash(self.clone())
    }
}

impl Pretty for genesis::BlockHeader {
    fn to_pretty(&self) -> Val {
        Val::Tree(vec![
            (
                "protocol magic".to_string(),
                Val::Raw(format!("{}", self.protocol_magic)),
            ),
            (
                "previous hash".to_string(),
                self.previous_header.to_pretty(),
            ),
            (
                "body proof".to_string(),
                Val::Raw(format!("{:?}", self.body_proof)),
            ),
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

impl Pretty for tx::TxProof {
    fn to_pretty(&self) -> Val {
        Val::Tree(vec![
            ("number".to_string(), Val::Raw(format!("{}", self.number))),
            ("root".to_string(), self.root.to_pretty()),
            ("witness hash".to_string(), self.witnesses_hash.to_pretty()),
        ])
    }
}

impl Pretty for hash::Blake2b256 {
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
            ("slot".to_string(), self.slot_id.to_pretty()),
            (
                "leader key".to_string(),
                Val::Raw(hex::encode(self.leader_key.as_ref())),
            ),
            (
                "chain difficulty".to_string(),
                Val::Raw(format!("{}", self.chain_difficulty)),
            ),
            (
                "block signature".to_string(),
                self.block_signature.to_pretty(),
            ),
        ])
    }
}

impl Pretty for normal::BlockSignature {
    fn to_pretty(&self) -> Val {
        Val::BlockSig(self.clone())
    }
}

impl Pretty for types::SlotId {
    fn to_pretty(&self) -> Val {
        Val::Tree(vec![
            ("epoch".to_string(), self.epoch.to_pretty()),
            ("slot id".to_string(), Val::SlotId(self.slotid)),
        ])
    }
}

impl Pretty for types::EpochId {
    fn to_pretty(&self) -> Val {
        Val::Epoch(*self)
    }
}

impl Pretty for genesis::Consensus {
    fn to_pretty(&self) -> Val {
        Val::Tree(vec![
            ("epoch".to_string(), self.epoch.to_pretty()),
            (
                "chain difficulty".to_string(),
                Val::Raw(format!("{}", self.chain_difficulty)),
            ),
        ])
    }
}

impl Pretty for normal::Body {
    fn to_pretty(&self) -> Val {
        Val::Tree(vec![
            ("tx payload".to_string(), self.tx.to_pretty()),
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

impl Pretty for genesis::Body {
    fn to_pretty(&self) -> Val {
        Val::List(
            self.slot_leaders
                .iter()
                .map(|cbor| Val::Raw(format!("{:?}", cbor)))
                .collect(),
        )
    }
}

impl Pretty for normal::TxPayload {
    fn to_pretty(&self) -> Val {
        Val::List(
            self.iter()
                .map(|txaux| {
                    Val::Tree(vec![
                        ("tx".to_string(), txaux.tx.to_pretty()),
                        (
                            "witnesses".to_string(),
                            txaux.witnesses.to_pretty()
                            //Val::Raw(format!("{:?}", txaux.witnesses)),
                        ),
                    ])
                })
                .collect(),
        )
    }
}

// XXX: impl for a parameterized generic type, Vec.. not sure if idiomatic
impl Pretty for Vec<tx::TxInWitness> {
    fn to_pretty(&self) -> Val {
        Val::List(
            self.iter()
                .map(|witness| Val::Raw(format!("TODO {}", witness)))
                .collect(),
        )
    }
}

impl Pretty for tx::Tx {
    fn to_pretty(&self) -> Val {
        Val::Tree(vec![
            (
                "inputs".to_string(),
                Val::List(
                    self.inputs
                        .iter()
                        .map(|input| Val::Raw(format!("{}", input)))
                        .collect(),
                ),
            ),
            (
                "outputs".to_string(),
                Val::List(
                    self.outputs
                        .iter()
                        .map(|input| Val::Raw(format!("{}", input)))
                        .collect(),
                ),
            ),
        ])
    }
}

impl Pretty for genesis::Block {
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

#[cfg(test)]
mod tests {
    use command::pretty::Val::*;
    use command::pretty::*;

    #[test]
    fn test_display_single() {
        assert_eq!(format!("{}", Raw(format!("{}", 123))), "123");
    }
    #[test]
    fn longest_key_length_works() {
        let input = vec![
            ("name".to_string(), Raw("zaphod".to_string())),
            ("age".to_string(), Raw(format!("{}", 42))),
        ];
        assert_eq!(longest_key_length(&input), 4);
    }
    #[test]
    fn test_display_flat_pairs() {
        let input = Tree(vec![
            ("name".to_string(), Raw("zaphod".to_string())),
            ("age".to_string(), Raw(format!("{}", 42))),
        ]);
        assert_eq!(
            format!("{}", input),
            "\
- name: zaphod
- age : 42
"
        );
    }
    #[test]
    fn test_display_nested_pairs() {
        let input = Tree(vec![
            (
                "character".to_string(),
                Tree(vec![
                    ("name".to_string(), Raw("zaphod".to_string())),
                    ("age".to_string(), Raw(format!("{}", 42))),
                ]),
            ),
            ("crook".to_string(), Raw("yes".to_string())),
        ]);
        assert_eq!(
            format!("{}", input),
            "\
- character:
    - name: zaphod
    - age : 42
- crook    : yes
"
        );
    }
    #[test]
    fn test_display_tested_list() {
        let input = Tree(vec![
            (
                "character".to_string(),
                Tree(vec![
                    ("name".to_string(), Raw("zaphod".to_string())),
                    ("age".to_string(), Raw(format!("{}", 42))),
                ]),
            ),
            ("crook".to_string(), Raw("yes".to_string())),
            (
                "facts".to_string(),
                List(vec![
                    Raw("invented pan-galactic gargle blaster".to_string()),
                    Raw("elected president".to_string()),
                    Tree(vec![
                        ("heads".to_string(), Raw(format!("{}", 2))),
                        ("arms".to_string(), Raw(format!("{}", 3))),
                    ]),
                    List(vec![
                        Raw("stole the heart of gold".to_string()),
                        Raw("one hoopy frood".to_string()),
                    ]),
                ]),
            ),
        ]);
        assert_eq!(
            format!("{}", input),
            "\
- character:
    - name: zaphod
    - age : 42
- crook    : yes
- facts    :
    * invented pan-galactic gargle blaster
    * elected president
    *
        - heads: 2
        - arms : 3
    *
        * stole the heart of gold
        * one hoopy frood
"
        );
    }
}
