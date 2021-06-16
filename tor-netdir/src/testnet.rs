//! Support for unit tests, in this crate and elsewhere.
//!
//! This module is only enabled when the `testing` feature is enabled.
//!
//! It is not covered by semver for the `tor-netdir` crate: see notes
//! on [`construct_network`].

use super::*;
use hex_literal::hex;
use std::time::{Duration, SystemTime};
use tor_llcrypto::pk::rsa;
use tor_netdoc::doc::netstatus::{Lifetime, RelayFlags, RelayWeight};

/// Helper: make a dummy 1024-bit RSA public key.
///
/// (I forget where I got this key, so you probably shouldn't encrypt any
/// secrets to it.
fn rsa_example() -> rsa::PublicKey {
    let der = hex!("30818902818100d527b6c63d6e81d39c328a94ce157dccdc044eb1ad8c210c9c9e22487b4cfade6d4041bd10469a657e3d82bc00cf62ac3b6a99247e573b54c10c47f5dc849b0accda031eca6f6e5dc85677f76dec49ff24d2fcb2b5887fb125aa204744119bb6417f45ee696f8dfc1c2fc21b2bae8e9e37a19dc2518a2c24e7d8fd7fac0f46950203010001");
    rsa::PublicKey::from_der(&der).unwrap()
}

/// As [`construct_network()`], but return a [`NetDir`].
pub fn construct_netdir() -> NetDir {
    let (consensus, microdescs) = construct_network();
    let mut dir = PartialNetDir::new(consensus, None);
    for md in microdescs {
        dir.add_microdesc(md);
    }

    dir.unwrap_if_sufficient().unwrap()
}

/// Build a fake network with enough information to enable some basic
/// tests.
///
/// The constructed network will contain 40 relays, numbered 0 through
/// 39. They will have with RSA and Ed25519 identity fingerprints set to
/// 0x0000...00 through 0x2727...27.  Each pair of relays is in a
/// family with one another: 0x00..00 with 0x01..01, and so on.
///
/// All relays are marked as usable.  The first ten are marked with no
/// additional flags.  The next ten are marked with the exit flag.
/// The next ten are marked with the guard flag.  The last ten are
/// marked with the exit _and_ guard flags.
///
/// TAP and Ntor onion keys are present, but unusable.
///
/// Odd-numbered exit relays are set to allow ports 80 and 443 on
/// IPv4.  Even-numbered exit relays are set to allow ports 1-65535
/// on IPv4.  No exit relays are marked to support IPv6.
///
/// Even-numbered relays support the `DirCache=2` protocol.
///
/// Every relay is given a measured weight based on its position
/// within its group of ten.  The weights for the ten relays in each
/// group are: 100, 200, 300, ... 10000.  There is no additional
/// flag-based bandwidth weighting.
///
/// The consensus is declared as using method 34, and as being valid for
/// one day (in realtime) after the current `SystemTime`.
///
/// # Notes for future expansion
///
/// _Resist the temptation to make unconditional changes to this
/// function._ If the network generated by this function gets more and
/// more complex, then it will become harder and harder over time to
/// make it support new test cases and new behavior, and eventually
/// we'll have to throw the whole thing away.  (We ran into this
/// problem with Tor's unit tests.)
///
/// Instead, I'd suggest refactoring this function so that it takes a
/// description of what kind of network to build, and then builds it from
/// that description.
pub fn construct_network() -> (MdConsensus, Vec<Microdesc>) {
    let f = RelayFlags::RUNNING | RelayFlags::VALID | RelayFlags::V2DIR;
    // define 4 groups of flags
    let flags = [
        f,
        f | RelayFlags::EXIT,
        f | RelayFlags::GUARD,
        f | RelayFlags::EXIT | RelayFlags::GUARD,
    ];

    let now = SystemTime::now();
    let one_day = Duration::new(86400, 0);
    let mut bld = MdConsensus::builder();
    bld.consensus_method(34)
        .lifetime(Lifetime::new(now, now + one_day / 2, now + one_day).unwrap())
        .param("bwweightscale", 1)
        .weights("".parse().unwrap());

    let mut microdescs = Vec::new();
    for idx in 0..40_u8 {
        // Each relay gets a couple of no-good onion keys.
        // Its identity fingerprints are set to `idx`, repeating.
        // They all get the same address.
        let flags = flags[(idx / 10) as usize];
        let policy = if flags.contains(RelayFlags::EXIT) {
            if idx % 2 == 1 {
                "accept 80,443"
            } else {
                "accept 1-65535"
            }
        } else {
            "reject 1-65535"
        };
        // everybody is family with the adjacent relay.
        let fam_id = [idx ^ 1; 20];
        let family = hex::encode(&fam_id);

        let md = Microdesc::builder()
            .tap_key(rsa_example())
            .ntor_key((*b"----nothing in dirmgr uses this-").into())
            .ed25519_id([idx; 32].into())
            .family(family.parse().unwrap())
            .parse_ipv4_policy(policy)
            .unwrap()
            .testing_md()
            .unwrap();
        let protocols = if idx % 2 == 0 {
            // even-numbered relays are dircaches.
            "DirCache=2".parse().unwrap()
        } else {
            "".parse().unwrap()
        };
        let weight = RelayWeight::Measured(1000 * (idx % 10 + 1) as u32);
        bld.rs()
            .identity([idx; 20].into())
            .add_or_port("127.0.0.1:9001".parse().unwrap())
            .doc_digest(*md.digest())
            .protos(protocols)
            .set_flags(flags)
            .weight(weight)
            .build_into(&mut bld)
            .unwrap();
        microdescs.push(md);
    }

    let consensus = bld.testing_consensus().unwrap();

    (consensus, microdescs)
}
