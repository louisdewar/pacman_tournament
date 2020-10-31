use model::{Map, Model, NetworkManager};

use ggez::{
    event::EventHandler,
    graphics,
    graphics::spritebatch::SpriteBatch,
    input::keyboard::{KeyCode, KeyMods},
    Context, GameResult,
};

mod view;
use view::View;

struct Game {
    model: Model,
    view: View,
}

impl Game {
    pub fn new<A: std::net::ToSocketAddrs>(map: Map, ctx: &mut Context, addr: A) -> Game {
        let (tx, rx) = NetworkManager::start(addr).expect("Couldn't start network manager");
        Game {
            model: Model::new(map, rx, tx),
            view: View::new(ctx),
        }
    }

    fn setup(&mut self, ctx: &mut Context) -> GameResult {
        self.view.setup(ctx)
    }
}

impl EventHandler for Game {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        self.model.handle_network_messages();

        if ggez::timer::check_update_time(ctx, 1) {
            self.model.simulate_tick();
        } else {
            std::thread::sleep(std::time::Duration::from_millis(15));
        }

        Ok(())
    }

    fn resize_event(&mut self, ctx: &mut Context, width: f32, height: f32) {
        graphics::set_screen_coordinates(
            ctx,
            graphics::Rect::new(-width / 2.0, -height / 2.0, width, height),
        )
        .unwrap();
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        self.view.draw(ctx, &self.model)
    }

    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        keycode: KeyCode,
        _keymods: KeyMods,
        _repeat: bool,
    ) {
        match keycode {
            KeyCode::M => {
                if !self.model.spawn_mob() {
                    println!("Couldn't spawn mob");
                } else {
                    println!(
                        "Spawned mob, there are now: {} mobs",
                        self.model.mobs().len()
                    );
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let map = Map::new(16, 16);

    let mut cb = ggez::ContextBuilder::new("Ai Game", "Louis de Wardt")
        .window_mode(ggez::conf::WindowMode::default().resizable(true))
        .window_setup(ggez::conf::WindowSetup::default().vsync(true));

    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let mut path = std::path::PathBuf::from(manifest_dir);
        path.push("resources");
        cb = cb.add_resource_path(path);
    } else {
        cb = cb.add_resource_path("./resources");
    }

    let (mut ctx, mut event_loop) = cb.build().unwrap();

    let mut game = Game::new(map, &mut ctx, "localhost:2010");
    println!("Listening on localhost:2010");
    game.setup(&mut ctx).unwrap();
    ggez::event::run(&mut ctx, &mut event_loop, &mut game).unwrap();
}
