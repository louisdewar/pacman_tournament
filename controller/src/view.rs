use ggez::{
    graphics::{self, spritebatch::SpriteBatch},
    Context, GameResult,
};

use model::{BaseTile, Entity, Food, GameEvent, Model};

fn load_sprite<P: AsRef<std::path::Path>>(ctx: &mut Context, path: P) -> SpriteBatch {
    let image = graphics::Image::new(ctx, path).unwrap();
    SpriteBatch::new(image)
}

fn tile_to_tex(tile: &BaseTile) -> graphics::Color {
    match tile {
        BaseTile::Water => (0.0, 0.0, 1.0).into(),
        BaseTile::Land => (0.0, 1.0, 0.0).into(),
        BaseTile::Wall => (0.2, 0.2, 0.2).into(),
    }
}

pub struct View {
    player_sprite: SpriteBatch,
    mob_sprite: SpriteBatch,
    fruit_sprite: SpriteBatch,
    powerpill_sprite: SpriteBatch,
}

impl View {
    pub fn new(ctx: &mut Context) -> View {
        View {
            player_sprite: load_sprite(ctx, "/player.png"),
            mob_sprite: load_sprite(ctx, "/mob.png"),
            fruit_sprite: load_sprite(ctx, "/fruit.png"),
            powerpill_sprite: load_sprite(ctx, "/power.png"),
        }
    }

    fn build_terrain_mesh<F: Fn(GameEvent)>(
        &self,
        model: &Model<F>,
        square_width: f32,
    ) -> graphics::MeshBuilder {
        let mut builder = graphics::MeshBuilder::new();

        for x in 0..(model.map().width() as usize) {
            for y in 0..(model.map().height() as usize) {
                let tex = tile_to_tex(&model.map().base_tile(x, y));
                let rect = graphics::Rect::new(
                    x as f32 * square_width,
                    y as f32 * square_width,
                    square_width,
                    square_width,
                );
                builder.rectangle(graphics::DrawMode::Fill(Default::default()), rect, tex);
            }
        }

        builder
    }

    pub fn setup(&mut self, ctx: &mut Context) -> GameResult {
        graphics::set_resizable(ctx, true)?;
        graphics::set_window_title(ctx, "Ai game");

        Ok(())
    }

    pub fn draw<F: Fn(GameEvent)>(&mut self, ctx: &mut Context, model: &Model<F>) -> GameResult {
        graphics::clear(ctx, [0.5, 0.5, 0.5, 1.0].into());
        let cur_time = model.tick_start().elapsed().as_secs_f32();
        let square_width = 15.0;
        let mesh = self.build_terrain_mesh(model, square_width).build(ctx)?;

        for (_index, player) in model.players().iter() {
            let (x, y) = player.borrow_mut().position_animated(cur_time);
            let param = graphics::DrawParam::new()
                .offset([0.5, 0.5])
                .dest([
                    (x as f32 + 0.5) * square_width,
                    (y as f32 + 0.5) * square_width,
                ])
                .scale([square_width / 170.0, square_width / 170.0])
                .rotation(player.borrow().direction().to_rad());

            self.player_sprite.add(param);
        }

        for (_index, mob) in model.mobs().iter() {
            let (x, y) = mob.borrow_mut().position_animated(cur_time);
            let param = graphics::DrawParam::new()
                .offset([0.5, 0.5])
                .dest([
                    (x as f32 + 0.5) * square_width,
                    (y as f32 + 0.5) * square_width,
                ])
                .scale([square_width / 170.0, square_width / 170.0])
                .rotation(mob.borrow().direction().to_rad());

            self.mob_sprite.add(param);
        }

        let food_grid = model.food();

        for x in 0..food_grid.width() {
            for y in 0..food_grid.height() {
                if let Some(food) = &food_grid[x][y] {
                    let param = graphics::DrawParam::new()
                        .offset([0.5, 0.5])
                        .dest([
                            (x as f32 + 0.5) * square_width,
                            (y as f32 + 0.5) * square_width,
                        ])
                        .scale([square_width / 32.0, square_width / 32.0]);
                    match food {
                        Food::Fruit => {
                            self.fruit_sprite.add(param);
                        }
                        Food::PowerPill => {
                            self.powerpill_sprite.add(param);
                        }
                    }
                }
            }
        }

        let scale = 1.5;

        let draw_params = graphics::DrawParam::new()
            .dest([
                -(model.map().width() as f32) * scale * square_width / 2.0,
                -(model.map().height() as f32) * scale * square_width / 2.0,
            ])
            .scale([scale; 2]);
        graphics::draw(ctx, &mesh, draw_params.clone())?;
        graphics::draw(ctx, &self.fruit_sprite, draw_params.clone())?;
        graphics::draw(ctx, &self.powerpill_sprite, draw_params.clone())?;
        graphics::draw(ctx, &self.player_sprite, draw_params.clone())?;
        graphics::draw(ctx, &self.mob_sprite, draw_params.clone())?;

        self.player_sprite.clear();
        self.mob_sprite.clear();
        self.fruit_sprite.clear();
        self.powerpill_sprite.clear();

        graphics::present(ctx)
    }
}
