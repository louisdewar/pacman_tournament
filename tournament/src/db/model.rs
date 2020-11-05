#[derive(Queryable)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub code: String,
    pub high_score: i32,
    pub live: bool,
}
