use hierarchical_wfc::{
    village::{
        facade_graph::{FacadePassData, FacadeTileset},
        LayoutGraphSettings,
    },
    wfc::{Neighbour, TileSet, WfcGraph},
};

fn main() {
    let _data = FacadePassData::from_layout(&test_graph(), &test_settings());
    let tileset = FacadeTileset::from_asset("semantics/frame_test.json");
    // let mut wfc_graph = data.create_wfc_graph(&tileset);

    dbg!(tileset.superposition_from_semantic_name("vertex".to_string()));
    dbg!(tileset.superposition_from_semantic_name("edge".to_string()));
    dbg!(tileset.superposition_from_semantic_name("quad".to_string()));

    dbg!(tileset.get_constraints());

    // dbg!(&wfc_graph.nodes);/

    // WaveFunctionCollapse::collapse(
    //     &mut wfc_graph,
    //     &tileset.get_constraints(),
    //     &tileset.get_weights(),
    //     &mut StdRng::from_entropy(),
    // );
    // let binding = tileset.superposition_from_semantic_name("edge_leaf_h_flat".to_string());
    // let tile: Vec<usize> = binding.tile_iter().collect();
}

fn test_graph() -> WfcGraph<usize> {
    WfcGraph {
        nodes: vec![
            7, 8, 5, 10, 13, 12, 3, 0, 10, 13, 3, 4, 0, 10, 13, 13, 9, 9, 13, 12, 9, 9, 9, 13, 11,
            11, 13, 13, 13, 13, 13, 13, 13, 12, 2, 1, 10, 13, 13, 11, 10, 13, 13, 12, 3, 0, 10, 13,
            12, 2, 10, 13, 13, 13, 9, 9, 13, 13, 12, 3, 13, 13, 13, 13, 13, 13, 13, 13, 13, 9, 13,
            11, 11, 11, 11, 11, 11, 13, 13, 13, 12, 2, 6, 6, 6, 6, 1, 10, 13, 12, 12, 7, 8, 8, 8,
            8, 5, 10, 13, 12,
        ],
        order: vec![
            83, 93, 73, 63, 72, 62, 74, 64, 53, 71, 61, 82, 92, 52, 84, 94, 75, 65, 54, 44, 43, 33,
            32, 42, 55, 66, 41, 51, 23, 24, 25, 34, 14, 35, 36, 45, 46, 47, 56, 37, 26, 15, 81, 80,
            91, 90, 70, 57, 50, 40, 31, 21, 3, 4, 12, 13, 22, 11, 1, 2, 20, 30, 85, 95, 76, 67, 68,
            10, 0, 48, 38, 39, 49, 29, 59, 58, 69, 79, 77, 87, 88, 86, 97, 98, 96, 78, 28, 27, 18,
            60, 17, 7, 8, 6, 5, 16, 9, 89, 19, 99,
        ],
        neighbors: Box::new([
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 1,
                },
                Neighbour {
                    arc_type: 4,
                    index: 10,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 2,
                },
                Neighbour {
                    arc_type: 1,
                    index: 0,
                },
                Neighbour {
                    arc_type: 4,
                    index: 11,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 3,
                },
                Neighbour {
                    arc_type: 1,
                    index: 1,
                },
                Neighbour {
                    arc_type: 4,
                    index: 12,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 4,
                },
                Neighbour {
                    arc_type: 1,
                    index: 2,
                },
                Neighbour {
                    arc_type: 4,
                    index: 13,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 5,
                },
                Neighbour {
                    arc_type: 1,
                    index: 3,
                },
                Neighbour {
                    arc_type: 4,
                    index: 14,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 6,
                },
                Neighbour {
                    arc_type: 1,
                    index: 4,
                },
                Neighbour {
                    arc_type: 4,
                    index: 15,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 7,
                },
                Neighbour {
                    arc_type: 1,
                    index: 5,
                },
                Neighbour {
                    arc_type: 4,
                    index: 16,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 8,
                },
                Neighbour {
                    arc_type: 1,
                    index: 6,
                },
                Neighbour {
                    arc_type: 4,
                    index: 17,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 9,
                },
                Neighbour {
                    arc_type: 1,
                    index: 7,
                },
                Neighbour {
                    arc_type: 4,
                    index: 18,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 1,
                    index: 8,
                },
                Neighbour {
                    arc_type: 4,
                    index: 19,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 11,
                },
                Neighbour {
                    arc_type: 4,
                    index: 20,
                },
                Neighbour {
                    arc_type: 5,
                    index: 0,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 12,
                },
                Neighbour {
                    arc_type: 1,
                    index: 10,
                },
                Neighbour {
                    arc_type: 4,
                    index: 21,
                },
                Neighbour {
                    arc_type: 5,
                    index: 1,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 13,
                },
                Neighbour {
                    arc_type: 1,
                    index: 11,
                },
                Neighbour {
                    arc_type: 4,
                    index: 22,
                },
                Neighbour {
                    arc_type: 5,
                    index: 2,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 14,
                },
                Neighbour {
                    arc_type: 1,
                    index: 12,
                },
                Neighbour {
                    arc_type: 4,
                    index: 23,
                },
                Neighbour {
                    arc_type: 5,
                    index: 3,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 15,
                },
                Neighbour {
                    arc_type: 1,
                    index: 13,
                },
                Neighbour {
                    arc_type: 4,
                    index: 24,
                },
                Neighbour {
                    arc_type: 5,
                    index: 4,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 16,
                },
                Neighbour {
                    arc_type: 1,
                    index: 14,
                },
                Neighbour {
                    arc_type: 4,
                    index: 25,
                },
                Neighbour {
                    arc_type: 5,
                    index: 5,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 17,
                },
                Neighbour {
                    arc_type: 1,
                    index: 15,
                },
                Neighbour {
                    arc_type: 4,
                    index: 26,
                },
                Neighbour {
                    arc_type: 5,
                    index: 6,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 18,
                },
                Neighbour {
                    arc_type: 1,
                    index: 16,
                },
                Neighbour {
                    arc_type: 4,
                    index: 27,
                },
                Neighbour {
                    arc_type: 5,
                    index: 7,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 19,
                },
                Neighbour {
                    arc_type: 1,
                    index: 17,
                },
                Neighbour {
                    arc_type: 4,
                    index: 28,
                },
                Neighbour {
                    arc_type: 5,
                    index: 8,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 1,
                    index: 18,
                },
                Neighbour {
                    arc_type: 4,
                    index: 29,
                },
                Neighbour {
                    arc_type: 5,
                    index: 9,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 21,
                },
                Neighbour {
                    arc_type: 4,
                    index: 30,
                },
                Neighbour {
                    arc_type: 5,
                    index: 10,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 22,
                },
                Neighbour {
                    arc_type: 1,
                    index: 20,
                },
                Neighbour {
                    arc_type: 4,
                    index: 31,
                },
                Neighbour {
                    arc_type: 5,
                    index: 11,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 23,
                },
                Neighbour {
                    arc_type: 1,
                    index: 21,
                },
                Neighbour {
                    arc_type: 4,
                    index: 32,
                },
                Neighbour {
                    arc_type: 5,
                    index: 12,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 24,
                },
                Neighbour {
                    arc_type: 1,
                    index: 22,
                },
                Neighbour {
                    arc_type: 4,
                    index: 33,
                },
                Neighbour {
                    arc_type: 5,
                    index: 13,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 25,
                },
                Neighbour {
                    arc_type: 1,
                    index: 23,
                },
                Neighbour {
                    arc_type: 4,
                    index: 34,
                },
                Neighbour {
                    arc_type: 5,
                    index: 14,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 26,
                },
                Neighbour {
                    arc_type: 1,
                    index: 24,
                },
                Neighbour {
                    arc_type: 4,
                    index: 35,
                },
                Neighbour {
                    arc_type: 5,
                    index: 15,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 27,
                },
                Neighbour {
                    arc_type: 1,
                    index: 25,
                },
                Neighbour {
                    arc_type: 4,
                    index: 36,
                },
                Neighbour {
                    arc_type: 5,
                    index: 16,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 28,
                },
                Neighbour {
                    arc_type: 1,
                    index: 26,
                },
                Neighbour {
                    arc_type: 4,
                    index: 37,
                },
                Neighbour {
                    arc_type: 5,
                    index: 17,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 29,
                },
                Neighbour {
                    arc_type: 1,
                    index: 27,
                },
                Neighbour {
                    arc_type: 4,
                    index: 38,
                },
                Neighbour {
                    arc_type: 5,
                    index: 18,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 1,
                    index: 28,
                },
                Neighbour {
                    arc_type: 4,
                    index: 39,
                },
                Neighbour {
                    arc_type: 5,
                    index: 19,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 31,
                },
                Neighbour {
                    arc_type: 4,
                    index: 40,
                },
                Neighbour {
                    arc_type: 5,
                    index: 20,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 32,
                },
                Neighbour {
                    arc_type: 1,
                    index: 30,
                },
                Neighbour {
                    arc_type: 4,
                    index: 41,
                },
                Neighbour {
                    arc_type: 5,
                    index: 21,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 33,
                },
                Neighbour {
                    arc_type: 1,
                    index: 31,
                },
                Neighbour {
                    arc_type: 4,
                    index: 42,
                },
                Neighbour {
                    arc_type: 5,
                    index: 22,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 34,
                },
                Neighbour {
                    arc_type: 1,
                    index: 32,
                },
                Neighbour {
                    arc_type: 4,
                    index: 43,
                },
                Neighbour {
                    arc_type: 5,
                    index: 23,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 35,
                },
                Neighbour {
                    arc_type: 1,
                    index: 33,
                },
                Neighbour {
                    arc_type: 4,
                    index: 44,
                },
                Neighbour {
                    arc_type: 5,
                    index: 24,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 36,
                },
                Neighbour {
                    arc_type: 1,
                    index: 34,
                },
                Neighbour {
                    arc_type: 4,
                    index: 45,
                },
                Neighbour {
                    arc_type: 5,
                    index: 25,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 37,
                },
                Neighbour {
                    arc_type: 1,
                    index: 35,
                },
                Neighbour {
                    arc_type: 4,
                    index: 46,
                },
                Neighbour {
                    arc_type: 5,
                    index: 26,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 38,
                },
                Neighbour {
                    arc_type: 1,
                    index: 36,
                },
                Neighbour {
                    arc_type: 4,
                    index: 47,
                },
                Neighbour {
                    arc_type: 5,
                    index: 27,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 39,
                },
                Neighbour {
                    arc_type: 1,
                    index: 37,
                },
                Neighbour {
                    arc_type: 4,
                    index: 48,
                },
                Neighbour {
                    arc_type: 5,
                    index: 28,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 1,
                    index: 38,
                },
                Neighbour {
                    arc_type: 4,
                    index: 49,
                },
                Neighbour {
                    arc_type: 5,
                    index: 29,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 41,
                },
                Neighbour {
                    arc_type: 4,
                    index: 50,
                },
                Neighbour {
                    arc_type: 5,
                    index: 30,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 42,
                },
                Neighbour {
                    arc_type: 1,
                    index: 40,
                },
                Neighbour {
                    arc_type: 4,
                    index: 51,
                },
                Neighbour {
                    arc_type: 5,
                    index: 31,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 43,
                },
                Neighbour {
                    arc_type: 1,
                    index: 41,
                },
                Neighbour {
                    arc_type: 4,
                    index: 52,
                },
                Neighbour {
                    arc_type: 5,
                    index: 32,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 44,
                },
                Neighbour {
                    arc_type: 1,
                    index: 42,
                },
                Neighbour {
                    arc_type: 4,
                    index: 53,
                },
                Neighbour {
                    arc_type: 5,
                    index: 33,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 45,
                },
                Neighbour {
                    arc_type: 1,
                    index: 43,
                },
                Neighbour {
                    arc_type: 4,
                    index: 54,
                },
                Neighbour {
                    arc_type: 5,
                    index: 34,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 46,
                },
                Neighbour {
                    arc_type: 1,
                    index: 44,
                },
                Neighbour {
                    arc_type: 4,
                    index: 55,
                },
                Neighbour {
                    arc_type: 5,
                    index: 35,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 47,
                },
                Neighbour {
                    arc_type: 1,
                    index: 45,
                },
                Neighbour {
                    arc_type: 4,
                    index: 56,
                },
                Neighbour {
                    arc_type: 5,
                    index: 36,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 48,
                },
                Neighbour {
                    arc_type: 1,
                    index: 46,
                },
                Neighbour {
                    arc_type: 4,
                    index: 57,
                },
                Neighbour {
                    arc_type: 5,
                    index: 37,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 49,
                },
                Neighbour {
                    arc_type: 1,
                    index: 47,
                },
                Neighbour {
                    arc_type: 4,
                    index: 58,
                },
                Neighbour {
                    arc_type: 5,
                    index: 38,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 1,
                    index: 48,
                },
                Neighbour {
                    arc_type: 4,
                    index: 59,
                },
                Neighbour {
                    arc_type: 5,
                    index: 39,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 51,
                },
                Neighbour {
                    arc_type: 4,
                    index: 60,
                },
                Neighbour {
                    arc_type: 5,
                    index: 40,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 52,
                },
                Neighbour {
                    arc_type: 1,
                    index: 50,
                },
                Neighbour {
                    arc_type: 4,
                    index: 61,
                },
                Neighbour {
                    arc_type: 5,
                    index: 41,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 53,
                },
                Neighbour {
                    arc_type: 1,
                    index: 51,
                },
                Neighbour {
                    arc_type: 4,
                    index: 62,
                },
                Neighbour {
                    arc_type: 5,
                    index: 42,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 54,
                },
                Neighbour {
                    arc_type: 1,
                    index: 52,
                },
                Neighbour {
                    arc_type: 4,
                    index: 63,
                },
                Neighbour {
                    arc_type: 5,
                    index: 43,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 55,
                },
                Neighbour {
                    arc_type: 1,
                    index: 53,
                },
                Neighbour {
                    arc_type: 4,
                    index: 64,
                },
                Neighbour {
                    arc_type: 5,
                    index: 44,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 56,
                },
                Neighbour {
                    arc_type: 1,
                    index: 54,
                },
                Neighbour {
                    arc_type: 4,
                    index: 65,
                },
                Neighbour {
                    arc_type: 5,
                    index: 45,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 57,
                },
                Neighbour {
                    arc_type: 1,
                    index: 55,
                },
                Neighbour {
                    arc_type: 4,
                    index: 66,
                },
                Neighbour {
                    arc_type: 5,
                    index: 46,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 58,
                },
                Neighbour {
                    arc_type: 1,
                    index: 56,
                },
                Neighbour {
                    arc_type: 4,
                    index: 67,
                },
                Neighbour {
                    arc_type: 5,
                    index: 47,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 59,
                },
                Neighbour {
                    arc_type: 1,
                    index: 57,
                },
                Neighbour {
                    arc_type: 4,
                    index: 68,
                },
                Neighbour {
                    arc_type: 5,
                    index: 48,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 1,
                    index: 58,
                },
                Neighbour {
                    arc_type: 4,
                    index: 69,
                },
                Neighbour {
                    arc_type: 5,
                    index: 49,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 61,
                },
                Neighbour {
                    arc_type: 4,
                    index: 70,
                },
                Neighbour {
                    arc_type: 5,
                    index: 50,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 62,
                },
                Neighbour {
                    arc_type: 1,
                    index: 60,
                },
                Neighbour {
                    arc_type: 4,
                    index: 71,
                },
                Neighbour {
                    arc_type: 5,
                    index: 51,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 63,
                },
                Neighbour {
                    arc_type: 1,
                    index: 61,
                },
                Neighbour {
                    arc_type: 4,
                    index: 72,
                },
                Neighbour {
                    arc_type: 5,
                    index: 52,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 64,
                },
                Neighbour {
                    arc_type: 1,
                    index: 62,
                },
                Neighbour {
                    arc_type: 4,
                    index: 73,
                },
                Neighbour {
                    arc_type: 5,
                    index: 53,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 65,
                },
                Neighbour {
                    arc_type: 1,
                    index: 63,
                },
                Neighbour {
                    arc_type: 4,
                    index: 74,
                },
                Neighbour {
                    arc_type: 5,
                    index: 54,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 66,
                },
                Neighbour {
                    arc_type: 1,
                    index: 64,
                },
                Neighbour {
                    arc_type: 4,
                    index: 75,
                },
                Neighbour {
                    arc_type: 5,
                    index: 55,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 67,
                },
                Neighbour {
                    arc_type: 1,
                    index: 65,
                },
                Neighbour {
                    arc_type: 4,
                    index: 76,
                },
                Neighbour {
                    arc_type: 5,
                    index: 56,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 68,
                },
                Neighbour {
                    arc_type: 1,
                    index: 66,
                },
                Neighbour {
                    arc_type: 4,
                    index: 77,
                },
                Neighbour {
                    arc_type: 5,
                    index: 57,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 69,
                },
                Neighbour {
                    arc_type: 1,
                    index: 67,
                },
                Neighbour {
                    arc_type: 4,
                    index: 78,
                },
                Neighbour {
                    arc_type: 5,
                    index: 58,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 1,
                    index: 68,
                },
                Neighbour {
                    arc_type: 4,
                    index: 79,
                },
                Neighbour {
                    arc_type: 5,
                    index: 59,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 71,
                },
                Neighbour {
                    arc_type: 4,
                    index: 80,
                },
                Neighbour {
                    arc_type: 5,
                    index: 60,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 72,
                },
                Neighbour {
                    arc_type: 1,
                    index: 70,
                },
                Neighbour {
                    arc_type: 4,
                    index: 81,
                },
                Neighbour {
                    arc_type: 5,
                    index: 61,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 73,
                },
                Neighbour {
                    arc_type: 1,
                    index: 71,
                },
                Neighbour {
                    arc_type: 4,
                    index: 82,
                },
                Neighbour {
                    arc_type: 5,
                    index: 62,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 74,
                },
                Neighbour {
                    arc_type: 1,
                    index: 72,
                },
                Neighbour {
                    arc_type: 4,
                    index: 83,
                },
                Neighbour {
                    arc_type: 5,
                    index: 63,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 75,
                },
                Neighbour {
                    arc_type: 1,
                    index: 73,
                },
                Neighbour {
                    arc_type: 4,
                    index: 84,
                },
                Neighbour {
                    arc_type: 5,
                    index: 64,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 76,
                },
                Neighbour {
                    arc_type: 1,
                    index: 74,
                },
                Neighbour {
                    arc_type: 4,
                    index: 85,
                },
                Neighbour {
                    arc_type: 5,
                    index: 65,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 77,
                },
                Neighbour {
                    arc_type: 1,
                    index: 75,
                },
                Neighbour {
                    arc_type: 4,
                    index: 86,
                },
                Neighbour {
                    arc_type: 5,
                    index: 66,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 78,
                },
                Neighbour {
                    arc_type: 1,
                    index: 76,
                },
                Neighbour {
                    arc_type: 4,
                    index: 87,
                },
                Neighbour {
                    arc_type: 5,
                    index: 67,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 79,
                },
                Neighbour {
                    arc_type: 1,
                    index: 77,
                },
                Neighbour {
                    arc_type: 4,
                    index: 88,
                },
                Neighbour {
                    arc_type: 5,
                    index: 68,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 1,
                    index: 78,
                },
                Neighbour {
                    arc_type: 4,
                    index: 89,
                },
                Neighbour {
                    arc_type: 5,
                    index: 69,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 81,
                },
                Neighbour {
                    arc_type: 4,
                    index: 90,
                },
                Neighbour {
                    arc_type: 5,
                    index: 70,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 82,
                },
                Neighbour {
                    arc_type: 1,
                    index: 80,
                },
                Neighbour {
                    arc_type: 4,
                    index: 91,
                },
                Neighbour {
                    arc_type: 5,
                    index: 71,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 83,
                },
                Neighbour {
                    arc_type: 1,
                    index: 81,
                },
                Neighbour {
                    arc_type: 4,
                    index: 92,
                },
                Neighbour {
                    arc_type: 5,
                    index: 72,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 84,
                },
                Neighbour {
                    arc_type: 1,
                    index: 82,
                },
                Neighbour {
                    arc_type: 4,
                    index: 93,
                },
                Neighbour {
                    arc_type: 5,
                    index: 73,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 85,
                },
                Neighbour {
                    arc_type: 1,
                    index: 83,
                },
                Neighbour {
                    arc_type: 4,
                    index: 94,
                },
                Neighbour {
                    arc_type: 5,
                    index: 74,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 86,
                },
                Neighbour {
                    arc_type: 1,
                    index: 84,
                },
                Neighbour {
                    arc_type: 4,
                    index: 95,
                },
                Neighbour {
                    arc_type: 5,
                    index: 75,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 87,
                },
                Neighbour {
                    arc_type: 1,
                    index: 85,
                },
                Neighbour {
                    arc_type: 4,
                    index: 96,
                },
                Neighbour {
                    arc_type: 5,
                    index: 76,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 88,
                },
                Neighbour {
                    arc_type: 1,
                    index: 86,
                },
                Neighbour {
                    arc_type: 4,
                    index: 97,
                },
                Neighbour {
                    arc_type: 5,
                    index: 77,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 89,
                },
                Neighbour {
                    arc_type: 1,
                    index: 87,
                },
                Neighbour {
                    arc_type: 4,
                    index: 98,
                },
                Neighbour {
                    arc_type: 5,
                    index: 78,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 1,
                    index: 88,
                },
                Neighbour {
                    arc_type: 4,
                    index: 99,
                },
                Neighbour {
                    arc_type: 5,
                    index: 79,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 91,
                },
                Neighbour {
                    arc_type: 5,
                    index: 80,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 92,
                },
                Neighbour {
                    arc_type: 1,
                    index: 90,
                },
                Neighbour {
                    arc_type: 5,
                    index: 81,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 93,
                },
                Neighbour {
                    arc_type: 1,
                    index: 91,
                },
                Neighbour {
                    arc_type: 5,
                    index: 82,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 94,
                },
                Neighbour {
                    arc_type: 1,
                    index: 92,
                },
                Neighbour {
                    arc_type: 5,
                    index: 83,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 95,
                },
                Neighbour {
                    arc_type: 1,
                    index: 93,
                },
                Neighbour {
                    arc_type: 5,
                    index: 84,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 96,
                },
                Neighbour {
                    arc_type: 1,
                    index: 94,
                },
                Neighbour {
                    arc_type: 5,
                    index: 85,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 97,
                },
                Neighbour {
                    arc_type: 1,
                    index: 95,
                },
                Neighbour {
                    arc_type: 5,
                    index: 86,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 98,
                },
                Neighbour {
                    arc_type: 1,
                    index: 96,
                },
                Neighbour {
                    arc_type: 5,
                    index: 87,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 0,
                    index: 99,
                },
                Neighbour {
                    arc_type: 1,
                    index: 97,
                },
                Neighbour {
                    arc_type: 5,
                    index: 88,
                },
            ]),
            Box::new([
                Neighbour {
                    arc_type: 1,
                    index: 98,
                },
                Neighbour {
                    arc_type: 5,
                    index: 89,
                },
            ]),
        ]),
    }
}

fn test_settings() -> LayoutGraphSettings {
    LayoutGraphSettings {
        x_size: 10,
        y_size: 1,
        z_size: 10,
        periodic: false,
    }
}
