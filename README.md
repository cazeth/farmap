# Farmap

A WIP analysis tool for [farcaster label datasets](https://github.com/farcasterxyz/labels), in particular [Warpcast spam labels](https://github.com/warpcast/labels/). Provides both a library with some tools to use in other projects and a simple CLI tool to explore the data directly.

## Get Started

Only tested on Linux so far, but it's probably not hard to make it run on other systems.

It's easy to get started. You need to get data and you need to get program:

### get data

Use the script in the scripts dir to setup the data. This will create several files with jsonl data to have the full history of the spam labels, not only the most recent labels for each fid.

```bash
git clone https://github.com/cazeth/farmap.git
cp farmap
bash scripts/data-setup.sh
```

You can also clone the data directly from the Warpcast spam labels repo (it's easiest if you store the data in ~/.local/share/farmap but anywhere is fine):

```bash
git clone https://github.com/warpcast/labels.git $HOME/.local/share/farmap
```

This should work but the program does not get as much history as it could have since the program only has the latest labels rather than the entire history. It is easiest to use the script to get full history but luddites can achieve the same result manually by running git checkout on each commit in the git repo and copying the spam.jsonl at that commit to a file called spam-{DATE}.jsonl. The names do not matter as long as the files are unique .jsonl files and in the same directory.

### get program

```bash
git clone https://github.com/cazeth/farmap.git
cp farmap
```

You can now run the program directly...

```bash
cargo run -- --help
```

...or build it and copy it to somewhere on PATH:

```bash
cargo build --release
cp ./target/release/farmap ~/.local/bin/  #.local/bin can be any dir that is on PATH.
```

Now (if you stored the data in the default directory as mentioned above) run the program with

```bash
farmap
```

otherwise you must point the program to your data directory:

```bash
farmap -p {data-dir}
```

To view other options, run

```bash
farmap -h
```
