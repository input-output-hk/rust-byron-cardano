use std::{io::{Write, Result}};

use cardano::block::{genesis, normal, types, Block};
use cardano::{address, tx};

use super::term::style::{Style, StyledObject};

// Constants for the fmt::Display instance
static DISPLAY_INDENT_SIZE: usize = 4; // spaces

pub trait Pretty {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write;
}

fn pretty_attribute<P: Pretty, W: Write>(w: &mut W, indent: usize, k: &'static str, v: P) -> Result<()> {
    write!(w, "{:width$}{}: ", "", k, width = indent)?;
    v.pretty(w, indent + DISPLAY_INDENT_SIZE)?;
    writeln!(w, "")?;
    Ok(())
}
fn pretty_object<P: Pretty, W: Write>(w: &mut W, indent: usize, k: &'static str, v: P) -> Result<()> {
    writeln!(w, "{:width$}{}:", "", k, width = indent)?;
    v.pretty(w, indent + DISPLAY_INDENT_SIZE)?;
    writeln!(w, "")?;
    Ok(())
}

impl<'a> Pretty for &'a str {
    fn pretty<W>(self, f: &mut W, _: usize) -> Result<()>
        where W: Write
    {
        write!(f, "{}", self)
    }
}

impl<D: ::std::fmt::Display> Pretty for StyledObject<D> {
    fn pretty<W>(self, f: &mut W, _: usize) -> Result<()>
        where W: Write
    {
        write!(f, "{}", self)
    }
}

impl<'a, D: ::std::fmt::Display> Pretty for &'a StyledObject<D> {
    fn pretty<W>(self, f: &mut W, _: usize) -> Result<()>
        where W: Write
    {
        write!(f, "{}", self)
    }
}

fn pretty_iterator<I, D, W>(w: &mut W, indent: usize, iter: I) -> Result<()>
    where I: IntoIterator<Item = D>
        , D: Pretty
        , W: Write
{
    for e in iter {
        write!(w, "{:width$}", "", width = indent)?;
        e.pretty(w, indent + DISPLAY_INDENT_SIZE)?;
        writeln!(w, "")?;
    }
    Ok(())
}

impl<D: Pretty> Pretty for Vec<D> {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_iterator(f, indent, self.into_iter())
    }
}

impl Pretty for Block {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        match self {
            Block::GenesisBlock(blk) => blk.pretty(f, indent),
            Block::MainBlock(blk) => blk.pretty(f, indent),
        }
    }
}
impl Pretty for genesis::Block {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_object(f, indent, "header", self.header)?;
        pretty_object(f, indent, "body", self.body)?;
        // TODO: extra?
        Ok(())
    }
}
impl Pretty for normal::Block {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_object(f, indent, "header", self.header)?;
        pretty_object(f, indent, "body", self.body)?;
        // TODO: extra?
        Ok(())
    }
}
impl Pretty for genesis::Body {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        // pretty_attribute(f, indent, "ssc", self.ssc)?;
        pretty_object(f, indent, "slot_leaders", self.slot_leaders)?;
        // TODO: delegation?
        // TODO: update?
        Ok(())
    }
}
impl Pretty for normal::Body {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        // pretty_attribute(f, indent, "ssc", self.ssc)?;
        self.tx.pretty(f, indent)?;
        // TODO: delegation?
        // TODO: update?
        Ok(())
    }
}
impl Pretty for normal::TxPayload {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_iterator(f, indent, self.into_iter())
    }
}
impl Pretty for tx::TxAux {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_object(f, indent, "tx", self.tx)?;
        pretty_object(f, indent, "witnesses", self.witness.in_witnesses)?;
        Ok(())
    }
}
impl Pretty for tx::Tx {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_object(f, indent, "inputs", self.inputs)?;
        pretty_object(f, indent, "outputs", self.outputs)?;
        Ok(())
    }
}
impl Pretty for tx::TxIn {
    fn pretty<W>(self, f: &mut W, _: usize) -> Result<()>
        where W: Write
    {
        write!(f, "{}@{}", style!(self.id), style!(self.index).yellow())
    }
}
impl Pretty for tx::TxOut {
    fn pretty<W>(self, f: &mut W, _: usize) -> Result<()>
        where W: Write
    {
        write!(f, "{} {}", style!(self.address), style!(self.value))
    }
}
impl Pretty for tx::TxInWitness {
    fn pretty<W>(self, f: &mut W, _: usize) -> Result<()>
        where W: Write
    {
        match self {
            tx::TxInWitness::PkWitness(xpub, signature) => {
                write!(f, "{} {} ({})", style!(xpub), style!(signature), style!("Public Key"))
            },
            tx::TxInWitness::ScriptWitness(_, _) => {
                write!(f, "({})", style!("Script"))
            },
            tx::TxInWitness::RedeemWitness(public, signature) => {
                write!(f, "{} {} ({})", style!(public), style!(signature), style!("Redeem"))
            },
        }
    }
}
impl Pretty for address::StakeholderId {
    fn pretty<W>(self, f: &mut W, _: usize) -> Result<()>
        where W: Write
    {
        write!(f, "{}", style!(self))
    }
}

impl Pretty for genesis::BlockHeader {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_attribute(f, indent, "protocol_magic", style!(self.protocol_magic))?;
        pretty_attribute(f, indent, "previous_header", style!(self.previous_header))?;
        pretty_attribute(f, indent, "body_proof", style!(self.body_proof))?;
        pretty_object(f, indent, "consensus", self.consensus)?;
        // pretty_attribute(f, indent, "extra_data", self.extra_data)?;
        Ok(())
    }
}
impl Pretty for normal::BlockHeader {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_attribute(f, indent, "protocol_magic", style!(self.protocol_magic))?;
        pretty_attribute(f, indent, "previous_header", style!(self.previous_header))?;
        pretty_object(f, indent, "body_proof", self.body_proof)?;
        pretty_object(f, indent, "consensus", self.consensus)?;
        Ok(())
    }
}

impl Pretty for genesis::Consensus {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_attribute(f, indent, "epochid", style!(self.epoch).red().bold())?;
        pretty_attribute(f, indent, "chain_difficulty", style!(self.chain_difficulty))?;
        Ok(())
    }
}
impl Pretty for normal::Consensus {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_attribute(f, indent, "slotid", style!(self.slot_id))?;
        pretty_attribute(f, indent, "leader_key", style!(self.leader_key))?;
        pretty_attribute(f, indent, "chain_difficulty", style!(self.chain_difficulty))?;
        match self.block_signature {
            normal::BlockSignature::Signature(blk) => {
                pretty_attribute(f, indent, "block_signature", style!(blk))?;
            },
            _ => {
                // TODO
            }
        }

        Ok(())
    }
}

impl Pretty for normal::BodyProof {
    fn pretty<W>(self, f: &mut W, indent: usize) -> Result<()>
        where W: Write
    {
        pretty_attribute(f, indent, "tx", self.tx)?;
        pretty_attribute(f, indent, "mpc", self.mpc)?;
        pretty_attribute(f, indent, "proxy_sk", style!(self.proxy_sk))?;
        pretty_attribute(f, indent, "update", style!(self.update))?;

        Ok(())
    }
}
impl Pretty for tx::TxProof {
    fn pretty<W>(self, f: &mut W, _: usize) -> Result<()>
        where W: Write
    {
        writeln!(f, "{} {} {}", style!(self.number), style!(self.root), style!(self.witnesses_hash))
    }
}
impl Pretty for types::SscProof {
    fn pretty<W>(self, f: &mut W, _: usize) -> Result<()>
        where W: Write
    {
        match self {
            types::SscProof::Commitments(h1, h2) => {
                write!(f, "{} {} ({})", style!(h1), style!(h2), style!("Commitments"))
            },
            types::SscProof::Openings(h1, h2) => {
                write!(f, "{} {} ({})", style!(h1), style!(h2), style!("Openings"))
            },
            types::SscProof::Shares(h1, h2) => {
                write!(f, "{} {} ({})", style!(h1), style!(h2), style!("Shares"))
            },
            types::SscProof::Certificate(h1) => {
                write!(f, "{} ({})", style!(h1), style!("Certificate"))
            }
        }
    }
}
