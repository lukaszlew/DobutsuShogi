//! wasm-bindgen surface. Thin pass-through to `GameLogic`.
//!
//! All cross-boundary moves and board bytes are in **human-display
//! coordinates** — the JS layer never sees the side-relative encoding.

use crate::game::GameLogic;
use crate::rules::Outcome as RulesOutcome;
use crate::search::Coefs;
use serde::{Deserialize, Serialize};
use tsify_next::Tsify;
use wasm_bindgen::prelude::*;

#[derive(Tsify, Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub enum Outcome {
    Ongoing,
    LionCaptured,
    Try,
}

impl From<RulesOutcome> for Outcome {
    fn from(o: RulesOutcome) -> Self {
        match o {
            RulesOutcome::Ongoing => Outcome::Ongoing,
            RulesOutcome::LionCaptured => Outcome::LionCaptured,
            RulesOutcome::Try => Outcome::Try,
        }
    }
}

#[derive(Tsify, Serialize, Deserialize, Copy, Clone, Debug)]
#[tsify(into_wasm_abi)]
pub struct HandCounts {
    pub chick: u8,
    pub elephant: u8,
    pub giraffe: u8,
}

#[derive(Tsify, Serialize, Deserialize, Copy, Clone, Debug)]
#[tsify(into_wasm_abi)]
pub struct AiMove {
    pub mv: u8,
    pub eval: i32,
}

/// AI search configuration sent from the UI: ply depth, material
/// coefficients used by the leaf evaluation, and a randomness magnitude
/// that jitters root-move scores during selection (0 = deterministic).
/// Lions are not represented — their capture is mate, scored separately.
#[derive(Tsify, Serialize, Deserialize, Copy, Clone, Debug)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct AiConfig {
    pub depth: u32,
    pub chick: i32,
    pub elephant: i32,
    pub giraffe: i32,
    pub hen: i32,
    pub randomness: i32,
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = Math)]
    fn random() -> f64;
}

/// Number of half-move depths reported in the move-log eval breakdown.
const EVAL_LOG_DEPTH: u32 = 10;

impl AiConfig {
    fn coefs(&self) -> Coefs {
        Coefs {
            chick: self.chick,
            elephant: self.elephant,
            giraffe: self.giraffe,
            hen: self.hen,
        }
    }
}

#[wasm_bindgen]
pub fn default_ai_config() -> AiConfig {
    let c = Coefs::DEFAULT;
    AiConfig {
        depth: 4,
        chick: c.chick,
        elephant: c.elephant,
        giraffe: c.giraffe,
        hen: c.hen,
        randomness: 2,
    }
}

#[wasm_bindgen]
pub struct Game {
    inner: GameLogic,
}

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new(human_plays_first: bool) -> Game {
        Game {
            inner: GameLogic::new(human_plays_first),
        }
    }

    pub fn legal_moves(&self) -> Vec<u8> {
        self.inner.legal_move_codes()
    }

    pub fn apply(&mut self, mv: u8) -> Outcome {
        self.inner.apply_code(mv).into()
    }

    pub fn board(&self) -> Vec<u8> {
        self.inner.board_bytes()
    }

    pub fn humans_turn(&self) -> bool {
        self.inner.humans_turn()
    }

    pub fn hand_human(&self) -> HandCounts {
        let (chick, elephant, giraffe) = self.inner.human_hand();
        HandCounts { chick, elephant, giraffe }
    }

    pub fn hand_ai(&self) -> HandCounts {
        let (chick, elephant, giraffe) = self.inner.ai_hand();
        HandCounts { chick, elephant, giraffe }
    }

    /// Search for the best move at the configured depth using the
    /// supplied evaluation coefficients. Returns `None` if no legal
    /// moves exist (terminal position).
    pub fn ai_move(&self, config: AiConfig) -> Option<AiMove> {
        let r = config.randomness.max(0);
        let mut jitter = || -> i32 {
            if r == 0 {
                0
            } else {
                // Uniform integer in [-r, +r].
                (random() * (2.0 * r as f64 + 1.0)) as i32 - r
            }
        };
        let (mv, eval) = self.inner.ai_search(&config.coefs(), config.depth, &mut jitter);
        mv.map(|mv| AiMove { mv, eval })
    }

    /// Deterministic evals at depths 1..=`EVAL_LOG_DEPTH` (10) using
    /// the supplied coefficients. Score at each depth is from the
    /// current side-to-move's perspective. Used by the move log.
    pub fn eval_log(&self, config: AiConfig) -> Vec<i32> {
        self.inner.eval_log(&config.coefs(), EVAL_LOG_DEPTH)
    }
}
