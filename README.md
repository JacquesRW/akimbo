# akimbo

akimbo is a UCI compatible chess engine written in Rust.

### TODO (before first release)
- Tuner (in separate repo)
- FRC support
- Syzygy tablebase support (maybe)

### Aims
The main aim of akimbo is to stay under 1500 lines of code.
At last count it was at 1155 lines, excluding blank lines and comments.

### Compiling
If you have cargo installed, run `cargo build --release`.

## Features

#### Move Generation
- Bitboards
- Pseudo-legal
- Hyperbola quintessence sliding attacks

#### Search
- Fail-soft
- Principle variation search
- Quiescence search
- Iterative deepening
- Check extensions

#### Move Ordering
1. Hash move
2. Captures (MVV-LVA)
3. Promotions
4. Killer moves
5. Quiets

#### Evaluation
- Tapered piece-square tables

#### Pruning/Reductions
- Mate distance pruning
- Hash score pruning
- Late move reductions
- Reverse futility pruning
- Null move pruning
- Delta pruning
