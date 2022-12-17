use super::{lsb, pop, consts::*, movegen::{bishop_attacks, rook_attacks}, zobrist::ZVALS};

/// The position is stored as global state.
pub static mut POS: Position = Position {
    pieces: [0; 6], sides: [0; 2], squares: [EMPTY as u8; 64], side_to_move: 0,
    state: GameState { zobrist: 0, phase: 0, mg: 0, eg: 0, en_passant_sq: 0, halfmove_clock: 0, castle_rights: 0 },
    nulls: 0, stack: Vec::new()
};

#[macro_export]
macro_rules! from {($m:expr) => {(($m >> 6) & 63) as usize}}

#[macro_export]
macro_rules! to {($m:expr) => {($m & 63) as usize}}

#[macro_export]
macro_rules! bit {($x:expr) => {1 << $x}}

pub struct Position {
    pub pieces: [u64; 6],
    pub sides: [u64; 2],
    pub squares: [u8; 64],
    pub side_to_move: usize,
    pub state: GameState,
    pub nulls: u8,
    pub stack: Vec<MoveState>,
}

#[derive(Clone, Copy, Default)]
pub struct GameState {
    pub zobrist: u64,
    pub phase: i16,
    pub mg: i16,
    pub eg: i16,
    pub en_passant_sq: u16,
    pub halfmove_clock: u8,
    pub castle_rights: u8,
}

#[derive(Clone, Copy)]
pub struct MoveState {
    state: GameState,
    m: u16,
    moved_pc: u8,
    captured_pc: u8,
}

pub struct MoveList {
    pub list: [u16; 256],
    pub len: usize,
}

impl Default for MoveList {
    fn default() -> Self {
        Self {list: unsafe {#[allow(clippy::uninit_assumed_init, invalid_value)] std::mem::MaybeUninit::uninit().assume_init()}, len: 0}
    }
}

impl MoveList {
    #[inline(always)]
    pub fn push(&mut self, m: u16) {
        self.list[self.len] = m;
        self.len += 1;
    }
}
impl Position {
    #[inline(always)]
    pub fn is_square_attacked(&self, idx: usize, side: usize, occ: u64) -> bool {
        let other: usize = side ^ 1;
        let s: u64 = self.sides[other];
        let opp_queen: u64 = self.pieces[QUEEN] & s;
        (KNIGHT_ATTACKS[idx] & self.pieces[KNIGHT] & s > 0)
        || (KING_ATTACKS[idx] & self.pieces[KING] & s > 0)
        || (PAWN_ATTACKS[side][idx] & self.pieces[PAWN] & s > 0)
        || (rook_attacks(idx, occ) & (self.pieces[ROOK] & s | opp_queen) > 0)
        || (bishop_attacks(idx, occ) & (self.pieces[BISHOP] & s | opp_queen) > 0)
    }

    #[inline(always)]
    pub fn is_in_check(&self) -> bool {
        let king_idx: usize = lsb!(self.pieces[KING] & self.sides[self.side_to_move]) as usize;
        self.is_square_attacked(king_idx, self.side_to_move, self.sides[0] | self.sides[1])
    }

    #[inline(always)]
    fn toggle(&mut self, side: usize, piece: usize, bit: u64) {
        self.pieces[piece] ^= bit;
        self.sides[side] ^= bit
    }

    #[inline(always)]
    fn add(&mut self, from: usize, side: usize, piece: usize) {
        let indx = from ^ (56 * (side == 0) as usize);
        self.state.zobrist ^= unsafe{ZVALS.pieces[side][piece][from]};
        self.state.mg += SIDE_FACTOR[side] * PST_MG[piece][indx];
        self.state.eg += SIDE_FACTOR[side] * PST_EG[piece][indx];
    }

    #[inline(always)]
    fn remove(&mut self, from: usize, side: usize, piece: usize) {
        let indx = from ^ (56 * (side == 0) as usize);
        self.state.zobrist ^= unsafe{ZVALS.pieces[side][piece][from]};
        self.state.mg -= SIDE_FACTOR[side] * PST_MG[piece][indx];
        self.state.eg -= SIDE_FACTOR[side] * PST_EG[piece][indx];
    }

    pub fn do_move(&mut self, m: u16) -> bool {
        unsafe {
        let opp: usize = self.side_to_move ^ 1;

        // move data
        let from: usize = from!(m);
        let to: usize = to!(m);
        let f: u64 = bit!(from);
        let t: u64 = bit!(to);
        let moved_pc: u8 = self.squares[from];
        let captured_pc: u8 = self.squares[to];
        let flag: u16 = m & MoveFlags::ALL;
        let rights: u8 = self.state.castle_rights;

        // initial updates
        self.stack.push(MoveState { state: self.state, m, moved_pc, captured_pc});
        self.toggle(self.side_to_move, moved_pc as usize, f | t);
        self.remove(from, self.side_to_move, moved_pc as usize);
        self.add(to, self.side_to_move, moved_pc as usize);
        self.squares[from] = EMPTY as u8;
        self.squares[to] = moved_pc;
        if self.state.en_passant_sq > 0 {self.state.zobrist ^= ZVALS.en_passant[(self.state.en_passant_sq & 7) as usize]}
        self.state.en_passant_sq = 0;
        self.state.zobrist ^= ZVALS.side;

        // captures
        if captured_pc != EMPTY as u8 {
            let cpc: usize = captured_pc as usize;
            self.toggle(opp, cpc, t);
            self.remove(to, opp, cpc);
            self.state.phase -= PHASE_VALS[cpc];
            if captured_pc == ROOK as u8 {
                self.state.castle_rights &= CASTLE_RIGHTS[to];
            }
        }

        if moved_pc == KING as u8 || moved_pc == ROOK as u8 {self.state.castle_rights &= CASTLE_RIGHTS[from]}

        // piece-specific updates
        match flag {
            MoveFlags::EN_PASSANT => {
                let pwn: usize = if opp == WHITE {to + 8} else {to - 8};
                let p: u64 = bit!(pwn);
                self.toggle(opp, PAWN, p);
                self.remove(pwn, opp, PAWN);
                self.squares[pwn] = EMPTY as u8;
            }
            MoveFlags::DBL_PUSH => {
                self.state.en_passant_sq = if opp == BLACK {to - 8} else {to + 8} as u16;
                self.state.zobrist ^= ZVALS.en_passant[to & 7];
            }
            MoveFlags::KNIGHT_PROMO => {
                let ppc: usize = (((flag >> 12) & 3) + 1) as usize;
                self.pieces[moved_pc as usize] ^= t;
                self.pieces[ppc] ^= t;
                self.squares[to] = ppc as u8;
                self.state.phase += PHASE_VALS[ppc];
                self.remove(to, self.side_to_move, moved_pc as usize);
                self.add(to, self.side_to_move, ppc);
            }
            MoveFlags::KS_CASTLE | MoveFlags::QS_CASTLE => {
                let (c, idx1, idx2): (u64, usize, usize) = CASTLE_MOVES[self.side_to_move][(flag == MoveFlags::KS_CASTLE) as usize];
                self.squares.swap(idx1, idx2);
                self.toggle(self.side_to_move, ROOK, c);
                self.remove(idx1, self.side_to_move, ROOK);
                self.add(idx2, self.side_to_move, ROOK);
            }
            _ => {}
        }

        // castle hashes
        let mut changed_castle: u8 = rights & !self.state.castle_rights;
        while changed_castle > 0 {
            let ls1b: u8 = changed_castle & changed_castle.wrapping_neg();
            self.state.zobrist ^= ZVALS.castle_hash(rights, ls1b);
            pop!(changed_castle)
        }

        // final updates
        self.state.halfmove_clock = (moved_pc > PAWN as u8 && flag != MoveFlags::CAPTURE) as u8 * (self.state.halfmove_clock + 1);
        self.side_to_move ^= 1;

        // is legal?
        let king_idx: usize = lsb!(self.pieces[KING] & self.sides[opp ^ 1]) as usize;
        let invalid: bool = self.is_square_attacked(king_idx, opp ^ 1, self.sides[0] | self.sides[1]);
        if invalid { self.undo_move() }
        invalid
        }
    }

    pub fn undo_move(&mut self) {
        let opp: usize = self.side_to_move;

        // restore state
        let state: MoveState = self.stack.pop().unwrap();

        // move data
        let moved_pc: u8 = state.moved_pc;
        let captured_pc: u8 = state.captured_pc;
        let from: usize = from!(state.m);
        let to: usize = to!(state.m);
        let f: u64 = bit!(from);
        let t: u64 = bit!(to);
        let flag: u16 = state.m & MoveFlags::ALL;

        // updates
        self.side_to_move ^= 1;
        self.state = state.state;
        self.toggle(self.side_to_move, moved_pc as usize, f | t);
        self.squares[from] = moved_pc;
        self.squares[to] = captured_pc;
        if captured_pc != EMPTY as u8 {self.toggle(opp, captured_pc as usize, t);}
        match flag {
            MoveFlags::EN_PASSANT => {
                let pwn: usize = if opp == WHITE {to + 8} else {to - 8};
                let p: u64 = bit!(pwn);
                self.toggle(opp, PAWN, p);
                self.squares[pwn] = PAWN as u8;
            }
            MoveFlags::KNIGHT_PROMO => {
                let promo_pc: u16 = ((flag >> 12) & 3) + 1;
                self.pieces[moved_pc as usize] ^= t;
                self.pieces[promo_pc as usize] ^= t;
            }
            MoveFlags::KS_CASTLE | MoveFlags::QS_CASTLE => {
                let (c, idx1, idx2): (u64, usize, usize) = CASTLE_MOVES[self.side_to_move][(flag == MoveFlags::KS_CASTLE) as usize];
                self.squares.swap(idx1, idx2);
                self.toggle(self.side_to_move, ROOK, c);
            }
            _ => {}
        }
    }

    pub fn do_null(&mut self) -> (u16, u64) {
        self.nulls += 1;
        let enp: u16 = self.state.en_passant_sq;
        let hash: u64 = self.state.zobrist;
        self.state.zobrist ^= (enp > 0) as u64 * unsafe{ZVALS.en_passant[(enp & 7) as usize]};
        self.state.en_passant_sq = 0;
        self.side_to_move ^= 1;
        self.state.zobrist ^= unsafe{ZVALS.side};
        (enp, hash)
    }

    pub fn undo_null(&mut self, (enp, hash): (u16, u64)) {
        self.nulls -= 1;
        self.state.zobrist = hash;
        self.state.en_passant_sq = enp;
        self.side_to_move ^= 1;
    }

    pub fn is_draw_by_repetition(&self, num: u8) -> bool {
        let l: usize = self.stack.len();
        if l < 6 || self.nulls > 0 { return false }
        let to: usize = l - 1;
        let mut from: usize = l.wrapping_sub(self.state.halfmove_clock as usize);
        if from > 1024 { from = 0 }
        let mut repetitions_count: u8 = 1;
        for i in (from..to).rev().step_by(2) {
            if self.stack[i].state.zobrist == self.state.zobrist {
                repetitions_count += 1;
                if repetitions_count >= num { return true }
            }
        }
        false
    }

    #[inline(always)]
    pub fn is_draw_by_50(&self) -> bool {
        self.state.halfmove_clock >= 100
    }

    /// Is there a FIDE draw by insufficient material?
    ///  - KvK
    ///  - KvKN or KvKB
    ///  - KBvKB and both bishops the same colour
    pub fn is_draw_by_material(&self) -> bool {
        let pawns: u64 = self.pieces[PAWN];
        if pawns == 0 && self.state.phase <= 2 {
            if self.state.phase == 2 {
                let bishops: u64 = self.pieces[BISHOP];
                return bishops & self.sides[0] != bishops && bishops & self.sides[1] != bishops && (bishops & 0x55AA55AA55AA55AA == bishops || bishops & 0xAA55AA55AA55AA55 == bishops)
            }
            return true
        }
        false
    }

    /// Calculates the midgame and endgame piece-square table evaluations and the game
    /// phase of the current position from scratch.
    pub fn calc(&self) -> (i16, i16, i16) {
        let mut res: (i16, i16, i16) = (0,0,0);
        for (i, side) in self.sides.iter().enumerate() {
            let factor: i16 = SIDE_FACTOR[i];
            for j in 0..6 {
                let mut pcs: u64 = self.pieces[j] & side;
                let count: i16 = pcs.count_ones() as i16;
                res.0 += PHASE_VALS[j] * count;
                while pcs > 0 {
                    let idx: usize = lsb!(pcs) as usize;
                    let white: usize = (i == 0) as usize * 56;
                    res.1 += factor * PST_MG[j][idx ^ white];
                    res.2 += factor * PST_EG[j][idx ^ white];
                    pop!(pcs);
                }
            }
        }
        res
    }
}
