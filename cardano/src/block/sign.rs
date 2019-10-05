use self::normal::BodyProof;
use block::*;
use cbor_event::{
    self,
    de::Deserializer,
    se::{self, Serializer},
};
use config::ProtocolMagic;
use hdwallet;
use std::io::{BufRead, Write};
use tags;

#[derive(Debug, Clone)]
pub struct MainToSign<'a> {
    previous_header: &'a HeaderHash,
    body_proof: &'a BodyProof,
    slot: &'a EpochSlotId,
    chain_difficulty: &'a ChainDifficulty,
    extra_data: &'a HeaderExtraData,
}

impl<'a> cbor_event::se::Serialize for MainToSign<'a> {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        serializer
            .write_array(cbor_event::Len::Len(5))?
            .serialize(&self.previous_header)?
            .serialize(&self.body_proof)?
            .serialize(&self.slot)?
            .serialize(&self.chain_difficulty)?
            .serialize(&self.extra_data)
    }
}

impl<'a> MainToSign<'a> {
    pub fn from_header(hdr: &'a normal::BlockHeader) -> Self {
        MainToSign {
            previous_header: &hdr.previous_header,
            body_proof: &hdr.body_proof,
            slot: &hdr.consensus.slot_id,
            chain_difficulty: &hdr.consensus.chain_difficulty,
            extra_data: &hdr.extra_data,
        }
    }

    pub fn verify_proxy_sig(
        &self,
        protocol_magic: ProtocolMagic,
        tag: tags::SigningTag,
        proxy_sig: &ProxySignature,
    ) -> bool {
        verify_signature_with(protocol_magic, tag, proxy_sig, self)
    }
}

fn verify_signature_with<T>(
    protocol_magic: ProtocolMagic,
    tag: tags::SigningTag,
    proxy_sig: &ProxySignature,
    data: &T,
) -> bool
where
    T: se::Serialize,
{
    let mut buf = vec!['0' as u8, '1' as u8];

    buf.extend(proxy_sig.psk.issuer_pk.as_ref());
    buf.push(tag as u8);

    se::Serializer::new(&mut buf)
        .serialize(&protocol_magic)
        .unwrap()
        .serialize(data)
        .unwrap();

    proxy_sig.psk.delegate_pk.verify(
        &buf,
        &hdwallet::Signature::<()>::from_bytes(*proxy_sig.sig.to_bytes()),
    )
}

type SignData = ();

type ProxyCert = hdwallet::Signature<()>;

#[derive(Debug, Clone)]
pub struct ProxySecretKey {
    pub omega: u64,
    pub issuer_pk: hdwallet::XPub,
    pub delegate_pk: hdwallet::XPub,
    pub cert: ProxyCert,
}

impl cbor_event::se::Serialize for ProxySecretKey {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        serializer
            .write_array(cbor_event::Len::Len(4))?
            .serialize(&self.omega)?
            .serialize(&self.issuer_pk)?
            .serialize(&self.delegate_pk)?
            .serialize(&self.cert)
    }
}

impl cbor_event::de::Deserialize for ProxySecretKey {
    fn deserialize<R: BufRead>(raw: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        raw.tuple(4, "ProxySecretKey")?;

        let omega = cbor_event::de::Deserialize::deserialize(raw)?;
        let issuer_pk = cbor_event::de::Deserialize::deserialize(raw)?;
        let delegate_pk = cbor_event::de::Deserialize::deserialize(raw)?;
        let cert = cbor_event::de::Deserialize::deserialize(raw)?;

        Ok(ProxySecretKey {
            omega,
            issuer_pk,
            delegate_pk,
            cert,
        })
    }
}

impl ProxySecretKey {
    /// Verify that 'cert' is a signature from 'issuer_pk' over
    /// 'delegate_pk' and 'omega'.
    pub fn verify(&self, protocol_magic: ProtocolMagic) -> bool {
        let buf = Self::data_to_sign(&self.delegate_pk, self.omega, protocol_magic);
        self.issuer_pk.verify(&buf, &self.cert)
    }

    /// Use 'issuer_prv' to sign 'delegate_pk' and 'omega' to create a
    /// ProxySecretKey.
    pub fn sign(
        issuer_prv: &hdwallet::XPrv,
        delegate_pk: hdwallet::XPub,
        omega: u64,
        protocol_magic: ProtocolMagic,
    ) -> Self {
        let buf = Self::data_to_sign(&delegate_pk, omega, protocol_magic);

        Self {
            omega,
            issuer_pk: issuer_prv.public(),
            delegate_pk,
            cert: issuer_prv.sign(&buf),
        }
    }

    fn data_to_sign(
        delegate_pk: &hdwallet::XPub,
        omega: u64,
        protocol_magic: ProtocolMagic,
    ) -> Vec<u8> {
        // Yes, this really is
        // CBOR-in-byte-vector-in-CBOR-in-byte-vector...
        let mut buf2 = vec!['0' as u8, '0' as u8];
        buf2.extend(delegate_pk.as_ref());
        se::Serializer::new(&mut buf2).serialize(&omega).unwrap();

        let mut buf = vec![];
        buf.push(tags::SigningTag::ProxySK as u8);
        se::Serializer::new(&mut buf)
            .serialize(&protocol_magic)
            .unwrap()
            .write_bytes(buf2)
            .unwrap();

        buf
    }
}

#[derive(Debug, Clone)]
pub struct ProxySignature {
    pub psk: ProxySecretKey,
    pub sig: hdwallet::Signature<()>,
}

impl cbor_event::se::Serialize for ProxySignature {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        serializer
            .write_array(cbor_event::Len::Len(2))?
            .serialize(&self.psk)?
            .serialize(&self.sig)
    }
}

impl cbor_event::de::Deserialize for ProxySignature {
    fn deserialize<R: BufRead>(raw: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        raw.tuple(2, "ProxySignature")?;

        let psk = cbor_event::de::Deserialize::deserialize(raw)?;
        let sig = cbor_event::de::Deserialize::deserialize(raw)?;

        Ok(ProxySignature { psk, sig })
    }
}

#[derive(Debug, Clone)]
pub enum BlockSignature {
    Signature(hdwallet::Signature<SignData>),
    ProxyLight(Vec<cbor_event::Value>), // TODO: decode
    ProxyHeavy(ProxySignature),
}
impl BlockSignature {
    pub fn to_bytes<'a>(&'a self) -> Option<&'a [u8; hdwallet::SIGNATURE_SIZE]> {
        match self {
            BlockSignature::Signature(s) => Some(s.to_bytes()),
            _ => None,
        }
    }
}
impl cbor_event::se::Serialize for BlockSignature {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        match self {
            &BlockSignature::Signature(ref sig) => serializer
                .write_array(cbor_event::Len::Len(2))?
                .write_unsigned_integer(0)?
                .serialize(sig),
            &BlockSignature::ProxyLight(ref v) => {
                let serializer = serializer
                    .write_array(cbor_event::Len::Len(2))?
                    .write_unsigned_integer(1)?;
                cbor_event::se::serialize_fixed_array(v.iter(), serializer)
            }
            &BlockSignature::ProxyHeavy(ref v) => serializer
                .write_array(cbor_event::Len::Len(2))?
                .write_unsigned_integer(2)?
                .serialize(v),
        }
    }
}
impl cbor_event::de::Deserialize for BlockSignature {
    fn deserialize<R: BufRead>(raw: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        raw.tuple(2, "BlockSignature")?;
        let sum_type_idx = raw.unsigned_integer()?;
        match sum_type_idx {
            0 => Ok(BlockSignature::Signature(raw.deserialize()?)),
            1 => Ok(BlockSignature::ProxyLight(raw.deserialize()?)),
            2 => Ok(BlockSignature::ProxyHeavy(
                cbor_event::de::Deserialize::deserialize(raw)?,
            )),
            _ => Err(cbor_event::Error::CustomError(format!(
                "Unsupported BlockSignature: {}",
                sum_type_idx
            ))),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::sign;
    use base64;
    use hdwallet;
    use std::str::FromStr;

    #[test]
    fn test_psk_verify() {
        let mut psk = sign::ProxySecretKey {
            omega: 0,
            issuer_pk: hdwallet::XPub::from_slice(&base64::decode(&"nFhj99RbuDjG5jU3XjRXlUbP+4LStPeiMh7E7l3oWWfwRqjxXg10jUFt+4pKRlnZTrmI4weBWMGpchDJA9MKnA==").unwrap()).unwrap(),
            delegate_pk: hdwallet::XPub::from_slice(&base64::decode(&"mLujHvc/6KIvUEt2IdnjmVRENEHx9ifl45ZmhZZ8e39+C4fe/HgnKjFtT1M5LjeeSn1Bp8tSAM4WZwL+ECWgsw==").unwrap()).unwrap(),
            cert: hdwallet::Signature::<()>::from_hex(&"fd30c5ac3f77df733eabe48de391ad6727b6ecd7ee72cc85207075a9bba90365f10455b80f3dbf5cc821f71075f00ebdfcffd30b264b5262c1473fd70125ee05").unwrap()
        };

        let pm = 1097911063.into();

        assert!(psk.verify(pm));

        psk.omega = 1;

        assert!(!psk.verify(pm));
    }

    #[test]
    fn test_psk_sign() {
        let pm = 328429219.into();

        let issuer_prv = hdwallet::XPrv::from_str("b8b054ec1b92dd4542db35e2f813f013a8d7ee9f53255b26f3ef3dafb74e11462545bd9c85aa0a6f6719a933eba16909c1a2fa0bbb58e9cd98bf9ddbb79f7d50fcfc22db8155f8d6ca0e3a975cb1b6aa5d6e7609b30c99877e469db06b5d5016").unwrap();
        let delegate_pk = hdwallet::XPub::from_str("695b380fc72ae7d830d46f902a7c9d4057a4b9a7a0be235b87fdf51e698619e033aac8d93fd4cb82785973bb943f2047ddd1e664d4e185e7be634722e108389a").unwrap();
        let expected_cert = hdwallet::Signature::from_hex("a72bf0119afd1ba5bed56b6521544105b6077c884609666296dbc59275477149a1b8230ce5b6c0fa81e1ec61c717164be57422e86a8f2f5773cdc66da99fcc0e").unwrap();

        let psk = sign::ProxySecretKey::sign(&issuer_prv, delegate_pk, 0, pm);

        assert_eq!(psk.cert, expected_cert);

        assert!(psk.verify(pm));
    }
}
