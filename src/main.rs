use basic_tileset::BasicTileset;
use bevy::prelude::*;
use grid_wfc::GridWfc;

mod basic_tileset;
mod grid_wfc;

fn main() {
    App::new()
        // .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup() {
    let mut grid_wfc: GridWfc<BasicTileset> = GridWfc::new(UVec2::new(10, 10));
    grid_wfc.collapse(1);

    let grid = match grid_wfc.validate() {
        Ok(grid) => grid,
        Err(e) => {
            error!("Error: {}", e);
            return;
        }
    };

    for y in (0..grid[0].len()).rev() {
        for x in 0..grid.len() {
            print!("{}", &grid[x][y]);
        }
        println!();
    }
}
