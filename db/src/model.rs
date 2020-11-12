use crate::schema::users;

#[derive(Queryable)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub code: String,
    pub high_score: i32,
    pub live: bool,
    pub enabled: bool,
}

#[derive(Queryable)]
pub struct LeaderboardUser {
    pub id: i32,
    pub username: String,
    pub high_score: i32,
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct InsertableUser {
    pub username: String,
    pub code: String,
    pub enabled: bool,
}
