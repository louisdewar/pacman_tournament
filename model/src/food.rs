#[derive(Clone, Debug, serde::Serialize, PartialEq)]
pub enum Food {
    #[serde(rename(serialize = "F"))]
    Fruit,
    #[serde(rename(serialize = "P"))]
    PowerPill,
}
