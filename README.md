Bitcoin SQLlite Dumper
======================

Dumps the Bitcoin UTXO set to SQLite

Thanks to [in3rsha/bitcoin-utxo-dump](https://github.com/in3rsha/bitcoin-utxo-dump) where most of this code is based on.


Usage
=====

The easiest way to get a fully synced node is to search for the "Pruned Bitcoin Node" community AMI on AWS.

The recommended one instance type `c7g.medium`. I used a `t4g.2xlarge` so it would sync faster.

It seems like they release new imagine biweeekly.

Once you;ve got your server up and running til the log to check when the chain has been synced: `tail -f /var/lib/bitcoin/debug.log`

Once it's synced you can shut down `bitcoind`: `sudo killall bitcoind`


Next clone this repo: `git clone https://github.com/masonforest/bitcoin-sqlite-dumper && cd bitcoin-sqlite-dumper`
Install dependencies: `sudo apt install cmake gcc g++ libleveldb-dev libsnappy-dev`

Build: `cargo build --release`
And run: `sudo ./target/release/utxo-dumper  /var/lib/bitcoin/chainstate`

Dumping took about half an hour on the `t4g.2xlarge`

The whole process should take about an hour and cost under a buck.