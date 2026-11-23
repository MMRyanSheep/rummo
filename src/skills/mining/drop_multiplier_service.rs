use pumpkin_plugin_api::{Player, Result};

use crate::skills::skill_manager::{SkillManager, SkillType};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DropMultiplier {
    Normal = 1,
    Double = 2,
    Triple = 3,
}

/// Mutually exclusive drop chances as percentages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DropProbabilities {
    normal: u8,
    double: u8,
    triple: u8,
}

impl DropProbabilities {
    const TOTAL: u8 = 100;

    fn for_mining_level(level: u32) -> Self {
        let total = u32::from(Self::TOTAL);
        let triple = level.saturating_mul(5).min(total);
        let double = level.saturating_mul(15).min(total - triple);
        Self {
            normal: (total - triple - double) as u8,
            double: double as u8,
            triple: triple as u8,
        }
    }
}

pub(crate) fn roll(player: &Player) -> Result<DropMultiplier> {
    let level = SkillManager::get_skill(player, SkillType::Mining)?.level;
    let probabilities = DropProbabilities::for_mining_level(level);
    let total = probabilities.triple + probabilities.double + probabilities.normal;
    Ok(classify(probabilities, fastrand::u8(..total)))
}

fn classify(probabilities: DropProbabilities, draw: u8) -> DropMultiplier {
    if draw < probabilities.triple {
        DropMultiplier::Triple
    } else if draw < probabilities.triple + probabilities.double {
        DropMultiplier::Double
    } else {
        DropMultiplier::Normal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mining_drop_probabilities_give_triple_priority() {
        for (level, normal, double, triple) in [
            (0, 100, 0, 0),
            (1, 80, 15, 5),
            (4, 20, 60, 20),
            (5, 0, 75, 25),
            (6, 0, 70, 30),
            (19, 0, 5, 95),
            (20, 0, 0, 100),
            (u32::MAX, 0, 0, 100),
        ] {
            assert_eq!(
                DropProbabilities::for_mining_level(level),
                DropProbabilities {
                    normal,
                    double,
                    triple,
                },
            );
        }
    }

    #[test]
    fn each_draw_belongs_to_one_outcome() {
        for probabilities in [
            DropProbabilities {
                normal: 80,
                double: 15,
                triple: 5,
            },
            DropProbabilities {
                normal: 0,
                double: 70,
                triple: 30,
            },
            DropProbabilities {
                normal: 0,
                double: 0,
                triple: 100,
            },
        ] {
            let mut counts = [0; 3];
            for draw in 0..DropProbabilities::TOTAL {
                let multiplier = classify(probabilities, draw);
                counts[multiplier as usize - 1] += 1;
            }
            assert_eq!(
                counts,
                [
                    probabilities.normal,
                    probabilities.double,
                    probabilities.triple
                ]
            );
        }
    }
}
