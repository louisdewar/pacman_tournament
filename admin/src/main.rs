use db::actions;
use db::diesel::prelude::*;

fn main() {
    let cli_config = clap::load_yaml!("cli.yml");
    let mut app = clap::App::from(cli_config);

    match app.clone().get_matches().subcommand() {
        Some(("register", matches)) => {
            let username = matches.value_of("USERNAME").unwrap();
            let disabled = matches.is_present("DISABLED");

            register_user(username.to_owned(), !disabled);
        }
        Some(("enable", _)) => {
            set_enabled_all_users(true);
            println!("All user accounts are now enabled");
        }
        Some(("disable", _)) => {
            set_enabled_all_users(false);
            println!("All user accounts are now disabled");
        }
        Some(("leaderboard", matches)) => {
            let limit = matches
                .value_of("LIMIT")
                .map(|s| s.parse().expect("Limit must be an integer"));

            print_leaderboard(limit);
        }
        Some(("list", _)) => {
            list_users();
        }
        Some(("info", matches)) => {
            let username = matches.value_of("USERNAME").unwrap().to_owned();

            user_info(username);
        }
        Some(("delete", matches)) => {
            let username = matches.value_of("USERNAME").unwrap().to_owned();

            delete_user(username);
        }
        _ => {
            println!("Unknown command");
            app.print_long_help().unwrap();
        }
    }
}

fn establish_connection() -> PgConnection {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url))
}

fn delete_user(username: String) {
    actions::delete_user(&establish_connection(), username.clone());
    println!("Deleted {}", username);
}

fn list_users() {
    let users = actions::list_users(&establish_connection());
    eprintln!("ID USERNAME CODE ENABLED");

    for user in users {
        println!(
            "{} {} {} {}",
            user.id, user.username, user.code, user.enabled
        );
    }
}

fn user_info(username: String) {
    let user = if let Some(user) = actions::user_info(&establish_connection(), username) {
        user
    } else {
        eprintln!("User doesn't exist");
        return;
    };

    eprintln!("ID USERNAME CODE ENABLED");
    println!(
        "{} {} {} {}",
        user.id, user.username, user.code, user.enabled
    );
}

fn register_user(username: String, enabled: bool) {
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};

    if let Some(c) = username.chars().find(|c| !c.is_alphanumeric()) {
        eprintln!(
            "{} is not a valid username character only alphanumeric characters are allowed",
            c
        );
        return;
    }

    let mut rng = thread_rng();
    let code: String = (0..5).map(|_| rng.sample(Alphanumeric)).collect();

    actions::register_user(
        &establish_connection(),
        username.clone(),
        code.clone(),
        enabled,
    );

    eprintln!("USERNAME CODE (enabled = {})", username);

    println!("{} {}", username, code);
}

fn set_enabled_all_users(enabled: bool) {
    actions::set_enabled_all_users(&establish_connection(), enabled);
}

fn print_leaderboard(limit: Option<i64>) {
    let users = actions::get_leaderboard(&establish_connection(), limit);

    eprintln!("PLACE USERNAME SCORE (USER_ID)");
    for (i, user) in users.iter().enumerate() {
        println!(
            "{}. {} {} ({})",
            i + 1,
            user.username,
            user.high_score,
            user.id
        );
    }
}
