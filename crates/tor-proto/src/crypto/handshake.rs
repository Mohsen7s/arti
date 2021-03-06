//! Circuit extension handshake for Tor.
//!
//! Tor circuit handshakes all implement a one-way-authenticated key
//! exchange, where a client that knows a public "onion key" for a
//! relay sends a "client onionskin" to extend to a relay, and receives a
//! "relay onionskin" in response.  When the handshake is successful,
//! both the client and relay share a set of session keys, and the
//! client knows that nobody _else_ shares those keys unless they
//! relay's private onion key.
//!
//! Currently, this module implements only the "ntor" handshake used
//! for circuits on today's Tor.
pub(crate) mod fast;
#[cfg(feature = "hs")]
pub(crate) mod hs_ntor;
pub(crate) mod ntor;
#[cfg(feature = "ntor_v3")]
pub(crate) mod ntor_v3;

use crate::{Result, SecretBytes};
//use zeroize::Zeroizing;
use rand_core::{CryptoRng, RngCore};

/// A ClientHandshake is used to generate a client onionskin and
/// handle a relay onionskin.
pub(crate) trait ClientHandshake {
    /// The type for the onion key.
    type KeyType;
    /// The type for the state that the client holds while waiting for a reply.
    type StateType;
    /// A type that is returned and used to generate session keys.x
    type KeyGen;
    /// Generate a new client onionskin for a relay with a given onion key.
    ///
    /// On success, return a state object that will be used to
    /// complete the handshake, along with the message to send.
    fn client1<R: RngCore + CryptoRng>(
        rng: &mut R,
        key: &Self::KeyType,
    ) -> Result<(Self::StateType, Vec<u8>)>;
    /// Handle an onionskin from a relay, and produce a key generator.
    ///
    /// The state object must match the one that was used to make the
    /// client onionskin that the server is replying to.
    fn client2<T: AsRef<[u8]>>(state: Self::StateType, msg: T) -> Result<Self::KeyGen>;
}

/// A ServerHandshake is used to handle a client onionskin and generate a
/// server onionskin.
pub(crate) trait ServerHandshake {
    /// The type for the onion key.  This is a private key type.
    type KeyType;
    /// The returned key generator type.
    type KeyGen;

    /// Perform the server handshake.  Take as input a strong PRNG in `rng`,
    /// a slice of all our private onion keys, and the client's message.
    ///
    /// On success, return a key generator and a server handshake message
    /// to send in reply.
    fn server<R: RngCore + CryptoRng, T: AsRef<[u8]>>(
        rng: &mut R,
        key: &[Self::KeyType],
        msg: T,
    ) -> Result<(Self::KeyGen, Vec<u8>)>;
}

/// A KeyGenerator is returned by a handshake, and used to generate
/// session keys for the protocol.
///
/// Typically, it wraps a KDF function, and some seed key material.
///
/// It can only be used once.
pub(crate) trait KeyGenerator {
    /// Consume the key
    fn expand(self, keylen: usize) -> Result<SecretBytes>;
}

/// Generates keys based on the KDF-TOR function.
///
/// This is deprecated and shouldn't be used for new keys.
pub(crate) struct TapKeyGenerator {
    /// Seed for the TAP KDF.
    seed: SecretBytes,
}

impl TapKeyGenerator {
    /// Create a key generator based on a provided seed
    pub(crate) fn new(seed: SecretBytes) -> Self {
        TapKeyGenerator { seed }
    }
}

impl KeyGenerator for TapKeyGenerator {
    fn expand(self, keylen: usize) -> Result<SecretBytes> {
        use crate::crypto::ll::kdf::{Kdf, LegacyKdf};
        LegacyKdf::new(1).derive(&self.seed[..], keylen)
    }
}

/// Generates keys based on SHAKE-256.
pub(crate) struct ShakeKeyGenerator {
    /// Seed for the key generator
    seed: SecretBytes,
}

impl ShakeKeyGenerator {
    /// Create a key generator based on a provided seed
    #[allow(dead_code)] // We'll construct these for v3 onion services
    pub(crate) fn new(seed: SecretBytes) -> Self {
        ShakeKeyGenerator { seed }
    }
}

impl KeyGenerator for ShakeKeyGenerator {
    fn expand(self, keylen: usize) -> Result<SecretBytes> {
        use crate::crypto::ll::kdf::{Kdf, ShakeKdf};
        ShakeKdf::new().derive(&self.seed[..], keylen)
    }
}
