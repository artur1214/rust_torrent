use std::collections::HashMap;
use serde;
use serde::{Serialize, ser::{SerializeMap}};
use serde_json::{json, Value};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

mod read_torrent_data;


#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Bencode {
    String(String),
    Integer(i64),
    List(Vec<Bencode>),
    Dictionary(Vec<(Bencode, Bencode)>),
    Bytes(Vec<u8>)
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
