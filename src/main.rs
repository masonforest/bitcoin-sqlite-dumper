use indicatif::{ProgressBar, ProgressStyle};
use itertools::Itertools;
use leveldb::database::key::Key;
use leveldb::{
    database::Database,
    iterator::Iterable,
    options::{Options, ReadOptions},
};
use rusqlite::Connection;
use std::{
    env, fs,
    io::{self, Cursor, Read, Seek, SeekFrom},
    path::Path,
    time::Duration,
};
use walkdir::WalkDir;

const OBFUSCATION_KEY: u8 = 14;
const UTXO_KEY: u8 = 67;
const BATCH_SIZE: usize = 10000;

// https://github.com/bitcoin/bitcoin/blob/4cc99df44aec4d104590aee46cf18318e22a8568/src/dbwrapper.cpp#L323C19-L323C73
// length + 0 + key

const OBFUSCATE_KEY_KEY: [u8; 15] = *b"\x0E\0obfuscate_key";

#[derive(Debug, Default)]
struct Utxo {
    tx_id: [u8; 32],
    vout: u16,
    coinbase: bool,
    height: u64,
    amount: u64,
    compressed_script: Vec<u8>,
}
impl Utxo {
    fn decode(tx_id: [u8; 32], vout: u16, value: Vec<u8>) -> Utxo {
        let mut cursor = Cursor::new(value.clone());
        let height_and_coinbase: u64 = read_varint(&mut cursor).unwrap();
        let height = height_and_coinbase >> 1;
        let coinbase = (height_and_coinbase & 1) != 0;
        let amount = decompress_amount(read_varint(&mut cursor).unwrap());
        let mut compressed_script = Vec::new();
        let _ = cursor.read_to_end(&mut compressed_script);

        Utxo {
            tx_id,
            vout,
            coinbase,
            height,
            amount,
            compressed_script,
        }
    }
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
struct BtcDb<'a> {
    iter: &'a mut leveldb::database::iterator::Iterator<'a, BtcKey>,
    obfuscate_key: [u8; 8],
}

impl<'a> BtcDb<'a> {
    fn new(iter: &'a mut leveldb::database::iterator::Iterator<'a, BtcKey>) -> Self {
        if let Some((BtcKey(KeyType::ObfuscationKey(OBFUSCATE_KEY_KEY)), obfuscate_key)) =
            iter.next()
        {
            Self {
                iter,
                obfuscate_key: obfuscate_key[1..]
                    .try_into()
                    .expect("Obfuscate key was wrong length"),
            }
        } else {
            panic!("Couldn't read obfuscate_key")
        }
    }

    fn deobfuscate(&self, value: Vec<u8>) -> Vec<u8> {
        value
            .bytes()
            .enumerate()
            .map(|(index, byte)| {
                byte.unwrap() ^ (self.obfuscate_key[index % self.obfuscate_key.len()])
            })
            .collect()
    }
}
impl Iterator for BtcDb<'_> {
    type Item = Utxo;
    fn next(&mut self) -> Option<Self::Item> {
        while let Some((key, value)) = self.iter.next() {
            if let BtcKey(KeyType::UtxoKey(tx_id, vout)) = key {
                return Some(Utxo::decode(tx_id, vout, self.deobfuscate(value)));
            }
        }

        None
    }
}

fn create_sqlite_db(conn: &Connection) {
    conn.execute(
        "CREATE TABLE utxos (
        transaction_id    BLOB,
        vout              INTEGER,
        coinbase          BOOL,
        height            INTEGER,
        amount            INTEGER,
        n_size            INTEGER,
        compressed_script      BLOB,
        PRIMARY KEY (transaction_id, vout)
    )",
        (),
    )
    .unwrap();
}

fn insert_utxo(conn: &Connection, utxo: &Utxo) {
    if let Err(e) = conn.execute(
        "INSERT INTO utxos (
                transaction_id,
                vout,
                coinbase,
                height,
                amount,
                compressed_script
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        (
            utxo.tx_id,
            utxo.vout,
            utxo.coinbase,
            utxo.height,
            utxo.amount,
            &utxo.compressed_script,
        ),
    ) {
        println!("Error executing insert: {:?}\nUTXO: {:?}", e, utxo)
    };
}

#[derive(Clone, Debug)]
enum KeyType {
    ObfuscationKey([u8; 15]),
    UtxoKey([u8; 32], u16),
    UnknownKey,
}

#[derive(Clone, Debug)]
struct BtcKey(KeyType);

impl Key for BtcKey {
    fn from_u8(inner: &[u8]) -> Self {
        match inner[0] {
            OBFUSCATION_KEY => {
                Self(KeyType::ObfuscationKey(inner.try_into().unwrap()))
            },

            UTXO_KEY => {
                let tx_id: [u8; 32] = inner[1..33].try_into().unwrap();
                // tx_id.reverse();
                let mut cursor = Cursor::new(inner[33..].to_vec().clone());
                let vout: u16 = read_varint(&mut cursor).unwrap().try_into().unwrap();
                Self(KeyType::UtxoKey(tx_id, vout))
            }

            _ => Self(KeyType::UnknownKey),
        }
    }

    fn as_slice<T, F: Fn(&[u8]) -> T>(&self, f: F) -> T {
        if let KeyType::ObfuscationKey(bytes) = self.0 {
            f(&bytes)
        } else {
            panic!("can't encode key {:?}", self.0)
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

fn main() {
    let input_path = env::args().nth(1).unwrap();
    let output_path = env::args().nth(2).unwrap_or("utxos.sqlite".into());
    let db: Database<BtcKey> = Database::open(Path::new(&input_path), Options::new()).unwrap();
    let mut iter = db.iter(ReadOptions::new());
    let mut btcdb = BtcDb::new(&mut iter);
    let size_hint = get_size_hint(input_path);
    let bar = ProgressBar::new(size_hint);
    let conn = Connection::open(output_path).unwrap();
    bar.enable_steady_tick(Duration::from_millis(100));
    bar.set_style(
        ProgressStyle::with_template(
            "[{elapsed}/~{duration}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
        )
        .unwrap()
        .progress_chars("##-"),
    );
    create_sqlite_db(&conn);
    for chunk in &btcdb.chunks(BATCH_SIZE) {
        bar.inc(BATCH_SIZE.try_into().unwrap());
        conn.execute("BEGIN TRANSACTION;", ()).unwrap();
        for utxo in chunk {
            insert_utxo(&conn, &utxo)
        }
        conn.execute("COMMIT;", ()).unwrap();
    }
}
