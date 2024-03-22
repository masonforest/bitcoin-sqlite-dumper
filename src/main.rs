use futures::stream::{self, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use leveldb::database::key::Key;
use leveldb::{
    database::Database,
    iterator::Iterable,
    kv::KV,
    options::{Options, ReadOptions},
};
use rusqlite::Connection;
use std::{
    env, fs,
    io::{self, Cursor, Read, Seek, SeekFrom},
    path::Path,
    time::Duration,
};
use tokio::sync::{
    mpsc,
    mpsc::{Receiver, Sender},
};
use tokio_stream::wrappers::ReceiverStream;
use walkdir::WalkDir;

const OBFUSCATION_KEY: u8 = 14;
const UTXO_KEY: u8 = 67;
const BATCH_SIZE: usize = 10000;


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
    tx_id: [u8; 32],
    vout: u16,
    coinbase: bool,
    height: u64,
    amount: u64,
    script_type: u64,
    script: Vec<u8>,
}
impl Utxo {
    fn decode(key: BtcKey, value: Vec<u8>) -> Utxo {
        let (tx_id, vout) = if let KeyType::UtxoKey(tx_id, vout) = key.inner {
            (tx_id, vout)
        } else {
            panic!("Can't decode {:?}", key.inner)
        };
        let mut cursor = Cursor::new(value.clone());
        let height_and_coinbase: u64 = read_varint(&mut cursor).unwrap();
        let height = height_and_coinbase >> 1;
        let coinbase = (height_and_coinbase & 1) != 0;
        let amount = decompress_amount(read_varint(&mut cursor).unwrap());
        let script_type = read_varint(&mut cursor).unwrap();
        if script_type != 0 {
            cursor.seek(SeekFrom::Current(-1)).unwrap();
        }
        let mut script = Vec::new();
        let _ = cursor.read_to_end(&mut script);

        Utxo {
            tx_id,
            vout,
            coinbase,
            height,
            amount,
            script_type,
            script,
        }
    }
}
fn get_obfuscation_key(db: &mut Database<BtcKey>) -> Vec<u8> {
    let read_opts = ReadOptions::new();
    db.get(
        read_opts,
        &BtcKey {
            inner: KeyType::ObfuscationKey(OBFUSCATE_KEY_KEY),
        },
    )
    .expect("obfuscation_key not set")
    .unwrap()[1..]
        .into()
}

//https://github.com/bitcoin/bitcoin/blob/4cc99df44aec4d104590aee46cf18318e22a8568/src/serialize.h#L464-L484

fn read_varint<R>(reader: &mut R) -> io::Result<u64>
where
    R: Read,
{
    let mut n = u64::from(0u8);
    loop {
        let mut buffer = [0; 1];
        reader.read_exact(&mut buffer)?;
        let ch_data = u64::from(buffer[0]);
        n = (n << 7) | (ch_data & u64::from(0x7Fu8));
        if ch_data & u64::from(0x80u8) != u64::from(0u8) {
            if n == u64::MAX {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "ReadVarInt: size too large",
                ));
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

    let mut n;

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

fn read_leveldb(db: &mut Database<BtcKey>, obfuscation_key: &[u8], sender: Sender<Utxo>) {
    let mut iter = db.iter(ReadOptions::new());
    let mut i = 0;
    while let Some((k, obfuscated_value)) = iter.next() {
        if let KeyType::UtxoKey(_, _) = k.inner {
            i += 1;
            let value = deobfuscate_value(obfuscated_value, obfuscation_key);
            let new_sender = sender.clone();
            tokio::spawn(async move {
                new_sender.send(Utxo::decode(k, value)).await;
            });
            // println!("{}", i);
        }
    }
}

async fn write_sqlite(conn: Connection, mut receiver: Receiver<Utxo>, size_hint: u64) {
    conn.execute(
        "CREATE TABLE utxos (
        transaction_id    BLOB,
        vout              INTEGER,
        coinbase          BOOL,
        height            INTEGER,
        amount            INTEGER,
        script_type       INTEGER,
        compressed_script      BLOB,
        PRIMARY KEY (transaction_id, vout)
    )",
        (),
    )
    .unwrap();
    let bar = ProgressBar::new(size_hint);
    bar.enable_steady_tick(Duration::from_millis(100));
    bar.set_style(
        ProgressStyle::with_template(
            "[{elapsed}/~{duration}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
        )
        .unwrap()
        .progress_chars("##-"),
    );
    let mut i = 0;
    conn.execute("BEGIN TRANSACTION;", ());
    let mut stream = ReceiverStream::new(receiver).chunks(BATCH_SIZE);
    while let Some(chunk) = stream.next().await {
        bar.inc(BATCH_SIZE.try_into().unwrap());
        for utxo in chunk {
            {
                let Utxo {
                    tx_id,
                    vout,
                    coinbase,
                    height,
                    amount,
                    script_type,
                    script,
                } = utxo;
                i += 1;
                conn.execute(
                    "INSERT INTO utxos (
                transaction_id,
                vout,
                coinbase,
                height,
                amount,
                script_type,
                compressed_script
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    (tx_id, vout, coinbase, height, amount, script_type, script),
                )
                .unwrap();
                // if (i % SQL_BATCH_SIZE == 0) {
                // }
            }
        }
                    conn.execute("COMMIT;", ());
                    conn.execute("BEGIN TRANSACTION;", ());
    }
    conn.execute("COMMIT;", ());
    bar.finish();
}

#[derive(Clone, Debug)]
enum KeyType {
    ObfuscationKey([u8; 15]),
    UtxoKey([u8; 32], u16),
    UnknownKey,
}

#[derive(Clone, Debug)]
struct BtcKey {
    inner: KeyType,
}

impl Key for BtcKey {
    fn from_u8(inner: &[u8]) -> Self {
        match inner[0] {
            OBFUSCATION_KEY => Self {
                inner: KeyType::ObfuscationKey(inner.try_into().unwrap()),
            },
            UTXO_KEY => Self {
                inner: {
                    let mut tx_id: [u8; 32] = inner[1..33].try_into().unwrap();
                    // tx_id.reverse();
                    let mut cursor = Cursor::new(inner[33..].to_vec().clone());
                    let vout: u16 = read_varint(&mut cursor).unwrap().try_into().unwrap();
                    KeyType::UtxoKey(tx_id, vout)
                },
            },
            _ => Self {
                inner: KeyType::UnknownKey,
            },
        }
    }

    fn as_slice<T, F: Fn(&[u8]) -> T>(&self, f: F) -> T {
        if let KeyType::ObfuscationKey(bytes) = self.inner {
            f(&bytes)
        } else {
            panic!("can't encode key {:?}", self.inner)
        }
    }
}
fn get_size_hint<P: AsRef<Path>>(path: P) -> u64 {
    get_directory_size(path).unwrap() / 67 // each record is about 67 bytes
}
fn get_directory_size<P: AsRef<Path>>(path: P) -> std::io::Result<u64> {
    let mut size = 0;

    for entry in WalkDir::new(path) {
        let entry = entry?;
        let metadata = fs::metadata(entry.path())?;

        if metadata.is_file() {
            size += metadata.len();
        }
    }

    Ok(size)
}

#[tokio::main]
async fn main() {
    let input_path = env::args().nth(1).unwrap();
    let output_path = env::args().nth(2).unwrap_or("utxos.sqlite".into());
    let conn = Connection::open(output_path).unwrap();
    let mut db: Database<BtcKey> = Database::open(Path::new(&input_path), Options::new()).unwrap();
    let size_hint = get_size_hint(input_path);
    let obfuscation_key = get_obfuscation_key(&mut db);
    let (tx, rx) = mpsc::channel::<Utxo>(BATCH_SIZE);
    tokio::task::spawn_blocking(move ||{ read_leveldb(&mut db, &obfuscation_key, tx) });

    write_sqlite(conn, rx, size_hint).await
}
