use bitcoin::consensus::Decodable;
use std::collections::HashMap;
use hex_lit::hex;
use rusqlite::Connection;
use rusty_leveldb::{DBIterator, LdbIterator, Options, DB};
use std::env;
use std::fs::OpenOptions;
use std::fs;
use std::io::Write;
use std::io::Seek;
use std::io::SeekFrom;
use std::collections::BTreeMap;
use sled;
use std::io::BufRead;
use std::io::{self, Cursor, Read};
const OBFUSCATION_KEY: u8 = 14;
const UTXO_KEY: u8 = 67;

// https://github.com/bitcoin/bitcoin/blob/4cc99df44aec4d104590aee46cf18318e22a8568/src/dbwrapper.cpp#L323C19-L323C73
// length + 0 + key

const OBFUSCATE_KEY_KEY: [u8; 15] = *b"\x0E\0obfuscate_key";

fn deobfuscate_value(value: Vec<u8>, obfuscate_key: &[u8]) -> Vec<u8> {
    value
        .bytes()
        .enumerate()
        .map(|(index, byte)| byte.unwrap() ^ (obfuscate_key[index % obfuscate_key.len()]))
        .collect()

}

#[derive(Debug)]
struct Utxo {
    coinbase: bool,
    height: u64,
    amount: u64,
    script_type: u64,
    script: Vec<u8>,
}

fn decode_utxo_value(value: Vec<u8>) -> Utxo {
    let mut cursor = Cursor::new(value.clone());
    let height_and_coinbase: u64 = read_varint(&mut cursor).unwrap();
    let height = height_and_coinbase >> 1;
    let coinbase = (height_and_coinbase & 1) != 0;
    let amount = decompress_amount(read_varint(&mut cursor).unwrap());
    let script_type = read_varint(&mut cursor).unwrap();
    // println!("{}", script_type);
    if script_type != 0 {
        cursor.seek(SeekFrom::Current(-1)).unwrap();
    }
    let mut script = Vec::new();
    cursor.read_to_end(&mut script);

    Utxo {
        coinbase,
        height,
        amount,
        script_type,
        script,
    }
}
fn get_obfuscation_key(mut db: &mut DB) -> Vec<u8> {
    db.get(&OBFUSCATE_KEY_KEY).expect("obfuscation_key not set")[1..].into() 
}

fn dump_utxos(mut db: &mut DB, obfuscation_key: &[u8]) {
    // let sled_db: sled::Db = sled::open("utxos").unwrap();
    let mut x = db.new_iter().unwrap();
    let total_keys = 166423828;
    // let mut segwit_total = 0;
    // let mut segwit_count = 0;
    let mut max_height = 0;
    let mut max_script_length = 0;
    let mut max_vout = 0;

    // let mut balances: HashMap<[u8; 20], u64>  = HashMap::new();
    let mut key: ([u8; 8], u16) = ([0; 8], 0);
    let mut new_key: ([u8; 8], u16) = ([0; 8], 0);
    // let mut key: u64 = 0;
    // let mut new_key: u64 = 0;

    let mut utxos: BTreeMap<([u8; 8], u16), [u8; 28]>  = BTreeMap::new();
    let mut amount: u128 = 0;
    let mut utxo_count: u32 = 0;
    let mut key_count: u64 = 0;
    let mut buffer :Vec<(Vec<u8>, Vec<u8>)>= vec![];
    while x.advance() {
        key_count += 1;
        let (mut k, mut obfuscated_value) = (vec![], vec![]);
        x.current(&mut k, &mut obfuscated_value);
        if (k[0] == UTXO_KEY) {
            utxo_count += 1;
            let mut value = deobfuscate_value(obfuscated_value, obfuscation_key);
            let mut tx_id = k[1..33].to_vec(); 
            tx_id.reverse();
            let utxo = decode_utxo_value(value.clone());
            let mut cursor = Cursor::new(k[33..].to_vec().clone());
            let vout: u16 = read_varint(&mut cursor).unwrap().try_into().unwrap();

            if(utxo.script_type == 28) {
                // new_key = u64::from_be_bytes(k[1..9].try_into().unwrap());

                // let new_key = [
                //     k[1..9].to_vec(),
                //     vout.to_be_bytes().to_vec(),
                // ].concat().try_into().unwrap();
                let new_key = (k[1..9].try_into().unwrap(), vout); 
                // println!("{} > {}", hex::encode(key), hex::encode(new_key));
                if new_key <= key{
                    panic!("not sequential")
                }
                key = new_key;
                // println!("{}",[
                //     tx_id.to_vec(),
                //     vout.to_be_bytes().to_vec(),
                // ].concat().len());
            utxos.insert(
                key,
                [
                    utxo.amount.to_be_bytes().to_vec(),
                    utxo.script[3..].try_into().unwrap(),

                ].concat().try_into().unwrap()
            );
            // ));
                // segwit_count += 1;
                // segwit_total += utxo.amount;
                // println!("{}", hex::encode(&tx_id));
                // println!("{}", utxo.script.len());
                // println!("{}", hex::encode(utxo.script));
                // *balances.entry(utxo.script[3..].try_into().unwrap()).or_insert(0) += utxo.amount;
                // break;
            }
            // println!("{} {}", utxo.compressed_script[0], utxo.compressed_script.len());
            // break;
            if (vout > max_vout) {
                max_vout = vout; 
                // println!("new max vout {} {}", vout, hex::encode(&tx_id));
                
            }
            if (utxo.height > max_height) {
                max_height = utxo.height; 
                
            }

            if (utxo.script.len() as u64> max_script_length) {
                max_script_length = utxo.script.len() as u64;
                // println!("new max script len {} {}", max_script_length, hex::encode(&tx_id));
            }
            // buffer.push((
            //     [
            //         tx_id[..8].to_vec(),
            //         k[33..].to_vec()
            //     ].concat(),

            //     [
            //         utxo.amount.to_be_bytes().to_vec(),
            //         utxo.compressed_script

            //     ].concat()

            // ));
            //  amount+= utxo.amount as u128;

             if(utxo_count %100000 == 0) {
                // break;
            //     for (key, value) in &buffer {
            //         sled_db.insert(key, value.to_vec());
            //     }
            //     let buffer :Vec<(Vec<u8>, Vec<u8>)>= vec![];

            //     println!("{}", (amount / 100_000_000) * 100 / 21_000_000);
             println!("{}", key_count * 100 / total_keys);
             }
        }
    }


// let mut batch = sled::Batch::default();
println!("writing");
let mut file = OpenOptions::new()
        .append(true) // Set the file to append mode
        .create(true) // Create the file if it doesn't exist
        .open("utxos").unwrap();
        #[derive(Debug)]
        struct P2WPKH {
            tx_id: [u8; 8],
            vout: u16,
            pub_key_hash: [u8; 20],
        }
let conn = Connection::open("utxos.sqlite").unwrap();
conn.execute(
    "CREATE TABLE p2pkwhs (
        transaction_id    BLOB,
        vout              INTEGER,
        pub_key_hash      BLOB,
        PRIMARY KEY (transaction_id, vout)
    )",
    (), // empty list of parameters.
).unwrap();
let len =   utxos.len() as u64;
let mut z = 0u64;    
for (k, v) in utxos.iter() {
    z += 1;
    conn.execute(
        "INSERT INTO p2pkwhs (transaction_id, vout, pub_key_hash) VALUES (?1, ?2, ?3)",
        (&k.0, &k.1, v),
    ).unwrap();
    if(z %10000u64 == 0) {
        println!("{}", z*100/len)
    }
    // println!("{}", hex::encode(&[k.to_vec(),v.to_vec()].concat()));
    // file.write_all(&[k.to_vec(),v.to_vec()].concat());
}

// println!("applying batch");
// sled_db.apply_batch(batch).unwrap();
    // println!("{} segwit_total", segwit_total);
    // println!("{} segwit_count", segwit_count);
    // println!("{} balances", balances.len());
    println!("{} keys scaned", key_count);
    println!("{} utxos exported", utxo_count);
    println!("{} max height", max_height);
    println!("{} max script length", max_script_length);
    println!("{} max vout", max_vout);
    println!("Total value: {}", amount);
}





//https://github.com/bitcoin/bitcoin/blob/4cc99df44aec4d104590aee46cf18318e22a8568/src/serialize.h#L464-L484

fn read_varint<R>(reader: &mut R) -> io::Result<u64>
where
    R: Read
{
    let mut n = u64::from(0u8);
    loop {
        let mut buffer = [0; 1];
        reader.read_exact(&mut buffer)?;
        let ch_data = u64::from(buffer[0]);
        n = (n << 7) | (ch_data & u64::from(0x7Fu8));
        if ch_data & u64::from(0x80u8) != u64::from(0u8) {
            if n == u64::MAX {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "ReadVarInt: size too large"));
            }
            n = n + 1;
        } else {
            return Ok(n);
        }
    }
}
// https://github.com/bitcoin/bitcoin/blob/4cc99df44aec4d104590aee46cf18318e22a8568/src/compressor.cpp#L168-L192
fn decompress_amount(mut x: u64) -> u64 {
    if x == 0 {
        return 0;
    }
    x -= 1;
    
    let mut e = x % 10;
    x /= 10;

    let mut n = 0u64;

    if e < 9 {
        let d = (x % 9) + 1;
        x /= 9;
        n = x * 10 + d as u64;
    } else {
        n = x + 1;
    }

    while e > 0 {
        n *= 10;
        e -= 1;
    }

    n
}
fn main() {
    let mut db = DB::open(env::args().nth(1).unwrap(), Default::default()).unwrap();
    let mut obfuscation_key = get_obfuscation_key(&mut db);
    dump_utxos(&mut db, &obfuscation_key);
    // let db: sled::Db = sled::open("utxos").unwrap();
    // let tx_id = hex!("d176d4960a78b41971f9d19207b59af6584b16ef323de55e983aec0100000000");
    // let mut iter = db.iter();
    // i.next().unwrap();
    // i.next().unwrap();
    // println!("{:?}", hex::encode(db.get([
    //     tx_id[..8].to_vec(),
    //     [0 as u8].to_vec()
    // ].concat()).unwrap().unwrap()));
}
