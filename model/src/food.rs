#[derive(Clone, Debug, serde::Serialize)]
pub enum Food {
    #[serde(rename(serialize = "F"))]
    Fruit,
    #[serde(rename(serialize = "P"))]
    PowerPill,
}
