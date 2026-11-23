use crate::skills::skill_manager::DropProbabilities;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DropMultiplier {
    Normal = 1,
    Double = 2,
    Triple = 3,
}

pub(crate) fn roll(probabilities: DropProbabilities) -> DropMultiplier {
    classify(probabilities, fastrand::u8(..DropProbabilities::TOTAL))
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
