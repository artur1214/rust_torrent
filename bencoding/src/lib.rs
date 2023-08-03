use std::collections::HashMap;
use std::iter::zip;
use std::str::FromStr;
use serde;
use serde::{Serialize, ser::{SerializeMap}};
use serde_json::{json, Value};
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, AsyncReadExt};

mod read_torrent_data;


#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Bencode {
    String(String),
    Integer(i64),
    List(Vec<Bencode>),
    Dictionary(Vec<(Bencode, Bencode)>),
    Bytes(Vec<u8>)
}

impl Bencode {
    fn to_bencode_bytes(&self) -> Vec<u8> {
        let mut collector: Vec<u8> = Vec::new();
        match self {
            Bencode::String(value) => {
                collector.extend((value.chars().count().to_string() + ":" + value).as_bytes());
                //old_collector.push_str((value.chars().count().to_string() + ":" + value).as_str());
                //return old_collector;
            },
            Bencode::Integer(value) => {
                collector.push(b'i');
                collector.extend(value.to_string().as_bytes());
                collector.push(b'e');

                // old_collector.push_str("i");
                // old_collector.push_str(value.to_string().as_str());
                // old_collector.push('e');
                //return old_collector;
            },
            Bencode::List(values) => {
                collector.push(b'l');
                for value in values {
                    collector.extend(value.to_bencode_bytes());
                }
                collector.push(b'e');
            },
            Bencode::Dictionary(values) => {
                collector.push(b'd');
                //let mut values: Vec<(Bencode, Bencode)> = Vec::new();
                //values.extend_from_slice(&old_values); 
                //values.sort_by(|a,b|a.0.to_string().cmp(&b.0.to_string()));
               
                // TODO: BITTORRENT SPECS SAYS, THAT DICTS MUST BE SORTED LEXICOGRAPHICAL. THEORETICALLY, ALL OF TORRENT FILES MUST BE ALREADY SORTED, 
                //BUT IN FACT, HERE MUST BE ADDED SORTING. PROBLEM IS VALUES ARE IMMUTABLE AND UNCLONABLE
                for (key, value) in values {
                    collector.extend(key.to_bencode_bytes());
                    collector.extend(value.to_bencode_bytes());
                }
                collector.push(b'e')
            },
            Bencode::Bytes(value) => {
                collector.extend(value.len().to_string().as_bytes());
                collector.push(b':');
                collector.extend(value);
                //return old_collector;
            },
        }
        return collector;
    }
    fn try_get_info_hash(&self) {

    }
}


impl Serialize for Bencode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Bencode::String(s) => serializer.serialize_str(s),
            Bencode::Integer(i) => serializer.serialize_i64(*i),
            Bencode::List(list) => {
                let serialized_list: Vec<Value> = list
                    .iter()
                    .map(|item| serde_json::to_value(item).unwrap())
                    .collect();
                serializer.collect_seq(serialized_list.into_iter())
            }
            Bencode::Dictionary(dict) => {
                let serialized_dict: HashMap<String, Value> = dict
                    .iter()
                    .map(|(key, value)| {
                        let key_str = serde_json::to_value(key).unwrap().as_str().unwrap().to_string();
                        let value_json = serde_json::to_value(value).unwrap();
                        (key_str, value_json)
                    })
                    .collect();
                let mut map_serializer = serializer.serialize_map(Some(serialized_dict.len()))?;
                for (key, value) in serialized_dict.clone() {
                    map_serializer.serialize_entry(&key, &value)?;
                }
                let res = map_serializer.end()?;
                Ok(res)
            }
            Bencode::Bytes(bytes) => {
                serializer.serialize_bytes(&bytes)
            },
        }
    }
}
impl ToString for Bencode {
    fn to_string(&self) -> String {
        match self {
            Bencode::String(value) => return value.clone(),
            Bencode::Integer(value) => value.to_string(),
            Bencode::List(list) => json!(list).to_string(),
            Bencode::Dictionary(_) => todo!(),
            Bencode::Bytes(_) => todo!(),
        }
    }
}

pub async fn read_torrent_from_file<T: AsRef<str>>(filename: T) -> Result<Bencode, String>{
    let content = tokio::fs::read(filename.as_ref()).await;
    match content {
        Ok(res) => {
            let data = decode_bencode(&res);
            match data {
                Some(value) => Ok(value),
                None => Err("Some uknown error. File has incorrect data.".to_string()),
            }
        },
        Err(err) => {
            Err(err.to_string())
        },
    }
}

pub fn decode_bencode(data: &[u8]) -> Option<Bencode> {
    let mut iterator = data.iter();
    decode_value(&mut iterator)
}

fn decode_value(iterator: &mut std::slice::Iter<u8>) -> Option<Bencode> {
    let mut length = String::new();
    match iterator.next()? {
        b'd' => decode_dictionary(iterator),
        b'l' => decode_list(iterator),
        b'i' => decode_integer(iterator),
        ch @ b'0'..=b'9' => {
            length.push(*ch as char);
            decode_string(iterator, &mut length)
        },
        _ => None
    }
}

fn decode_dictionary(iterator: &mut std::slice::Iter<u8>) -> Option<Bencode> {
    let mut dictionary = Vec::new();
    let mut x = 0;
    while let Some(key) = decode_string(iterator, &mut String::new()) {
        //println!("{:?}", &key);
        x+=1;
        let value = decode_value(iterator)?;
        dictionary.push((key, value));
    }
    //println!("while ended at {:?}", x);
    Some(Bencode::Dictionary(dictionary))
}

fn decode_list(iterator: &mut std::slice::Iter<u8>) -> Option<Bencode> {
    let mut list = Vec::new();

    while let Some(value) = decode_value(iterator) {
        list.push(value);
    }

    Some(Bencode::List(list))
}

fn decode_integer(iterator: &mut std::slice::Iter<u8>) -> Option<Bencode> {
    let mut integer = String::new();

    while let Some(&byte) = iterator.next() {
        match byte {
            b'e' => {
                if let Ok(value) = integer.parse::<i64>() {
                    return Some(Bencode::Integer(value));
                } else {
                    break;
                }
            }
            b'0'..=b'9' | b'-' => integer.push(byte as char),
            _ => break,
        }
    }

    None
}

fn decode_string(iterator: &mut std::slice::Iter<u8>, length: &mut String) -> Option<Bencode> {
    //let mut length = String::new();

    while let Some(&byte) = iterator.next() {
        match byte {
            b':' => {
                //println!("GOT DATA: {:?} {:?} ", length, byte);
                if let Ok(len) = length.parse::<usize>() {
                    let mut string = Vec::with_capacity(len);
                    for _ in 0..len {
                        if let Some(&byte) = iterator.next() {
                            string.push(byte);
                        } else {
                            return None;
                        }
                    }
                    if String::from_utf8(string.clone()).is_err(){
                        return Some(Bencode::Bytes(string.clone()));
                    }
                    return Some(Bencode::String(String::from_utf8_lossy(&string).into_owned()));
                } else {
                    //println!("ERRRR");
                    break;
                }
            }
            b'0'..=b'9' => length.push(byte as char),
            _ => break,
        }
    }

    None
}
impl Bencode {
    pub async fn to_json(&self) -> Value {
        return json!(self);
    }
}
#[tokio::test]
async fn test_decode_from_file_and_write_json() {

    
    if let Some(decoded) = read_torrent_from_file("test2.torrent").await.ok() {
        let json_data = decoded.to_json().await;
        let file_path = "data3.json";
        let mut file = File::create(file_path).await.expect("Не удалось создать файл");
        let json_string = serde_json::to_string_pretty(&json_data).expect("Ошибка сериализации в JSON");
        file.write_all(json_string.as_bytes()).await.expect("Ошибка записи в файл");

    } else {
        println!("Error decoding Bencode");
        assert!(false);
    }
}

#[tokio::test]
async fn test_decode_from_file_and_encode_again() {
    if let Some(decoded) = read_torrent_from_file("test2.torrent").await.ok() {
        let encoded_data = decoded.to_bencode_bytes();
    
        let file_path = "data2_encoded.torrent";
        let mut file = File::create(file_path).await.expect("Не удалось создать файл");
        
        file.write_all(&encoded_data).await.expect("Ошибка записи в файл");
        let mut container = vec![];
        File::open("test2.torrent").await.unwrap().read_to_end(&mut container).await.unwrap();
        let mut i = 0;
        for (old, new) in zip(container, encoded_data) {
            println!("{}:{}    {}", old, new, i);
            assert_eq!(old, new);
            i+=1;
        }
    } else {
        println!("Error decoding Bencode");
        assert!(false);
    }
}