use diesel::expression_methods::{BoolExpressionMethods, ExpressionMethods};
use diesel::prelude::*;
use diesel::result::{Error as DieselError, OptionalExtension};

use super::model::User;
use super::schema::users;

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
