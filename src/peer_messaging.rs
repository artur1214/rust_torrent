use std::{error::Error, net::SocketAddr, mem};

use derive_builder::Builder;
use num_enum::TryFromPrimitive;
use tokio::net::{lookup_host, UdpSocket};

#[derive(Debug)]
enum AnnounceType {
    IPv6,
    IPv4,
}



#[derive(Clone, TryFromPrimitive)]
#[repr(u32)]
enum AnnounceEventType {
    UNDEFINED,
    COMPLETED,
    STARTED, 
    STOPPED
}

//TODO: Maybe rewrite with tuple struct??
//#[repr(packed)] //Maybe enable? GOOD: Easy memcopy BAD: may cause indian problems, may cause crush at ARM arch. 
#[repr(C)]
#[derive(Builder) ]
struct IpV4AnnounceRequest {
    connection_id: u64,
    action: u32,
    transaction_id: u32,
    info_hash: [u8; 20],
    peer_id: [u8; 20],
    downloaded: u64,
    left: u64,
    uploaded: u64,
    event: AnnounceEventType,
    ip_address: u32,
    key: u32,
    num_want: u32,
    port: u16
}
impl IpV4AnnounceRequest {
    fn to_bytes(&self) -> [u8; 98] {
        return [0;98];
    }
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() <  104 {
            return None;
        }

        Some(Self {
            connection_id: u64::from_be_bytes(bytes[0..8].try_into().unwrap()),
            action: u32::from_be_bytes(bytes[8..12].try_into().unwrap()),
            transaction_id: u32::from_be_bytes(bytes[12..16].try_into().unwrap()),
            info_hash: bytes[16..36].try_into().unwrap(),
            peer_id: bytes[36..56].try_into().unwrap(),
            downloaded: u64::from_be_bytes(bytes[56..64].try_into().unwrap()),
            left: u64::from_be_bytes(bytes[64..72].try_into().unwrap()),
            uploaded: u64::from_be_bytes(bytes[72..80].try_into().unwrap()),
            event: AnnounceEventType::try_from(u32::from_be_bytes(bytes[80..84].try_into().unwrap())).unwrap(),
            ip_address: u32::from_be_bytes(bytes[84..88].try_into().unwrap()),
            key: u32::from_be_bytes(bytes[88..92].try_into().unwrap()),
            num_want: u32::from_be_bytes(bytes[92..96].try_into().unwrap()),
            port: u16::from_be_bytes(bytes[96..98].try_into().unwrap()),
        })
    }
}

impl IpV4AnnounceResponse {
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 20 {
            return None;
        }
        let action = u32::from_be_bytes(bytes[0..4].try_into().unwrap());
        let transaction_id = u32::from_be_bytes(bytes[4..8].try_into().unwrap());
        let interval = u32::from_be_bytes(bytes[8..12].try_into().unwrap());
        let leechers = u32::from_be_bytes(bytes[12..16].try_into().unwrap());
        let seeders = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        let mut addresses = Vec::new();
        if bytes.len() >= 26 {
            let address_bytes = &bytes[20..];
            if address_bytes.len() % 6 == 0 {
                for i in (0..address_bytes.len()).step_by(6) {
                    let ip = u32::from_be_bytes(//[
                        address_bytes[i..i+4].try_into().unwrap()
                        // address_bytes[i + 1],
                        // address_bytes[i + 2],
                        // address_bytes[i + 3],]
                    );
                    let port = u16::from_be_bytes([address_bytes[i + 4], address_bytes[i + 5]]);
                    addresses.push(IpV4AnnounceAddress { ip, port });
                }
            }
            else{
                return None;
            }
        }

        Some(IpV4AnnounceResponse {
            action,
            transaction_id,
            interval,
            leechers,
            seeders,
            addresses,
        })
    }
}

#[repr(C)]
struct IpV4AnnounceResponse {
    action: u32,
    transaction_id: u32,
    interval: u32,
    leechers: u32,
    seeders: u32,
    addresses: Vec<IpV4AnnounceAddress>,
}

#[repr(C)]
struct IpV4AnnounceAddress {
    ip: u32,
    port: u16,
}


#[derive(Debug)]
struct Announce {
    host: String,
    port: String,
    ip: String,
    sock_addr: SocketAddr,
    sock: UdpSocket,
    connection_id: Option<u64>,
    connected: bool,
    announce_type: AnnounceType,
}

async fn resolve_hostname_dns<T: AsRef<str>>(addr: T) -> Result<SocketAddr, String> {
    let addr = addr.as_ref();
    let resolved = lookup_host(addr).await;
    match resolved {
        Ok(mut addr) => match addr.next() {
            Some(value) => Ok(value),
            None => Err("Error during addr resolving".to_string()),
        },
        Err(err) => Err(err.to_string()),
    }
}


impl Announce {
    async fn get_announce_data() {
        
    }

    async fn get_connection_id(&mut self) -> Result<(), String> {
        let mut buf: Vec<u8> = vec![0; 16];
        buf[0..8].copy_from_slice(&(0x41727101980u64.to_be_bytes())); // Write magic constant. ALL IN BIG ENDIAN;
        buf[8..12].copy_from_slice(&0u32.to_be_bytes());
        buf[12..].copy_from_slice(&12345u32.to_be_bytes());
        let _sended_len = self.sock.send(&mut buf).await.unwrap();
        let mut buf = vec![0; 1024];
        let recieved_len = self
            .sock
            .recv(&mut buf)
            .await
            .expect("Error while getting response");
        buf.resize(recieved_len, 0);
        println!("{recieved_len}");
        assert!(buf.len() >= 16);
        println!(
            "Got Action (0=connect) {}",
            u32::from_be_bytes(buf[0..4].try_into().unwrap())
        );
        println!(
            "Got transaction_id sended 12345 {}",
            u32::from_be_bytes(buf[4..8].try_into().unwrap())
        );
        println!(
            "Got connection id {}",
            u64::from_be_bytes(buf[8..16].try_into().unwrap())
        );
        self.connection_id = Some(u64::from_be_bytes(buf[8..16].try_into().unwrap()));
        Ok(())
    }

    async fn new<T: AsRef<str>>(addr: T) -> Result<Self, String> {
        let created: Result<Announce, String> = {
            let addr = addr.as_ref().to_string();
            match addr.parse::<SocketAddr>().ok() {
                Some(sock_addr) => match UdpSocket::bind("0.0.0.0:6881").await {
                    Ok(sock) => Ok(Self {
                        host: addr.split(':').next().unwrap().to_string(),
                        port: sock_addr.port().to_string(),
                        ip: sock_addr.ip().to_string(),
                        sock_addr: sock_addr,
                        sock: sock,
                        connection_id: None,
                        connected: false,
                        announce_type: if sock_addr.is_ipv4() {
                            AnnounceType::IPv4
                        } else {
                            AnnounceType::IPv6
                        },
                    }),
                    Err(_err) => return Err("asdds".to_string()),
                },
                None => {
                    // todo!();
                    let resolved = resolve_hostname_dns(addr.clone()).await;
                    match resolved {
                        Ok(sock_addr) => match UdpSocket::bind("0.0.0.0:6881").await {
                            Ok(sock) => Ok(Self {
                                host: addr.split(':').next().unwrap().to_string(),
                                port: sock_addr.port().to_string(),
                                ip: sock_addr.ip().to_string(),
                                sock_addr: sock_addr,
                                sock: sock,
                                connection_id: None,
                                connected: false,
                                announce_type: if sock_addr.is_ipv4() {
                                    AnnounceType::IPv4
                                } else {
                                    AnnounceType::IPv6
                                },
                            }),
                            Err(_err) => return Err("asdds".to_string()),
                        },
                        Err(err) => Err(err),
                    }
                }
            }
        };
        match created {
            Ok(mut created) => match created.sock.connect(created.sock_addr).await {
                Ok(_) => {
                    created.connected = true;
                    Ok(created)
                }
                Err(err) => Err(err.to_string()),
            },
            Err(err) => Err(err),
        }
        //return created;
    }
}

#[tokio::test]
async fn test_announce() {
    let mut announcer = Announce::new("opentor.net:6969").await.unwrap();
    announcer.get_connection_id().await;

    print!("{:?}", announcer);
}

#[tokio::test]
async fn test_struct_size() {
    let size = mem::size_of::<IpV4AnnounceRequest>();
    println!("Размер структуры: {} байт", size);
}

#[tokio::test]
async fn test_connect_to_udp_peer() {
    let sock = UdpSocket::bind("0.0.0.0:6681")
        .await
        .expect("Unable to connect to udp peer");

    let resolved = lookup_host("opentor.net:6969")
        .await
        .unwrap()
        .next()
        .expect("No hostname resolved");
    sock.connect(resolved).await.unwrap();
    let mut buf: Vec<u8> = vec![0; 16];
    buf[0..8].copy_from_slice(&(0x41727101980u64.to_be_bytes())); // Write magic constant. ALL IN BIG ENDIAN;
    buf[8..12].copy_from_slice(&0u32.to_be_bytes());
    buf[12..].copy_from_slice(&12345u32.to_be_bytes());
    let _sended_len = sock.send(&mut buf).await.unwrap();
    let mut buf = vec![0; 1024];
    let recieved_len = sock
        .recv(&mut buf)
        .await
        .expect("Error while getting response");
    buf.resize(recieved_len, 0);
    //println!("{recieved_len}");
    assert!(buf.len() >= 16);
    //println!("Got Action (0=connect) {}", u32::from_be_bytes(buf[0..4].try_into().unwrap()) );
    //println!("Got transaction_id sended 12345 {}", u32::from_be_bytes(buf[4..8].try_into().unwrap()) );
    //println!("Got connection id {}", u64::from_be_bytes(buf[8..16].try_into().unwrap()) );
}
