//! The cross-run meta: what a player has unlocked, and what a run earns them.
//!
//! This is pure, deterministic bookkeeping and knows nothing about storage, the shell
//! or the clock. The run rules never see it — the core only ever receives a
//! [`Loadout`], and this module is simply one way to build one. That keeps the whole
//! meta layer additive: it can grow without touching a single rule.
//!
//! A fresh player flies the base ship — always fully playable — and earns options by
//! playing: scores, cleared systems, an Orbit win, and wins at Ascension tiers.

use super::Loadout;

/// A ship option a player can unlock and fly with. Each folds into the run's starting
/// [`Loadout`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Content {
    /// Begin with the spread fan (weapon tier 1).
    Spread,
    /// Begin with a shield up.
    Shield,
    /// Begin with piercing fire (weapon tier 2).
    Pierce,
    /// Begin with an extra ship.
    Reserve,
    /// Begin with rapid fire (weapon tier 3).
    Rapid,
    /// Begin with the collapse meter full.
    Charged,
}

/// Every unlockable option — the order the collection screen shows them in, and the
/// order of their bits.
pub const ALL: [Content; 6] = [
    Content::Spread,
    Content::Shield,
    Content::Pierce,
    Content::Reserve,
    Content::Rapid,
    Content::Charged,
];

impl Content {
    /// This option's stable bit position in an [`Unlocked`] set.
    pub fn index(self) -> u32 {
        // `ALL` is the single source of truth for bit order.
        ALL.iter()
            .position(|c| *c == self)
            .expect("every Content is listed in ALL") as u32
    }

    /// A short name for the collection screen.
    pub fn label(self) -> &'static str {
        match self {
            Content::Spread => "SPREAD",
            Content::Shield => "SHIELD",
            Content::Pierce => "PIERCE",
            Content::Reserve => "RESERVE",
            Content::Rapid => "RAPID",
            Content::Charged => "CHARGED",
        }
    }

    /// How this option is earned, for the collection screen.
    pub fn condition(self) -> &'static str {
        match self {
            Content::Spread => "SCORE 400",
            Content::Shield => "CLEAR A SYSTEM",
            Content::Pierce => "SCORE 1500",
            Content::Reserve => "SCORE 3500",
            Content::Rapid => "WIN AN ORBIT RUN",
            Content::Charged => "WIN AT ASCENSION 1",
        }
    }

    /// Whether a run with this `outcome` earns this option.
    fn earned_by(self, outcome: Outcome) -> bool {
        match self {
            Content::Spread => outcome.score >= 400,
            Content::Shield => outcome.systems_cleared >= 1,
            Content::Pierce => outcome.score >= 1500,
            Content::Reserve => outcome.score >= 3500,
            Content::Rapid => outcome.won,
            Content::Charged => outcome.won && outcome.tier >= 1,
        }
    }
}

/// What a finished run achieved, as the meta reads it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// The run won — an Orbit run's final boss fell.
    pub won: bool,
    /// How many systems it cleared (bosses felled).
    pub systems_cleared: u32,
    /// Its score.
    pub score: u32,
    /// The Ascension tier it played at (0 for a plain run).
    pub tier: u32,
}

/// The options a player has unlocked, as a small bitset. Empty is a fresh player, who
/// flies the base ship.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Unlocked(u32);

impl Unlocked {
    /// Rebuilds a set from its saved bits, ignoring any bits outside known content.
    pub fn from_bits(bits: u32) -> Self {
        let known = ALL.iter().fold(0, |mask, c| mask | (1 << c.index()));
        Self(bits & known)
    }

    /// The bits to save.
    pub fn bits(self) -> u32 {
        self.0
    }

    /// Whether `content` is unlocked.
    pub fn has(self, content: Content) -> bool {
        self.0 & (1 << content.index()) != 0
    }

    /// Unlocks `content`, reporting whether it was newly earned.
    pub fn unlock(&mut self, content: Content) -> bool {
        let newly = !self.has(content);
        self.0 |= 1 << content.index();
        newly
    }

    /// Every unlocked option, in [`ALL`] order.
    pub fn iter(self) -> impl Iterator<Item = Content> {
        ALL.into_iter().filter(move |c| self.has(*c))
    }

    /// The starting loadout a run should fly: the base ship, stepped up by every
    /// unlocked option.
    pub fn loadout(self) -> Loadout {
        let mut loadout = Loadout::default();
        if self.has(Content::Spread) {
            loadout.start_weapon = loadout.start_weapon.max(1);
        }
        if self.has(Content::Pierce) {
            loadout.start_weapon = loadout.start_weapon.max(2);
        }
        if self.has(Content::Rapid) {
            loadout.start_weapon = loadout.start_weapon.max(3);
        }
        if self.has(Content::Shield) {
            loadout.shield = true;
        }
        if self.has(Content::Reserve) {
            loadout.bonus_lives += 1;
        }
        if self.has(Content::Charged) {
            loadout.start_charge = true;
        }
        loadout
    }

    /// Records a finished run, unlocking whatever it earned and returning only the
    /// newly-earned options — so the shell can announce them. Recording the same
    /// outcome again returns nothing.
    pub fn record(&mut self, outcome: Outcome) -> Vec<Content> {
        let mut newly = Vec::new();
        for content in ALL {
            if content.earned_by(outcome) && self.unlock(content) {
                newly.push(content);
            }
        }
        newly
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_option_has_a_unique_bit() {
        let mut seen = Vec::new();
        for content in ALL {
            let index = content.index();
            assert!(index < 32, "the set must fit in its bits");
            assert!(!seen.contains(&index), "{content:?} shares a bit");
            seen.push(index);
        }
        assert_eq!(seen.len(), ALL.len());
    }

    #[test]
    fn a_fresh_player_flies_the_base_loadout() {
        assert_eq!(Unlocked::default().loadout(), Loadout::default());
        assert_eq!(Unlocked::from_bits(0), Unlocked::default());
    }

    #[test]
    fn unlocked_options_build_the_loadout() {
        let mut unlocked = Unlocked::default();
        unlocked.unlock(Content::Pierce);
        unlocked.unlock(Content::Shield);
        unlocked.unlock(Content::Reserve);
        unlocked.unlock(Content::Charged);
        let loadout = unlocked.loadout();
        assert_eq!(
            loadout.start_weapon, 2,
            "pierce starts the weapon at tier 2"
        );
        assert!(loadout.shield, "shield starts up");
        assert_eq!(loadout.bonus_lives, 1, "reserve grants a ship");
        assert!(loadout.start_charge, "charged primes the collapse meter");
    }

    #[test]
    fn the_set_round_trips_through_its_bits() {
        let mut unlocked = Unlocked::default();
        unlocked.unlock(Content::Rapid);
        let restored = Unlocked::from_bits(unlocked.bits());
        assert_eq!(restored, unlocked);
        assert!(restored.has(Content::Rapid));
    }

    #[test]
    fn a_scoring_run_earns_the_score_options() {
        let mut unlocked = Unlocked::default();
        let newly = unlocked.record(Outcome {
            score: 400,
            ..Default::default()
        });
        assert_eq!(newly, vec![Content::Spread]);
        assert!(unlocked.has(Content::Spread));
        assert!(
            !unlocked.has(Content::Pierce),
            "the 1500 option stays locked"
        );
    }

    #[test]
    fn clearing_a_system_earns_the_shield() {
        let mut unlocked = Unlocked::default();
        let newly = unlocked.record(Outcome {
            systems_cleared: 1,
            ..Default::default()
        });
        assert!(newly.contains(&Content::Shield));
    }

    #[test]
    fn recording_the_same_outcome_twice_earns_nothing_new() {
        let mut unlocked = Unlocked::default();
        let outcome = Outcome {
            score: 1600,
            ..Default::default()
        };
        assert!(!unlocked.record(outcome).is_empty(), "the first run earns");
        assert!(
            unlocked.record(outcome).is_empty(),
            "nothing is earned twice"
        );
    }

    #[test]
    fn winning_at_a_high_tier_unlocks_everything() {
        let mut unlocked = Unlocked::default();
        unlocked.record(Outcome {
            won: true,
            systems_cleared: 5,
            score: 9000,
            tier: 1,
        });
        for content in ALL {
            assert!(unlocked.has(content), "{content:?} should be unlocked");
        }
    }
}
