# Massa slot replayer

Block replay tool for Massa blockchain (for debugging, testing & performance checking)

# Dev

## Help

* cargo run -- --help
* cargo run -- replay  --help

## List snapshots

* cargo run -- --path /tmp/massa_8_g4j_3n/massa-node/storage/ledger/rocks_db/ --initial_roll_path /tmp/compile_massa_5tm8z0am/massa-node/base_config/initial_rolls.json list-snapshot

## Replay blocks

* cargo run -- --path /tmp/massa_8_g4j_3n/massa-node/storage/ledger/rocks_db/ --initial_roll_path /tmp/compile_massa_5tm8z0am/massa-node/base_config/initial_rolls.json replay -b /tmp/massa_8_g4j_3n/massa-node/dump/blocks/ --backup /tmp/massa_8_g4j_3n/massa-node/storage/ledger/rocks_db/backup_40_0/