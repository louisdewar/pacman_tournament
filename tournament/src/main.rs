mod competitor;
mod game;
mod spectator;

fn main() {
    println!("Hello, world!");
    let (competitor_tx, competitor_rx) =
        competitor::Manager::start("localhost:2010").expect("Couldn't start client manager");
}
