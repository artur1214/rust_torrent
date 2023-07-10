use std::{error::Error, net::SocketAddr};

use derive_builder::Builder;
use tokio::net::{lookup_host, UdpSocket};

#[derive(Debug)]
enum AnnounceType {
    IPv6,
    IPv4,
}


#[repr(u32)]
#[derive(Clone)]
enum AnnounceEventType {
    UNDEFINED,
    COMPLETED,
    STARTED, 
    STOPPED
}

//TODO: Maybe rewrite with tuple struct??
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
