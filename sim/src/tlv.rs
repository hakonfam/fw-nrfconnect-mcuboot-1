//! TLV Support
//!
//! mcuboot images are followed immediately by a list of TLV items that contain integrity
//! information about the image.  Their generation is made a little complicated because the size of
//! the TLV block is in the image header, which is included in the hash.  Since some signatures can
//! vary in size, we just make them the largest size possible.
//!
//! Because of this header, we have to make two passes.  The first pass will compute the size of
//! the TLV, and the second pass will build the data for the TLV.

use pem;
use base64;
use ring::{digest, rand};
use ring::signature::{
    RsaKeyPair,
    RSA_PSS_SHA256,
    EcdsaKeyPair,
    ECDSA_P256_SHA256_ASN1_SIGNING,
};
use untrusted;
use mcuboot_sys::c;

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq)]
#[allow(dead_code)] // TODO: For now
pub enum TlvKinds {
    KEYHASH = 0x01,
    SHA256 = 0x10,
    RSA2048 = 0x20,
    ECDSA224 = 0x21,
    ECDSA256 = 0x22,
    ENCRSA2048 = 0x30,
    ENCKW128 = 0x31,
}

#[allow(dead_code, non_camel_case_types)]
pub enum TlvFlags {
    PIC = 0x01,
    NON_BOOTABLE = 0x02,
    ENCRYPTED = 0x04,
    RAM_LOAD = 0x20,
}

pub struct TlvGen {
    flags: u32,
    kinds: Vec<TlvKinds>,
    size: u16,
    payload: Vec<u8>,
}

pub const AES_SEC_KEY: &[u8; 16] = b"0123456789ABCDEF";

impl TlvGen {
    /// Construct a new tlv generator that will only contain a hash of the data.
    #[allow(dead_code)]
    pub fn new_hash_only() -> TlvGen {
        TlvGen {
            flags: 0,
            kinds: vec![TlvKinds::SHA256],
            size: 4 + 32,
            payload: vec![],
        }
    }

    #[allow(dead_code)]
    pub fn new_rsa_pss() -> TlvGen {
        TlvGen {
            flags: 0,
            kinds: vec![TlvKinds::SHA256, TlvKinds::RSA2048],
            size: 4 + 32 + 4 + 32 + 4 + 256,
            payload: vec![],
        }
    }

    #[allow(dead_code)]
    pub fn new_ecdsa() -> TlvGen {
        TlvGen {
            flags: 0,
            kinds: vec![TlvKinds::SHA256, TlvKinds::ECDSA256],
            size: 4 + 32 + 4 + 32 + 4 + 72,
            payload: vec![],
        }
    }

    #[allow(dead_code)]
    pub fn new_enc_rsa() -> TlvGen {
        TlvGen {
            flags: TlvFlags::ENCRYPTED as u32,
            kinds: vec![TlvKinds::SHA256, TlvKinds::ENCRSA2048],
            size: 4 + 32 + 4 + 256,
            payload: vec![],
        }
    }

    #[allow(dead_code)]
    pub fn new_sig_enc_rsa() -> TlvGen {
        TlvGen {
            flags: TlvFlags::ENCRYPTED as u32,
            kinds: vec![TlvKinds::SHA256, TlvKinds::RSA2048, TlvKinds::ENCRSA2048],
            size: 4 + 32 + 4 + 32 + 4 + 256 + 4 + 256,
            payload: vec![],
        }
    }

    #[allow(dead_code)]
    pub fn new_enc_kw() -> TlvGen {
        TlvGen {
            flags: TlvFlags::ENCRYPTED as u32,
            kinds: vec![TlvKinds::SHA256, TlvKinds::ENCKW128],
            size: 4 + 32 + 4 + 24,
            payload: vec![],
        }
    }

    #[allow(dead_code)]
    pub fn new_rsa_kw() -> TlvGen {
        TlvGen {
            flags: TlvFlags::ENCRYPTED as u32,
            kinds: vec![TlvKinds::SHA256, TlvKinds::RSA2048, TlvKinds::ENCKW128],
            size: 4 + 32 + 4 + 32 + 4 + 256 + 4 + 24,
            payload: vec![],
        }
    }

    #[allow(dead_code)]
    pub fn new_ecdsa_kw() -> TlvGen {
        TlvGen {
            flags: TlvFlags::ENCRYPTED as u32,
            kinds: vec![TlvKinds::SHA256, TlvKinds::ECDSA256, TlvKinds::ENCKW128],
            size: 4 + 32 + 4 + 32 + 4 + 72 + 4 + 24,
            payload: vec![],
        }
    }

    /// Retrieve the header flags for this configuration.  This can be called at any time.
    pub fn get_flags(&self) -> u32 {
        self.flags
    }

    /// Retrieve the size that the TLV will occupy.  This can be called at any time.
    pub fn get_size(&self) -> u16 {
        4 + self.size
    }

    /// Add bytes to the covered hash.
    pub fn add_bytes(&mut self, bytes: &[u8]) {
        self.payload.extend_from_slice(bytes);
    }

    /// Compute the TLV given the specified block of data.
    pub fn make_tlv(self) -> Vec<u8> {
        let mut result: Vec<u8> = vec![];

        let size = self.get_size();
        result.push(0x07);
        result.push(0x69);
        result.push((size & 0xFF) as u8);
        result.push(((size >> 8) & 0xFF) as u8);

        if self.kinds.contains(&TlvKinds::SHA256) {
            let hash = digest::digest(&digest::SHA256, &self.payload);
            let hash = hash.as_ref();

            assert!(hash.len() == 32);
            result.push(TlvKinds::SHA256 as u8);
            result.push(0);
            result.push(32);
            result.push(0);
            result.extend_from_slice(hash);
        }

        if self.kinds.contains(&TlvKinds::RSA2048) {
            // Output the hash of the public key.
            let hash = digest::digest(&digest::SHA256, RSA_PUB_KEY);
            let hash = hash.as_ref();

            assert!(hash.len() == 32);
            result.push(TlvKinds::KEYHASH as u8);
            result.push(0);
            result.push(32);
            result.push(0);
            result.extend_from_slice(hash);

            // For now assume PSS.
            let key_bytes = pem::parse(include_bytes!("../../root-rsa-2048.pem").as_ref()).unwrap();
            assert_eq!(key_bytes.tag, "RSA PRIVATE KEY");
            let key_bytes = untrusted::Input::from(&key_bytes.contents);
            let key_pair = RsaKeyPair::from_der(key_bytes).unwrap();
            let rng = rand::SystemRandom::new();
            let mut signature = vec![0; key_pair.public_modulus_len()];
            assert_eq!(signature.len(), 256);
            key_pair.sign(&RSA_PSS_SHA256, &rng, &self.payload, &mut signature).unwrap();

            result.push(TlvKinds::RSA2048 as u8);
            result.push(0);
            result.push((signature.len() & 0xFF) as u8);
            result.push(((signature.len() >> 8) & 0xFF) as u8);
            result.extend_from_slice(&signature);
        }

        if self.kinds.contains(&TlvKinds::ECDSA256) {
            let keyhash = digest::digest(&digest::SHA256, ECDSA256_PUB_KEY);
            let keyhash = keyhash.as_ref();

            assert!(keyhash.len() == 32);
            result.push(TlvKinds::KEYHASH as u8);
            result.push(0);
            result.push(32);
            result.push(0);
            result.extend_from_slice(keyhash);

            let key_bytes = pem::parse(include_bytes!("../../root-ec-p256-pkcs8.pem").as_ref()).unwrap();
            assert_eq!(key_bytes.tag, "PRIVATE KEY");

            let key_bytes = untrusted::Input::from(&key_bytes.contents);
            let key_pair = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING,
                                                    key_bytes).unwrap();
            let rng = rand::SystemRandom::new();
            let payload = untrusted::Input::from(&self.payload);
            let signature = key_pair.sign(&rng, payload).unwrap();

            result.push(TlvKinds::ECDSA256 as u8);
            result.push(0);

            // signature must be padded...
            let mut signature = signature.as_ref().to_vec();
            while signature.len() < 72 {
                signature.push(0);
                signature[1] += 1;
            }

            result.push((signature.len() & 0xFF) as u8);
            result.push(((signature.len() >> 8) & 0xFF) as u8);
            result.extend_from_slice(signature.as_ref());
        }

        if self.kinds.contains(&TlvKinds::ENCRSA2048) {
            let key_bytes = pem::parse(include_bytes!("../../enc-rsa2048-pub.pem")
                                       .as_ref()).unwrap();
            assert_eq!(key_bytes.tag, "PUBLIC KEY");

            let encbuf = match c::rsa_oaep_encrypt(&key_bytes.contents, AES_SEC_KEY) {
                Ok(v) => v,
                Err(_) => panic!("Failed to encrypt secret key"),
            };

            assert!(encbuf.len() == 256);
            result.push(TlvKinds::ENCRSA2048 as u8);
            result.push(0);
            result.push(0);
            result.push(1);
            result.extend_from_slice(&encbuf);
        }

        if self.kinds.contains(&TlvKinds::ENCKW128) {
            let key_bytes = base64::decode(
                include_str!("../../enc-aes128kw.b64").trim()).unwrap();

            let encbuf = match c::kw_encrypt(&key_bytes, AES_SEC_KEY) {
                Ok(v) => v,
                Err(_) => panic!("Failed to encrypt secret key"),
            };

            assert!(encbuf.len() == 24);
            result.push(TlvKinds::ENCKW128 as u8);
            result.push(0);
            result.push(24);
            result.push(0);
            result.extend_from_slice(&encbuf);
        }

        result
    }
}

include!("rsa_pub_key-rs.txt");
include!("ecdsa_pub_key-rs.txt");
