use strum_macros::Display;

use super::layout_graph::LayoutGraphSettings;
use super::{
    super::{
        graph::{Graph, Superposition},
        tileset::TileSet,
    },
    layout_graph,
};
use std::convert::TryInto;
use std::fmt::format;

#[derive(Debug, Default)]
pub struct LayoutTileset;

// struct Sym3D([usize; 6]);

// impl Sym3D {
//     const X: usize = 0;
//     const NEG_X: usize = 1;
//     const Y: usize = 2;
//     const NEG_Y: usize = 3;
//     const Z: usize = 4;
//     const NEG_Z: usize = 5;
//     const ID: Sym3D = Sym3D([
//         Self::X,
//         Self::NEG_X,
//         Self::Y,
//         Self::NEG_Y,
//         Self::Z,
//         Self::NEG_Z,
//     ]);

//     fn group_operation(&self, rhs: &Sym3D) -> Self {
//         Self([
//             rhs.0[self.0[0]],
//             rhs.0[self.0[1]],
//             rhs.0[self.0[2]],
//             rhs.0[self.0[3]],
//             rhs.0[self.0[4]],
//             rhs.0[self.0[5]],
//         ])
//     }

//     fn from_map(map: [(usize, usize); 6]) -> Self {
//         let mut permuation = [0; 6];
//         for (from, to) in map.into_iter() {
//             permuation[to] = from;
//         }
//         Self(permuation)
//     }
// }
// impl std::ops::Mul for Sym3D {
//     type Output = Sym3D;
//     fn mul(self, rhs: Self) -> Self::Output {
//         self.group_operation(&rhs)
//     }
// }
// impl std::ops::Mul for &Sym3D {
//     type Output = Sym3D;
//     fn mul(self, rhs: Self) -> Self::Output {
//         self.group_operation(rhs)
//     }
// }
// impl PartialEq for Sym3D {
//     fn eq(&self, other: &Self) -> bool {
//         self.0 == other.0
//     }

//     fn ne(&self, other: &Self) -> bool {
//         self.0 != other.0
//     }
// }
// struct SymGroup3D {
//     elements: Vec<Sym3D>,
// }
// impl SymGroup3D {
//     pub fn cycle(e: Sym3D) -> Self {
//         let mut elements = vec![e];
//         while elements.last() != elements.first() {
//             elements.push(elements.first().unwrap() * elements.last().unwrap())
//         }
//         Self { elements }
//     }

//     pub fn product(a: Self, b: Self) -> Self {
//         let mut product = Self::merge(&a, &b);
//         product.expand_to_subgroup();
//         return product;
//     }

//     fn expand_to_subgroup(&mut self) {
//         loop {
//             let new = self
//                 .elements
//                 .iter()
//                 .zip(self.elements.iter())
//                 .map(|(f, g)| f * g)
//                 .filter(|e| self.elements.contains(e));

//             let length = self.elements.len();
//             self.elements.extend(new);
//             if self.elements.len() == length {
//                 break;
//             }
//         }
//     }

//     fn merge(a: &Self, b: &Self) -> Self {
//         Self {
//             elements: a
//                 .elements
//                 .into_iter()
//                 .filter(|e| !b.elements.contains(e))
//                 .chain(b.elements.into_iter())
//                 .collect::<Vec<Sym3D>>(),
//         }
//     }
// }

impl TileSet for LayoutTileset {
    type GraphSettings = LayoutGraphSettings;

    // const TILE_COUNT: usize = 120;
    // const DIRECTIONS: usize = 4;

    fn tile_count(&self) -> usize {
        14
    }

    fn arc_types(&self) -> usize {
        6 // down, north, east, south, west, up
    }

    fn get_constraints(&self) -> Vec<Vec<Superposition>> {
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum Direction {
            X,
            NegX,
            Y,
            NegY,
            Z,
            NegZ,
        }
        impl Direction {
            fn connects_to(&self) -> Self {
                match self {
                    Self::X => Self::NegX,
                    Self::NegX => Self::X,
                    Self::Y => Self::NegY,
                    Self::NegY => Self::Y,
                    Self::Z => Self::NegZ,
                    Self::NegZ => Self::Z,
                }
            }
        }

        #[derive(Clone, Copy, PartialEq, Eq, Debug, Display)]
        enum Edge {
            CornerF,
            CornerL,
            CornerR,
            CornerU,
            CornerD,

            SideF,
            SideB,
            SideL,
            SideR,
            SideU,
            SideD,

            Center,
            CenterU,
            CenterD,

            SpaceF,
            SpaceB,
            SpaceL,
            SpaceR,
            SpaceU,
            SpaceD,

            Air,
        }
        type E = Edge;

        impl Edge {
            fn connects_to(&self) -> Vec<Self> {
                match self {
                    Self::CornerF => vec![E::SpaceB],
                    Self::CornerL => vec![E::CornerR, E::SideR],
                    Self::CornerR => vec![E::CornerL, E::SideL],
                    Self::CornerU => vec![E::CornerD, E::SpaceD, E::Air],
                    Self::CornerD => vec![E::CornerU, E::CenterU, E::SideU],

                    Self::SideF => vec![E::SpaceB],
                    Self::SideB => vec![E::Center],
                    Self::SideL => vec![E::SideR, E::CornerR],
                    Self::SideR => vec![E::SideL, E::CornerL],
                    Self::SideU => vec![E::SideD, E::SpaceD, E::Air, E::CornerD],
                    Self::SideD => vec![E::SideU, E::CenterU],

                    Self::Center => vec![E::Center, E::SideB],
                    Self::CenterU => vec![E::CenterD, E::SpaceD, E::Air, E::CornerD, E::SideD],
                    Self::CenterD => vec![E::CenterU],

                    Self::SpaceF => vec![E::Air],
                    Self::SpaceB => vec![E::CornerF, E::SideF],
                    Self::SpaceR => vec![E::SpaceL, E::Air],
                    Self::SpaceL => vec![E::SpaceR, E::Air],
                    Self::SpaceU => vec![E::SpaceD, E::Air],
                    Self::SpaceD => vec![E::SpaceU, E::CornerU, E::SideU, E::CenterU],

                    Self::Air => vec![
                        E::SpaceL,
                        E::SpaceR,
                        E::SpaceF,
                        E::Air,
                        E::CenterU,
                        E::SideU,
                        E::CornerU,
                        E::SpaceU,
                    ],
                }
            }
        }

        let tile_edge_types = [
            [
                E::CornerF,
                E::CornerL,
                E::CornerU,
                E::CornerD,
                E::CornerF,
                E::CornerR,
            ],
            [E::SideR, E::SideL, E::SideU, E::SideD, E::SideF, E::SideB],
            [
                E::Center,
                E::Center,
                E::CenterU,
                E::CenterD,
                E::Center,
                E::Center,
            ],
            [
                E::SpaceR,
                E::SpaceL,
                E::SpaceU,
                E::SpaceD,
                E::SpaceF,
                E::SpaceB,
            ],
            [E::Air, E::Air, E::Air, E::Air, E::Air, E::Air],
        ];

        // Permute the edges
        fn rotate_y<T: Copy>(edges: [T; 6]) -> [T; 6] {
            return [
                edges[4], //  x <-  z       z
                edges[5], // -x <- -z     /  \
                edges[2], //  y <-  y    -x   x
                edges[3], // -y <- -y     \  /
                edges[1], //  z <- -x     -z
                edges[0], // -z <-  x
            ];
        }

        // rotate all tiles to get all possible edge types
        let mut rotated_tile_edge_types: Vec<[Edge; 6]> = Vec::with_capacity(self.tile_count());
        for edges in tile_edge_types.iter() {
            let mut rotated_edges = edges.clone();
            for rotation in 0..4 {
                if rotation != 0 && &rotated_edges == edges {
                    break;
                }
                rotated_tile_edge_types.push(rotated_edges);
                rotated_edges = rotate_y(rotated_edges);
            }
        }
        assert_eq!(self.tile_count(), rotated_tile_edge_types.len());

        // convert to allowed neighbors
        let mut allowed_neighbors = Vec::with_capacity(self.tile_count());
        for edges in rotated_tile_edge_types.iter() {
            let mut allowed_neighbors_for_tile = Vec::with_capacity(4);
            for (edge_index, edge) in edges.into_iter().enumerate() {
                // let direction = Direction::from(edge_index);
                let direction = [
                    Direction::X,
                    Direction::NegX,
                    Direction::Y,
                    Direction::NegY,
                    Direction::Z,
                    Direction::NegZ,
                ][edge_index];
                let mut supperposition = Superposition::empty();

                // add all tiles with this edge type to the neighbor set
                for (other_tile, other_edges) in rotated_tile_edge_types.iter().enumerate() {
                    let other_index = match direction.connects_to() {
                        Direction::X => 0,
                        Direction::NegX => 1,
                        Direction::Y => 2,
                        Direction::NegY => 3,
                        Direction::Z => 4,
                        Direction::NegZ => 5,
                        _ => 0,
                    };

                    if edge.connects_to().contains(&other_edges[other_index]) {
                        supperposition.add_tile(other_tile);
                    }
                }

                allowed_neighbors_for_tile.push(supperposition);
            }
            allowed_neighbors.push(allowed_neighbors_for_tile);
        }

        assert_eq!(self.tile_count(), allowed_neighbors.len());

        // for (tile, directions) in allowed_neighbors.iter().enumerate() {
        //     let edges = rotated_tile_edge_types[tile];
        //     println!(
        //         "{}: {} {} {} {}",
        //         tile, edges[0], edges[1], edges[2], edges[3]
        //     );
        //     for allowed in directions.iter() {
        //         println!(
        //             "\t[{}]",
        //             allowed
        //                 .tile_iter()
        //                 .map(|t| format!("{}", t))
        //                 .collect::<Vec<String>>()
        //                 .join(", ")
        //         )
        //     }
        //     println!("");
        // }

        // dbg!(&allowed_neighbors
        //     .iter()
        //     .map(|allowed| allowed
        //         .iter()
        //         .map(|sp| sp
        //             .tile_iter()
        //             .map(|t| format!("{}", t))
        //             .collect::<Vec<String>>()
        //             .join(", "))
        //         .collect::<Vec<String>>())
        //     .collect::<Vec<Vec<String>>>());
        allowed_neighbors
    }

    fn get_weights(&self) -> Vec<u32> {
        let mut weights = Vec::with_capacity(self.tile_count());
        for _ in 0..self.tile_count() {
            weights.push(100);
        }
        weights
    }

    fn get_tile_paths(&self) -> Vec<String> {
        let mut paths: Vec<String> = Vec::new();

        paths
    }

    fn create_graph(&self, settings: &Self::GraphSettings) -> Graph<Superposition> {
        let cell = Superposition::filled(self.tile_count());
        layout_graph::create(settings, cell)
    }
}
