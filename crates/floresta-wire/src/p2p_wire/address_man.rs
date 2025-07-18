//! Address manager is a module that keeps track of known peer addresses and associated
//! metadata. This module is very important in keeping our node protected against targeted
//! attacks, like eclipse attacks.

use std::collections::HashMap;
use std::fs::read_to_string;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use bitcoin::p2p::address::AddrV2;
use bitcoin::p2p::address::AddrV2Message;
use bitcoin::p2p::ServiceFlags;
use bitcoin::Network;
use floresta_chain::DnsSeed;
use floresta_common::service_flags;
use log::debug;
use log::error;
use log::info;
use log::warn;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

/// How long we'll wait before trying to connect to a peer that failed
const RETRY_TIME: u64 = 10 * 60; // 10 minutes

/// A type alias for a list of addresses to send to our peers
type AddressToSend = Vec<(AddrV2, u64, ServiceFlags, u16)>;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum AddressState {
    /// We never tried this peer before, so we don't know what to expect. This variant
    /// also applies to peers that we tried to connect, but failed or we didn't connect
    /// to for a long time.
    NeverTried,
    /// We tried this peer before, and had success at least once, so we know what to expect
    Tried(u64),
    /// This peer misbehaved and we banned them
    Banned(u64),
    /// We are connected to this peer right now
    Connected,
    /// We tried connecting, but failed
    Failed(u64),
}
/// How do we store peers locally
#[derive(Debug, Clone, PartialEq)]
pub struct LocalAddress {
    /// An actual address
    address: AddrV2,
    /// Last time we successfully connected to this peer, only relevant is state == State::Tried
    last_connected: u64,
    /// Our local state for this peer, as defined in AddressState
    state: AddressState,
    /// Network services announced by this peer
    services: ServiceFlags,
    /// Network port this peers listens to
    port: u16,
    /// Random id for this peer
    pub id: usize,
}
#[derive(Debug, Error)]
pub enum AddrManError {}
impl From<AddrV2> for LocalAddress {
    fn from(value: AddrV2) -> Self {
        LocalAddress {
            address: value,
            last_connected: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            state: AddressState::NeverTried,
            services: ServiceFlags::NONE,
            port: 8333,
            id: rand::random::<usize>(),
        }
    }
}
impl From<AddrV2Message> for LocalAddress {
    fn from(value: AddrV2Message) -> Self {
        LocalAddress {
            address: value.addr,
            last_connected: value.time.into(),
            state: AddressState::NeverTried,
            services: value.services,
            port: value.port,
            id: rand::random::<usize>(),
        }
    }
}
impl FromStr for LocalAddress {
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        LocalAddress::try_from(s)
    }
    type Err = std::net::AddrParseError;
}
impl TryFrom<&str> for LocalAddress {
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let split = value.split(':').collect::<Vec<_>>();
        let address = split[0].parse::<Ipv4Addr>()?;
        let port = if let Some(port) = split.get(1) {
            port.parse().unwrap_or(8333)
        } else {
            8333
        };
        Ok(LocalAddress::new(
            AddrV2::Ipv4(address),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            super::address_man::AddressState::NeverTried,
            ServiceFlags::NONE,
            port,
            rand::random::<usize>(),
        ))
    }

    type Error = std::net::AddrParseError;
}

impl LocalAddress {
    pub fn new(
        address: AddrV2,
        last_connected: u64,
        state: AddressState,
        services: ServiceFlags,
        port: u16,
        id: usize,
    ) -> LocalAddress {
        LocalAddress {
            address,
            last_connected,
            state,
            services,
            port,
            id,
        }
    }
    /// Returns this address's port
    pub fn get_port(&self) -> u16 {
        self.port
    }
    /// Return an IP address associated with this peer address
    pub fn get_net_address(&self) -> IpAddr {
        match self.address {
            // IPV4
            AddrV2::Ipv4(ipv4) => IpAddr::V4(ipv4),
            // IPV6
            AddrV2::Ipv6(ipv6) => IpAddr::V6(ipv6),
            _ => IpAddr::V4(Ipv4Addr::LOCALHOST),
        }
    }
    /// Returns the actual address, as defined in AddrV2. This is useful
    /// if we are trying a peer that needs a proxy like Tor.
    pub fn get_address(&self) -> AddrV2 {
        self.address.clone()
    }
}

/// A module that keeps track of know addresses and serve them to our node to connect
#[derive(Default, Clone)]
pub struct AddressMan {
    addresses: HashMap<usize, LocalAddress>,
    good_addresses: Vec<usize>,
    good_peers_by_service: HashMap<ServiceFlags, Vec<usize>>,
    peers_by_service: HashMap<ServiceFlags, Vec<usize>>,
}

impl AddressMan {
    /// Add a new address to our list of known address
    pub fn push_addresses(&mut self, addresses: &[LocalAddress]) {
        for address in addresses {
            let id = address.id;

            // don't add addresses that don't have the minimum required services
            if !address.services.has(ServiceFlags::WITNESS)
                | !address.services.has(ServiceFlags::NETWORK)
            {
                continue;
            }

            // don't add private addresses
            if Self::is_localhost(address) || Self::is_private(address) {
                continue;
            }

            // don't add duplicate addresses
            if self
                .addresses
                .values()
                .any(|x| x.address == address.address)
            {
                continue;
            }

            if let std::collections::hash_map::Entry::Vacant(e) = self.addresses.entry(id) {
                e.insert(address.clone());
                if Self::is_good_peer(address) {
                    self.good_addresses.push(id);
                }

                self.push_if_has_service(address, service_flags::UTREEXO.into());
                self.push_if_has_service(address, ServiceFlags::NONE); // this means any peer
                self.push_if_has_service(address, ServiceFlags::COMPACT_FILTERS);
            }
        }
    }

    fn is_good_peer(address: &LocalAddress) -> bool {
        if Self::is_private(address) {
            return false;
        }

        matches!(address.state, AddressState::Connected)
            || matches!(address.state, AddressState::Tried(_))
    }

    fn is_private(address: &LocalAddress) -> bool {
        match address.address {
            AddrV2::Ipv4(ip) => ip.is_private(),
            AddrV2::Ipv6(ip) => ip.octets()[0] == 0xfd || ip.octets()[0] == 0xfe,
            _ => false,
        }
    }

    fn is_localhost(address: &LocalAddress) -> bool {
        match address.address {
            AddrV2::Ipv4(ip) => ip.is_loopback(),
            AddrV2::Ipv6(ip) => ip.is_loopback(),
            _ => false,
        }
    }

    fn push_if_has_service(&mut self, address: &LocalAddress, service: ServiceFlags) {
        if address.services.has(service) {
            if Self::is_good_peer(address) {
                self.good_peers_by_service
                    .entry(service)
                    .or_default()
                    .push(address.id);
            }

            self.peers_by_service
                .entry(service)
                .or_default()
                .push(address.id);
        }
    }

    pub fn get_addresses_to_send(&self) -> AddressToSend {
        let addresses = self
            .good_addresses
            .iter()
            .filter_map(|id| {
                let address = self.addresses.get(id)?;
                Some((
                    address.address.clone(),
                    address.last_connected,
                    address.services,
                    address.port,
                ))
            })
            .collect();

        addresses
    }

    fn do_lookup(host: &str, default_port: u16, socks5: Option<SocketAddr>) -> Vec<LocalAddress> {
        let ips = match socks5 {
            Some(proxy) => {
                debug!("Performing DNS lookup for host: {host}, using SOCKS5 proxy: {proxy}");
                // SOCKS5 proxy lookup (proxied DNS-over-HTTPS).
                dns_proxy::lookup_host_via_proxy(host, proxy).unwrap_or_else(|e| {
                    error!("DNS lookup via SOCKS5 proxy failed: {e}");
                    Vec::new()
                })
            }
            None => {
                debug!("Performing DNS lookup for host: {host}, using the system resolver");
                // System lookup (usually unencrypted, resolver sees both query and our IP).
                dns_lookup::lookup_host(host).unwrap_or_else(|e| {
                    error!("DNS lookup failed: {e}");
                    Vec::new()
                })
            }
        };

        if ips.is_empty() {
            warn!("No peer addresses read from DNS host: {host}");
        } else {
            info!("Fetched {} peer addresses from DNS host: {host}", ips.len());
        }

        let mut addresses = Vec::new();
        for ip in ips {
            if let Ok(ip) = LocalAddress::try_from(format!("{ip}:{default_port}").as_str()) {
                addresses.push(ip);
            }
        }

        addresses
    }

    pub fn get_seeds_from_dns(
        seed: &DnsSeed,
        default_port: u16,
        socks5: Option<SocketAddr>,
    ) -> Result<Vec<LocalAddress>, std::io::Error> {
        let mut addresses = Vec::new();

        // ask for utreexo peers (if filtering is available)
        if seed.filters.has(service_flags::UTREEXO.into()) {
            let host = format!("x1000000.{}", seed.seed);
            let _addresses = Self::do_lookup(&host, default_port, socks5);
            let _addresses = _addresses.into_iter().map(|mut x| {
                x.services =
                    ServiceFlags::NETWORK | service_flags::UTREEXO.into() | ServiceFlags::WITNESS;
                x
            });

            addresses.extend(_addresses);
        }

        // ask for compact filter peers (if filtering is available)
        if seed.filters.has(ServiceFlags::COMPACT_FILTERS) {
            let host = format!("x49.{}", seed.seed);
            let _addresses = Self::do_lookup(&host, default_port, socks5);
            let _addresses = _addresses.into_iter().map(|mut x| {
                x.services =
                    ServiceFlags::COMPACT_FILTERS | ServiceFlags::NETWORK | ServiceFlags::WITNESS;
                x
            });

            addresses.extend(_addresses);
        }

        // ask for any peer (if filtering is available)
        if seed.filters.has(ServiceFlags::WITNESS) {
            let host = format!("x9.{}", seed.seed);
            let _addresses = Self::do_lookup(&host, default_port, socks5);
            let _addresses = _addresses.into_iter().map(|mut x| {
                x.services = ServiceFlags::NETWORK | ServiceFlags::WITNESS;
                x
            });

            addresses.extend(_addresses);
        }

        // ask for any peer (if filtering isn't available)
        if seed.filters == ServiceFlags::NONE {
            let _addresses = Self::do_lookup(seed.seed, default_port, socks5);
            let _addresses = _addresses.into_iter().map(|mut x| {
                x.services = ServiceFlags::NETWORK | ServiceFlags::WITNESS;
                x
            });

            addresses.extend(_addresses);
        }

        Ok(addresses)
    }

    /// Returns a new random address to open a new connection, we try to get addresses with
    /// a set of features supported for our peers
    pub fn get_address_to_connect(
        &mut self,
        required_service: ServiceFlags,
        feeler: bool,
    ) -> Option<(usize, LocalAddress)> {
        if self.addresses.is_empty() {
            return None;
        }

        // Feeler connection are used to test if a peer is still alive, we don't care about
        // the features it supports or even if it's a valid peer. The only thing we care about
        // is that we haven't banned it.
        if feeler {
            let idx = rand::random::<usize>() % self.addresses.len();
            let peer = self.addresses.keys().nth(idx)?;
            let address = self.addresses.get(peer)?.to_owned();

            // don't try to connect to a peer that is banned or already connected
            if matches!(address.state, AddressState::Banned(_))
                | matches!(address.state, AddressState::Connected)
            {
                return None;
            }

            return Some((*peer, address));
        };

        for _ in 0..10 {
            let (id, peer) = self
                .get_address_by_service(required_service)
                .or_else(|| self.get_random_address(required_service))?;

            match peer.state {
                AddressState::NeverTried | AddressState::Tried(_) => {
                    return Some((id, peer));
                }

                AddressState::Connected => {
                    // if we are connected to this peer, don't try to connect again
                    continue;
                }

                AddressState::Banned(when) | AddressState::Failed(when) => {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();

                    if when + RETRY_TIME < now {
                        return Some((id, peer));
                    }

                    if let Some(peers) = self.good_peers_by_service.get_mut(&required_service) {
                        peers.retain(|&x| x != id)
                    }

                    self.good_addresses.retain(|&x| x != id);
                }
            }
        }

        None
    }

    pub fn dump_peers(&self, datadir: &str) -> std::io::Result<()> {
        let peers: Vec<_> = self
            .addresses
            .values()
            .cloned()
            .map(Into::<DiskLocalAddress>::into)
            .collect::<Vec<_>>();
        let peers = serde_json::to_string(&peers);
        if let Ok(peers) = peers {
            std::fs::write(datadir.to_owned() + "/peers.json", peers)?;
        }
        Ok(())
    }

    /// Dumps the connected utreexo peers to a file on dir `datadir/anchors.json` in json format `
    /// inputs are the directory to save the file and the list of ids of the connected utreexo peers
    pub fn dump_utreexo_peers(&self, datadir: &str, peers_id: &[usize]) -> std::io::Result<()> {
        let addresses: Vec<DiskLocalAddress> = peers_id
            .iter()
            .filter_map(|id| Some(self.addresses.get(id)?.to_owned().into()))
            .collect();
        let addresses: Result<String, serde_json::Error> = serde_json::to_string(&addresses);
        if let Ok(addresses) = addresses {
            std::fs::write(datadir.to_owned() + "/anchors.json", addresses)?;
        }
        Ok(())
    }

    fn get_address_by_service(&self, service: ServiceFlags) -> Option<(usize, LocalAddress)> {
        let peers = self.good_peers_by_service.get(&service)?;
        if peers.is_empty() {
            return None;
        }

        let idx = rand::random::<usize>() % peers.len();
        let utreexo_peer = peers.get(idx)?;

        Some((*utreexo_peer, self.addresses.get(utreexo_peer)?.to_owned()))
    }

    pub fn start_addr_man(&mut self, datadir: String) -> Vec<LocalAddress> {
        let persisted_peers = read_to_string(format!("{datadir}/peers.json"))
            .map(|seeds| serde_json::from_str::<Vec<DiskLocalAddress>>(&seeds));

        if let Ok(Ok(peers)) = persisted_peers {
            let peers = peers
                .into_iter()
                .map(Into::<LocalAddress>::into)
                .collect::<Vec<_>>();

            self.push_addresses(&peers);
        }

        let anchors = read_to_string(format!("{datadir}/anchors.json")).and_then(|anchors| {
            let anchors = serde_json::from_str::<Vec<DiskLocalAddress>>(&anchors)?;
            Ok(anchors
                .into_iter()
                .map(Into::<LocalAddress>::into)
                .collect::<Vec<_>>())
        });

        if anchors.is_err() {
            warn!("Failed to init Utreexo peers: anchors.json does not exist yet, or is invalid");
        }

        anchors.unwrap_or_default()
    }

    /// This function moves addresses between buckets, like if the ban time of a peer expired,
    /// or if we tried to connect to a peer and it failed in the past, but now it might be online
    /// again.
    pub fn rearrange_buckets(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        for (_, address) in self.addresses.iter_mut() {
            match address.state {
                AddressState::Banned(ban_time) => {
                    if ban_time < now {
                        address.state = AddressState::NeverTried;
                    }
                }
                AddressState::Tried(tried_time) => {
                    if tried_time + RETRY_TIME < now {
                        address.state = AddressState::NeverTried;
                    }
                }
                AddressState::Failed(failed_time) => {
                    if failed_time + RETRY_TIME < now {
                        address.state = AddressState::NeverTried;
                    }
                }
                AddressState::Connected | AddressState::NeverTried => {}
            }
        }
    }

    /// Attept to find one random peer that advertises the required service
    ///
    /// If we cannot find a peer that advertises the required service, we return any peer
    /// that we have in our list of known peers. Luckily, either we'll connect to a peer that has
    /// this but we didn't know, or one of those peers will give us useful addresses.
    fn try_with_service(&self, service: ServiceFlags) -> Option<(usize, LocalAddress)> {
        if let Some(peers) = self.peers_by_service.get(&service) {
            let peers = peers
                .iter()
                .filter(|&x| {
                    if let Some(address) = self.addresses.get(x) {
                        if let AddressState::Failed(when) = address.state {
                            let now = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_secs();

                            if (when + RETRY_TIME) < now {
                                return true;
                            }
                        }

                        return matches!(address.state, AddressState::Tried(_))
                            || matches!(address.state, AddressState::NeverTried);
                    }

                    false
                })
                .collect::<Vec<_>>();

            if peers.is_empty() {
                return None;
            }

            let idx = rand::random::<usize>() % peers.len();
            let utreexo_peer = peers.get(idx)?;
            return Some((**utreexo_peer, self.addresses.get(utreexo_peer)?.to_owned()));
        }

        None
    }

    fn get_random_address(&self, service: ServiceFlags) -> Option<(usize, LocalAddress)> {
        if self.addresses.is_empty() {
            return None;
        }

        if let Some(address) = self.try_with_service(service) {
            return Some(address);
        }

        // if we can't find a peer that advertises the required service, get any peer
        let idx = rand::random::<usize>() % self.addresses.len();
        let peer = self.addresses.keys().nth(idx)?;

        Some((*peer, self.addresses.get(peer)?.to_owned()))
    }

    /// Updates the state of an address
    pub fn update_set_state(&mut self, idx: usize, state: AddressState) -> &mut Self {
        match state {
            AddressState::Banned(_) => {
                self.good_addresses.retain(|&x| x != idx);
            }
            AddressState::Tried(_) => {
                if !self.good_addresses.contains(&idx) {
                    self.good_addresses.push(idx);
                }

                if let Some(address) = self.addresses.get(&idx).cloned() {
                    self.push_if_has_service(&address, service_flags::UTREEXO.into());
                    self.push_if_has_service(&address, ServiceFlags::from(1 << 25)); // UTREEXO_FILTER
                    self.push_if_has_service(&address, ServiceFlags::NONE); // this means any peer
                    self.push_if_has_service(&address, ServiceFlags::COMPACT_FILTERS);
                }
            }
            AddressState::NeverTried => {
                self.good_addresses.retain(|&x| x != idx);
            }
            AddressState::Connected => {
                if !self.good_addresses.contains(&idx) {
                    self.good_addresses.push(idx);
                }
            }
            AddressState::Failed(_) => {
                self.good_addresses.retain(|&x| x != idx);
                for peers in self.good_peers_by_service.values_mut() {
                    peers.retain(|&x| x != idx);
                }
            }
        }

        if let Some(address) = self.addresses.get_mut(&idx) {
            address.state = state;
        };

        self
    }

    /// Adds a peer to the list of peers known to have some service
    fn add_peer_to_service(&mut self, idx: usize, service: ServiceFlags) {
        if let Some(peers) = self.peers_by_service.get_mut(&service) {
            if peers.contains(&idx) {
                return;
            }

            peers.push(idx);
        } else {
            self.peers_by_service.insert(service, vec![idx]);
        }
    }

    /// Removes a peer from the list of peers known to have some service
    fn remove_peer_from_service(&mut self, idx: usize, service: ServiceFlags) {
        if let Some(peers) = self.peers_by_service.get_mut(&service) {
            peers.retain(|&x| x != idx);
        }
    }

    /// Updates the list of peers that have a service
    ///
    /// If a peer used to advertise a service, but now it doesn't, we remove it from the list
    /// of peers that have that service. If a peer didn't advertise a service, but now it does,
    /// we add it to the list of peers that have that service.
    fn update_peer_for_service(&mut self, id: usize, service: ServiceFlags) {
        let Some(peer) = self.addresses.get(&id) else {
            return;
        };

        match peer.services.has(service) {
            true => self.add_peer_to_service(id, service),
            false => self.remove_peer_from_service(id, service),
        }
    }

    /// Updates `peers_by_service` buckets with the latest service flags info about a peer
    ///
    /// This function is called when we receive a version message from a peer, telling which
    /// services it advertises.
    ///
    /// We only index for Compact Filters and Utreexo. For NODE_NETWORK and NODE_WITNESS we already
    /// filter them out when we add them to the address manager, therefore, all peers in this list
    /// is already known for having those. And we don't care about the rest of the services,
    /// like NODE_BLOOM.
    fn update_peer_services_buckets(&mut self, idx: usize) {
        self.update_peer_for_service(idx, service_flags::UTREEXO.into());
        self.update_peer_for_service(idx, ServiceFlags::COMPACT_FILTERS);
    }

    /// Updates the service flags after we receive a version message
    pub fn update_set_service_flag(&mut self, idx: usize, flags: ServiceFlags) -> &mut Self {
        // if this peer turns out to not have the minimum required services, we remove it
        if !flags.has(ServiceFlags::NETWORK) || !flags.has(ServiceFlags::WITNESS) {
            self.addresses.remove(&idx);
            for peers in self.peers_by_service.values_mut() {
                peers.retain(|&x| x != idx);
            }

            self.good_addresses.retain(|&x| x != idx);
            self.good_peers_by_service
                .values_mut()
                .for_each(|peers| peers.retain(|&x| x != idx));

            return self;
        }

        if let Some(address) = self.addresses.get_mut(&idx) {
            address.services = flags;
        }

        self.update_peer_services_buckets(idx);
        self
    }

    /// Returns the file path to the seeds file for the given network
    fn get_net_seeds(network: Network) -> &'static str {
        match network {
            Network::Bitcoin => include_str!("seeds/mainnet_seeds.json"),
            Network::Testnet => include_str!("seeds/testnet_seeds.json"),
            Network::Signet => include_str!("seeds/signet_seeds.json"),
            Network::Regtest => include_str!("seeds/regtest_seeds.json"),
            Network::Testnet4 => include_str!("seeds/testnet4_seeds.json"),
            // TODO: handle possible Err
            _ => panic!("Unsupported network"),
        }
    }

    /// Reads the hard-coded addresses from the seeds file and adds them to the address manager
    ///
    /// This is a last-resort method to try to connect to a peer, if we don't have any other
    /// addresses to connect to.
    pub(crate) fn add_fixed_addresses(&mut self, network: Network) {
        let addresses = Self::get_net_seeds(network);
        let peers: Vec<DiskLocalAddress> =
            serde_json::from_str(addresses).expect("BUG: fixed peers are invalid");

        let peers = peers
            .into_iter()
            .map(Into::<LocalAddress>::into)
            .collect::<Vec<_>>();

        self.push_addresses(&peers);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskLocalAddress {
    /// An actual address
    address: Address,
    /// Last time we successfully connected to this peer, only relevant is state == State::Tried
    last_connected: u64,
    /// Our local state for this peer, as defined in AddressState
    state: AddressState,
    /// Network services announced by this peer
    services: u64,
    /// Network port this peers listens to
    port: u16,
    /// An id to identify this address
    id: Option<usize>,
}
impl From<LocalAddress> for DiskLocalAddress {
    fn from(value: LocalAddress) -> Self {
        let address = match value.address {
            AddrV2::Ipv4(ip) => Address::V4(ip),
            AddrV2::Ipv6(ip) => Address::V6(ip),
            AddrV2::Cjdns(ip) => Address::Cjdns(ip),
            AddrV2::I2p(ip) => Address::I2p(ip),
            AddrV2::TorV2(ip) => Address::OnionV2(ip),
            AddrV2::TorV3(ip) => Address::OnionV3(ip),
            AddrV2::Unknown(_, _) => Address::V4(Ipv4Addr::LOCALHOST),
        };

        DiskLocalAddress {
            address,
            last_connected: value.last_connected,
            state: if value.state == AddressState::Connected {
                AddressState::Tried(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                )
            } else {
                value.state
            },
            services: value.services.to_u64(),
            port: value.port,
            id: Some(value.id),
        }
    }
}
impl From<DiskLocalAddress> for LocalAddress {
    fn from(value: DiskLocalAddress) -> Self {
        let address = match value.address {
            Address::V4(ip) => AddrV2::Ipv4(ip),
            Address::V6(ip) => AddrV2::Ipv6(ip),
            Address::Cjdns(ip) => AddrV2::Cjdns(ip),
            Address::I2p(ip) => AddrV2::I2p(ip),
            Address::OnionV2(ip) => AddrV2::TorV2(ip),
            Address::OnionV3(ip) => AddrV2::TorV3(ip),
        };
        let services = ServiceFlags::from(value.services);
        LocalAddress {
            address,
            last_connected: value.last_connected,
            state: value.state,
            services,
            port: value.port,
            id: value.id.unwrap_or_else(rand::random::<usize>),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Address {
    /// Regular ipv4 address
    V4(Ipv4Addr),
    /// Regular ipv6 address
    V6(Ipv6Addr),
    /// Tor v2 address, this may never be used, as OnionV2 is deprecated
    /// but we'll keep it here for completeness sake
    OnionV2([u8; 10]),
    /// Tor v3 address. This is the preferred way to connect to a tor node
    OnionV3([u8; 32]),
    /// Cjdns ipv6 address
    Cjdns(Ipv6Addr),
    /// I2p address, a 32 byte node key
    I2p([u8; 32]),
}

/// Simple implementation of a DNS-over-HTTPS (DoH) lookup routed through the SOCKS5 proxy
pub mod dns_proxy {
    use std::net::IpAddr;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::time::Duration;

    use rustls::crypto;
    use serde::Deserialize;
    use ureq::tls::TlsConfig;
    use ureq::tls::TlsProvider;
    use ureq::Agent;
    use ureq::Proxy;

    #[derive(Deserialize)]
    /// JSON format from [Google's DoH API](https://developers.google.com/speed/public-dns/docs/doh/json#dns_response_in_json)
    struct DnsResponse {
        /// We only care about the "Answer" array
        #[serde(rename = "Answer")]
        answers: Option<Vec<AnswerEntry>>,
    }

    #[derive(Deserialize)]
    struct AnswerEntry {
        /// The IP address as a string
        data: String,

        /// Record type; 1=A, 28=AAAA
        #[serde(rename = "type")]
        record_type: u8,
    }

    /// Lookup `host` by DNS-over-HTTPS (DoH) through a SOCKS5 proxy. Returns both A (IPv4)
    /// and AAAA (IPv6) records. Only Google sees the actual DNS query but doesn't learn our IP.
    pub fn lookup_host_via_proxy(
        host: &str,
        proxy_addr: SocketAddr,
    ) -> Result<Vec<IpAddr>, ureq::Error> {
        // Note: ureq does not implement "socks5h://", so this will resolve "dns.google" locally,
        // but the Bitcoin DNS query remains encrypted. Only Google can see the query contents.
        let proxy = Proxy::new(&format!("socks5://{proxy_addr}"))?;

        let crypto = Arc::new(crypto::aws_lc_rs::default_provider());
        let tls_config = TlsConfig::builder()
            .provider(TlsProvider::Rustls)
            .unversioned_rustls_crypto_provider(crypto)
            .build();

        let agent: Agent = Agent::config_builder()
            .tls_config(tls_config)
            .timeout_global(Some(Duration::from_secs(30)))
            .proxy(Some(proxy))
            .build()
            .into();

        // We will perform two queries in sequence: type=1 (A) and type=28 (AAAA).
        let mut all_ips = Vec::new();
        for record_type in [1u8, 28u8] {
            let mut ips = query(&agent, host, record_type)?;
            all_ips.append(&mut ips);
        }

        Ok(all_ips)
    }

    // Helper function that performs a single DoH query for the given record_type.
    fn query(agent: &Agent, host: &str, record_type: u8) -> Result<Vec<IpAddr>, ureq::Error> {
        // Construct the DoH URL for the JSON API:
        // https://developers.google.com/speed/public-dns/docs/secure-transports
        let url = format!("https://dns.google/resolve?name={host}&type={record_type}");

        // Send a GET over HTTPS. The proxy will only see Google's address and the TLS handshake.
        let mut response = agent.get(&url).call()?;
        let dns_response: DnsResponse = response.body_mut().read_json()?;

        let answers = dns_response.answers.unwrap_or_default();

        // Filter by record_type (sanity) and parse each "data" field into an IpAddr.
        let mut result = Vec::new();
        for entry in answers.into_iter().filter(|e| e.record_type == record_type) {
            if let Ok(ip) = entry.data.parse() {
                result.push(ip);
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use std::fs::File;
    use std::io::Read;
    use std::io::{self};
    use std::net::Ipv4Addr;

    use bitcoin::p2p::address::AddrV2;
    use bitcoin::p2p::ServiceFlags;
    use bitcoin::Network;
    use floresta_chain::get_chain_dns_seeds;
    use floresta_common::assert_ok;
    use floresta_common::service_flags;
    use rand::Rng;
    use serde::Deserialize;
    use serde::Serialize;

    use super::AddressState;
    use super::LocalAddress;
    use crate::address_man::AddressMan;

    /// Seed Data for paesing in tests.
    #[derive(Debug, Clone, PartialEq, Deserialize)]
    pub struct SeedData {
        /// An actual address
        address: SeedAddress,
        /// Last time we successfully connected to this peer, only relevant is state == State::Tried
        last_connected: u64,
        /// Our local state for this peer, as defined in AddressState
        state: AddressState,
        /// Network services announced by this peer
        pub services: u64,
        /// Network port this peers listens to
        port: u16,
    }

    #[allow(non_snake_case)]
    #[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
    struct SeedAddress {
        V4: Ipv4Addr,
    }

    fn load_addresses_from_json(file_path: &str) -> io::Result<Vec<LocalAddress>> {
        let mut contents = String::new();
        File::open(file_path)?.read_to_string(&mut contents)?;

        let seeds: Vec<SeedData> =
            serde_json::from_str(&contents).expect("JSON not well-formatted");
        let mut addresses = Vec::new();
        let mut rng = rand::thread_rng();

        for seed in seeds {
            let state = match seed.state {
                AddressState::Tried(time) => AddressState::Tried(time),
                _ => continue,
            };

            let _address = AddrV2::Ipv4(seed.address.V4);

            let local_address = LocalAddress {
                address: _address,
                last_connected: seed.last_connected,
                state,
                services: ServiceFlags::from(seed.services),
                port: seed.port,
                id: rng.gen(),
            };
            addresses.push(local_address);
        }

        Ok(addresses)
    }
    #[test]
    fn test_parse() {
        let signet_address =
            load_addresses_from_json("./src/p2p_wire/seeds/signet_seeds.json").unwrap();

        assert!(!signet_address.is_empty());
        let random = rand::thread_rng().gen_range(1..=13);
        let loc_adr_1 = LocalAddress::from(signet_address[random].address.clone());
        assert_eq!(loc_adr_1.address, signet_address[random].address);
    }

    #[test]
    fn test_fixed_peers() {
        let _ = load_addresses_from_json("./src/p2p_wire/seeds/signet_seeds.json").unwrap();
        let _ = load_addresses_from_json("./src/p2p_wire/seeds/mainnet_seeds.json").unwrap();
        let _ = load_addresses_from_json("./src/p2p_wire/seeds/testnet_seeds.json").unwrap();
        let _ = load_addresses_from_json("./src/p2p_wire/seeds/regtest_seeds.json").unwrap();
    }

    #[test]
    fn test_address_man() {
        let mut address_man = AddressMan::default();

        let signet_address =
            load_addresses_from_json("./src/p2p_wire/seeds/signet_seeds.json").unwrap();

        address_man.push_addresses(&signet_address);

        assert!(!address_man.good_addresses.is_empty());

        assert!(!address_man.peers_by_service.is_empty());

        assert!(!address_man.get_addresses_to_send().is_empty());

        assert!(address_man
            .get_address_to_connect(ServiceFlags::default(), true)
            .is_some());

        assert!(address_man
            .get_address_to_connect(ServiceFlags::default(), false)
            .is_some());

        assert!(address_man
            .get_address_to_connect(ServiceFlags::NONE, false)
            .is_some());

        assert!(address_man
            .get_address_to_connect(service_flags::UTREEXO.into(), false)
            .is_some());

        assert!(!AddressMan::get_net_seeds(Network::Signet).is_empty());
        assert!(!AddressMan::get_net_seeds(Network::Bitcoin).is_empty());
        assert!(!AddressMan::get_net_seeds(Network::Regtest).is_empty());
        assert!(!AddressMan::get_net_seeds(Network::Testnet).is_empty());

        assert_ok!(AddressMan::get_seeds_from_dns(
            &get_chain_dns_seeds(Network::Signet).unwrap()[0],
            8333,
            None, // No proxy
        ));

        address_man.rearrange_buckets();
    }
}
