use std;
use std::fmt;
use std::string::String;

use blockchain::{genesis, normal, types, Block, SscProof};
use wallet_crypto::{address, cbor, config, hash, hdwallet, redeem, tx, util::hex};

use ansi_term::Colour;

// Constants for the fmt::Display instance
static DISPLAY_INDENT_SIZE: usize = 4; // spaces
static DISPLAY_INDENT_LEVEL: usize = 0; // beginning starts at zero

type AST<'a> = Vec<(Key<'a>, Val<'a>)>;

type Key<'a> = &'a str;

// XXX: consider splitting into two mutually-recursive types (one with only terminals, one with only nonterminals)
// TODO: extend with blockchain-specific constructors with color
pub enum Val<'a> {
    // terminals
    Raw(String),
    Hash(types::HeaderHash), // XXX: consider naming this with a more specific meaning, as we'll probably have other hashes?
    //// numbers
    Epoch(u32),
    SlotId(u32),
    //// signatures
    BlockSig(normal::BlockSignature),
    Signature(redeem::Signature),
    //// actor ids
    XPub(hdwallet::XPub),
    Stakeholder(address::StakeholderId),

    // recursive
    List(Vec<Val<'a>>),
    Tree(AST<'a>),
}

fn from_debug<'a>(d: impl fmt::Debug) -> Val<'a> {
    Val::Raw(format!("TODO {:?}", d))
}

fn from_display<'a>(d: impl fmt::Display) -> Val<'a> {
    Val::Raw(format!("{}", d))
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
        Val::Raw(_)
        | Val::Hash(_)
        | Val::Epoch(_)
        | Val::SlotId(_)
        | Val::BlockSig(_)
        | Val::Signature(_)
        | Val::XPub(_)
        | Val::Stakeholder(_) => {
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
        //// numbers get colors for meanings
        Val::Epoch(epoch) => write!(f, "{}", Colour::Blue.paint(format!("{}", epoch))),
        Val::SlotId(slotid) => write!(f, "{}", Colour::Purple.paint(format!("{}", slotid))),
        //// signatures are cyan
        Val::BlockSig(blksig) => write!(f, "{}", Colour::Cyan.paint(format!("{:?}", blksig))),
        Val::Signature(sig) => write!(f, "{}", Colour::Cyan.paint(format!("{:?}", sig))),
        //// actor ids are yellow
        Val::XPub(pubkey) => write!(f, "{}", Colour::Yellow.paint(format!("{}", pubkey))),
        Val::Stakeholder(stkhodl) => write!(f, "{}", Colour::Yellow.paint(format!("{}", stkhodl))),

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

impl<'a> fmt::Display for Val<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt_pretty(self, f, DISPLAY_INDENT_SIZE, DISPLAY_INDENT_LEVEL)
    }
}

// the rest of the file is `impl` and `test`

// XXX: eventually there should be no uses of this
impl Pretty for cbor::Value {
    fn to_pretty(&self) -> Val {
        from_debug(self)
    }
}

impl Pretty for Block {
    fn to_pretty(&self) -> Val {
        match self {
            Block::GenesisBlock(b) => Val::Tree(vec![("GenesisBlock", b.to_pretty())]),
            Block::MainBlock(b) => Val::Tree(vec![("MainBlock", b.to_pretty())]),
        }
    }
}

impl Pretty for normal::Block {
    fn to_pretty(&self) -> Val {
        Val::Tree(vec![
            ("header", self.header.to_pretty()),
            ("body", self.body.to_pretty()),
            ("extra", self.extra.to_pretty()),
        ])
    }
}

impl Pretty for normal::BlockHeader {
    fn to_pretty(&self) -> Val {
        Val::Tree(vec![
            ("protocol magic", self.protocol_magic.to_pretty()),
            ("previous hash", self.previous_header.to_pretty()),
            ("body proof", self.body_proof.to_pretty()),
            ("consensus", self.consensus.to_pretty()),
            ("extra data", self.extra_data.to_pretty()),
        ])
    }
}

// TODO: do Val::Tree because this is a struct w/fields
impl Pretty for types::HeaderExtraData {
    fn to_pretty(&self) -> Val {
        from_debug(self)
    }
}

// XXX: consider moving this instance into config.rs so it can use the number directly?
impl Pretty for config::ProtocolMagic {
    fn to_pretty(&self) -> Val {
        from_display(self)
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
            ("protocol magic", self.protocol_magic.to_pretty()),
            ("previous hash", self.previous_header.to_pretty()),
            ("body proof", self.body_proof.to_pretty()),
            ("consensus", self.consensus.to_pretty()),
            ("extra data", self.extra_data.to_pretty()),
        ])
    }
}

// XXX: struct is still bare cbor
impl Pretty for types::BlockHeaderAttributes {
    fn to_pretty(&self) -> Val {
        from_debug(self)
    }
}

// XXX: consider moving this instance into genesis.rs so it can use the hash directly?
impl Pretty for genesis::BodyProof {
    fn to_pretty(&self) -> Val {
        from_debug(self)
    }
}

impl Pretty for normal::BodyProof {
    fn to_pretty(&self) -> Val {
        Val::Tree(vec![
            ("tx proof", self.tx.to_pretty()),
            ("mpc", self.mpc.to_pretty()),
            ("proxy sk", self.proxy_sk.to_pretty()),
            ("update", self.update.to_pretty()),
        ])
    }
}

impl Pretty for tx::TxProof {
    fn to_pretty(&self) -> Val {
        Val::Tree(vec![
            (
                "number",
                from_display(self.number),
                // TODO: add a Val::U32 constructor for this and other bare u32
            ),
            ("root", self.root.to_pretty()),
            ("witness hash", self.witnesses_hash.to_pretty()),
        ])
    }
}

// XXX: unify with the instance for HeaderHash?
impl Pretty for hash::Blake2b256 {
    fn to_pretty(&self) -> Val {
        from_display(self)
    }
}

// SscProof is an enum over hashes. This instance is fine.
impl Pretty for SscProof {
    fn to_pretty(&self) -> Val {
        from_debug(self)
    }
}

impl Pretty for normal::Consensus {
    fn to_pretty(&self) -> Val {
        Val::Tree(vec![
            ("slot", self.slot_id.to_pretty()),
            ("leader key", self.leader_key.to_pretty()),
            ("chain difficulty", self.chain_difficulty.to_pretty()),
            ("block signature", self.block_signature.to_pretty()),
        ])
    }
}

// XXX: consider moving this instance into types.rs so it can use the number directly?
impl Pretty for types::ChainDifficulty {
    fn to_pretty(&self) -> Val {
        from_display(self)
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
            ("epoch", self.epoch.to_pretty()),
            ("slot id", Val::SlotId(self.slotid)),
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
            ("epoch", self.epoch.to_pretty()),
            ("chain difficulty", self.chain_difficulty.to_pretty()),
        ])
    }
}

impl Pretty for normal::Body {
    fn to_pretty(&self) -> Val {
        Val::Tree(vec![
            ("tx payload", self.tx.to_pretty()),
            ("ssc", self.ssc.to_pretty()),
            ("delegation", self.delegation.to_pretty()),
            ("update", self.update.to_pretty()),
        ])
    }
}

impl Pretty for normal::SscPayload {
    fn to_pretty(&self) -> Val {
        match self {
            normal::SscPayload::CommitmentsPayload(m, vss) => Val::Tree(vec![
                ("commitments", m.to_pretty()),
                ("vss certificatates", vss.to_pretty()),
            ]),
            normal::SscPayload::OpeningsPayload(m, vss) => Val::Tree(vec![
                ("openings", m.to_pretty()),
                ("vss certificatates", vss.to_pretty()),
            ]),
            normal::SscPayload::SharesPayload(m, vss) => Val::Tree(vec![
                ("shares", m.to_pretty()),
                ("vss certificatates", vss.to_pretty()),
            ]),
            normal::SscPayload::CertificatesPayload(vss) => {
                Val::Tree(vec![("vss certificatates", vss.to_pretty())])
            }
        }
    }
}

impl Pretty for normal::VssCertificates {
    fn to_pretty(&self) -> Val {
        Val::List(self.iter().map(|cert| cert.to_pretty()).collect())
    }
}

impl Pretty for normal::VssCertificate {
    fn to_pretty(&self) -> Val {
        Val::Tree(vec![
            ("vss key", self.vss_key.to_pretty()),
            ("expiry epoch", self.expiry_epoch.to_pretty()),
            ("signature", self.signature.to_pretty()),
            ("signing key", self.signing_key.to_pretty()),
        ])
    }
}

impl Pretty for redeem::Signature {
    fn to_pretty(&self) -> Val {
        Val::Signature(self.clone())
    }
}

impl Pretty for hdwallet::XPub {
    fn to_pretty(&self) -> Val {
        Val::XPub(self.clone())
    }
}

impl Pretty for genesis::Body {
    fn to_pretty(&self) -> Val {
        Val::List(
            self.slot_leaders
                .iter()
                .map(|stakeholder| stakeholder.to_pretty())
                .collect(),
        )
    }
}

impl Pretty for address::StakeholderId {
    fn to_pretty(&self) -> Val {
        Val::Stakeholder(*self)
    }
}

impl Pretty for normal::TxPayload {
    fn to_pretty(&self) -> Val {
        Val::List(
            self.iter()
                .map(|txaux| {
                    Val::Tree(vec![
                        ("tx", txaux.tx.to_pretty()),
                        ("witnesses", txaux.witnesses.to_pretty()),
                    ])
                })
                .collect(),
        )
    }
}

// XXX: impl for a parameterized generic type, Vec<..> not sure if idiomatic
impl Pretty for Vec<tx::TxInWitness> {
    fn to_pretty(&self) -> Val {
        Val::List(self.iter().map(from_display).collect())
    }
}

impl Pretty for tx::Tx {
    fn to_pretty(&self) -> Val {
        Val::Tree(vec![
            (
                "inputs",
                Val::List(self.inputs.iter().map(from_display).collect()),
            ),
            (
                "outputs",
                Val::List(self.outputs.iter().map(from_display).collect()),
            ),
        ])
    }
}

impl Pretty for genesis::Block {
    fn to_pretty(&self) -> Val {
        Val::Tree(vec![
            ("header", self.header.to_pretty()),
            ("body", self.body.to_pretty()),
            ("extra", self.extra.to_pretty()),
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
        let input = vec![("name", Raw("zaphod")), ("age", Raw(format!("{}", 42)))];
        assert_eq!(longest_key_length(&input), 4);
    }
    #[test]
    fn test_display_flat_pairs() {
        let input = Tree(vec![
            ("name", Raw("zaphod")),
            ("age", Raw(format!("{}", 42))),
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
                "character",
                Tree(vec![
                    ("name", Raw("zaphod")),
                    ("age", Raw(format!("{}", 42))),
                ]),
            ),
            ("crook", Raw("yes")),
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
                "character",
                Tree(vec![
                    ("name", Raw("zaphod")),
                    ("age", Raw(format!("{}", 42))),
                ]),
            ),
            ("crook", Raw("yes")),
            (
                "facts",
                List(vec![
                    Raw("invented pan-galactic gargle blaster"),
                    Raw("elected president"),
                    Tree(vec![
                        ("heads", Raw(format!("{}", 2))),
                        ("arms", Raw(format!("{}", 3))),
                    ]),
                    List(vec![Raw("stole the heart of gold"), Raw("one hoopy frood")]),
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
