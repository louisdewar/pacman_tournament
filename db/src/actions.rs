use diesel::expression_methods::{BoolExpressionMethods, ExpressionMethods};
use diesel::prelude::*;
use diesel::result::{Error as DieselError, OptionalExtension};

use crate::model::{InsertableUser, LeaderboardUser, User};
use crate::schema::users;

// pub fn get_user_by_id(conn: &PgConnection, id: &i32) -> Result<Option<User>, DieselError> {
//     users::table.find(id).first(conn).optional()
// }

pub fn get_user_by_username(
    conn: &PgConnection,
    username: &String,
) -> Result<Option<User>, DieselError> {
    users::table
        .filter(users::columns::username.eq(username))
        .first(conn)
        .optional()
}

pub fn update_scores_if_higher(conn: &PgConnection, user_scores: Vec<(i32, i32)>) {
    for (user_id, score) in user_scores {
        diesel::update(
            users::table.filter(
                users::columns::high_score
                    .le(score)
                    .and(users::columns::id.eq(user_id)),
            ),
        )
        .set(users::columns::high_score.eq(score))
        .execute(conn)
        .expect("Couldn't update user score");
    }
}

pub fn register_user(conn: &PgConnection, username: String, code: String, enabled: bool) {
    diesel::insert_into(users::table)
        .values(&InsertableUser {
            username,
            code,
            enabled,
        })
        .execute(conn)
        .expect("Failed to register user");
}

pub fn set_enabled_all_users(conn: &PgConnection, enabled: bool) {
    diesel::update(users::table)
        .set(users::columns::enabled.eq(enabled))
        .execute(conn)
        .unwrap();
}

pub fn get_leaderboard(conn: &PgConnection, limit: Option<i64>) -> Vec<LeaderboardUser> {
    use users::columns as c;
    let query = users::table
        .select((c::id, c::username, c::high_score))
        .order(c::high_score.desc());

    if let Some(limit) = limit {
        query.limit(limit).get_results(conn)
    } else {
        query.get_results(conn)
    }
    .expect("Failed to get leaderboard")
}

pub fn list_users(conn: &PgConnection) -> Vec<User> {
    users::table.get_results(conn).expect("Couldn't get users")
}

pub fn user_info(conn: &PgConnection, username: String) -> Option<User> {
    users::table
        .filter(users::columns::username.eq(username))
        .first(conn)
        .optional()
        .expect("Couldn't get user info")
}

pub fn delete_user(conn: &PgConnection, username: String) {
    diesel::delete(users::table.filter(users::columns::username.eq(username)))
        .execute(conn)
        .expect("Couldn't delete user");
}
