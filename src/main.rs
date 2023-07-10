mod network_manager;
mod peer_messaging;
use bencoding::read_torrent_from_file;
use tokio::io;
use tokio::net::TcpStream;

#[tokio::main]
async fn main() {
    println!("Hello world!");
    let torrent_data = read_torrent_from_file("test.torrent").await.expect("Err");

    //println!("{:?}", torrent_data)
}

#[tokio::test]
async fn test_read_file() {}
