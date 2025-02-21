# Farmap

A WIP analysis tool for [farcaster label datasets](https://github.com/farcasterxyz/labels), in particular [Warpcast spam labels](https://github.com/warpcast/labels/). Provides both a library with some tools to use in other projects and a simple CLI tool to explore the data directly.

## Get Started

Only tested on Linux so far, but it's probably not hard to make it run on other systems.

It's easy to get started. You need to get data and you need to get program:

### get data

Use the script in the scripts dir to setup the data. This will create several files with jsonl data to have the full history of the spam labels, not only the most recent labels for each fid.

```bash
git clone https://github.com/cazeth/farmap.git
cd farmap
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
cd farmap
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

### Examples

```bash

# print spam distribution at today's date
farmap

# print spam distribution at 2025-01-01 of all users that had a spam score at that date
farmap spam-distribution -d 2025-01-01

# print spam distribution at 2025-02-20 of users who
# got their first spam score between 7th and 14th of february
farmap -a 2025-02-07 -b 2025-02-14 spam-distribution -d 2025-02-20

# print spam distribution at 2025-02-14 of users who
# got their first spam score in january and have a latest spam score of 2
farmap -a 2025-01-01 -b 2025-01-31 -c 2 spam-distribution -d 2025-02-14

# print a matrix of the spam score changes betwwen 1st and 20th of february,
# where row represents the spam score at the 1st and the column the spam score at the 20th
# Do this only for the users created in january
farmap -a 2025-01-01 -b 2025-01-31 change-matrix --from-date 2025-02-01 --to-date 2025-02-20

# print all fids to output.txt that got their first spam score
# between 14th and 21st of february and have a latest spam score of 2
farmap -a 2025-02-14 -b 2025-02-21 -c 2 all-fids > output.txt

# print all spam records for user with fid 11720 (caz.eth)
farmap fid -f 11720

```
