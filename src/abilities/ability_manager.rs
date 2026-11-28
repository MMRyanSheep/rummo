use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AbilityId {
    SuperBreaker,
    Berserk,
}

impl AbilityId {
    pub(crate) const fn name_key(self) -> &'static str {
        match self {
            Self::SuperBreaker => "ability.super_breaker.name",
            Self::Berserk => "ability.berserk.name",
        }
    }
}
