impl GraphBuilder {
    const ARC_COLORS: [Vec4; 7] = [
        vec4(1.0, 0.1, 0.1, 1.0),
        vec4(0.1, 1.0, 1.0, 1.0),
        vec4(0.1, 1.0, 0.1, 1.0),
        vec4(1.0, 0.1, 1.0, 1.0),
        vec4(0.1, 0.1, 1.0, 1.0),
        vec4(1.0, 1.0, 0.1, 1.0),
        vec4(0.1, 0.1, 0.1, 1.0),
    ];

    pub fn debug_vertex_mesh(&self, vertex_mesh: Mesh) -> Mesh {
        let mut vertex_mesh_builder = MeshBuilder::new();

        for vertex in self.vertices.iter() {
            vertex_mesh_builder.add_mesh(
                &vertex_mesh,
                Transform::from_translation(vertex.pos.as_vec3() * vec3(2.0, 3.0, 2.0)),
                0,
            );
        }

        vertex_mesh_builder.build()
    }

    pub fn debug_edge_mesh(&self, edge_mesh: Mesh) -> Mesh {
        let mut vertex_mesh_builder = MeshBuilder::new();

        for vertex in self.edges.iter() {
            vertex_mesh_builder.add_mesh(
                &edge_mesh,
                Transform::from_translation(vertex.pos.as_vec3() * vec3(2.0, 3.0, 2.0) * 0.5),
                0,
            );
        }

        vertex_mesh_builder.build()
    }

    pub fn debug_quad_mesh(&self, quad_mesh: Mesh) -> Mesh {
        let mut vertex_mesh_builder = MeshBuilder::new();

        for vertex in self.quads.iter() {
            vertex_mesh_builder.add_mesh(
                &quad_mesh,
                Transform::from_translation(vertex.pos.as_vec3() * vec3(2.0, 3.0, 2.0) * 0.25),
                0,
            );
        }

        vertex_mesh_builder.build()
    }

    pub fn debug_arcs_mesh(&self) -> Mesh {
        let mut arc_vertex_positions = Vec::new();
        let mut arc_vertex_normals = Vec::new();
        let mut arc_vertex_uvs = Vec::new();
        let mut arc_vertex_colors = Vec::new();

        for edge in self.edges.iter() {
            for dir_quad in edge.quads.iter() {
                let quad = &self.quads[dir_quad.1];
                let color = Self::ARC_COLORS[dir_quad.0]; //[*arc_type.min(&6)];

                let u = edge.pos.as_vec3() * vec3(2.0, 3.0, 2.0) * 0.50;
                let v = quad.pos.as_vec3() * vec3(2.0, 3.0, 2.0) * 0.25;
                let normal = (u - v).normalize();

                arc_vertex_positions.extend([u, v, u, v, v, u]);
                arc_vertex_normals.extend([
                    Vec3::ZERO,
                    Vec3::ZERO,
                    normal,
                    Vec3::ZERO,
                    normal,
                    normal,
                ]);

                arc_vertex_uvs.extend([
                    Vec2::ZERO,
                    (v - u).length() * Vec2::X,
                    Vec2::Y,
                    (v - u).length() * Vec2::X,
                    (v - u).length() * Vec2::X + Vec2::Y,
                    Vec2::Y,
                ]);

                arc_vertex_colors.extend([color; 6])
            }

            for (invert_direction, vertex) in [edge.to, edge.from].iter().enumerate() {
                let vertex = &self.vertices[*vertex];
                let color = Self::ARC_COLORS[edge.tangent ^ invert_direction]; //[*arc_type.min(&6)];

                let u = edge.pos.as_vec3() * vec3(2.0, 3.0, 2.0) * 0.50;
                let v = vertex.pos.as_vec3() * vec3(2.0, 3.0, 2.0);
                let normal = (u - v).normalize();

                arc_vertex_positions.extend([u, v, u, v, v, u]);
                arc_vertex_normals.extend([
                    Vec3::ZERO,
                    Vec3::ZERO,
                    normal,
                    Vec3::ZERO,
                    normal,
                    normal,
                ]);

                arc_vertex_uvs.extend([
                    Vec2::ZERO,
                    (v - u).length() * Vec2::X,
                    Vec2::Y,
                    (v - u).length() * Vec2::X,
                    (v - u).length() * Vec2::X + Vec2::Y,
                    Vec2::Y,
                ]);

                arc_vertex_colors.extend([color; 6])
            }
        }

        for vertex in self.vertices.iter() {
            for (direction, edge) in vertex.edges.iter().enumerate() {
                if let Some(edge) = edge {
                    let edge = &self.edges[*edge];
                    let color = Self::ARC_COLORS[direction]; //[*arc_type.min(&6)];

                    let u = vertex.pos.as_vec3() * vec3(2.0, 3.0, 2.0);
                    let v = edge.pos.as_vec3() * vec3(2.0, 3.0, 2.0) * 0.5;
                    let normal = (u - v).normalize();

                    arc_vertex_positions.extend([u, v, u, v, v, u]);
                    arc_vertex_normals.extend([
                        Vec3::ZERO,
                        Vec3::ZERO,
                        normal,
                        Vec3::ZERO,
                        normal,
                        normal,
                    ]);

                    arc_vertex_uvs.extend([
                        Vec2::ZERO,
                        (v - u).length() * Vec2::X,
                        Vec2::Y,
                        (v - u).length() * Vec2::X,
                        (v - u).length() * Vec2::X + Vec2::Y,
                        Vec2::Y,
                    ]);

                    arc_vertex_colors.extend([color; 6])
                }
            }
        }

        for quad in self.quads.iter() {
            for (quad_edge_index, edge) in quad.edges.iter().enumerate() {
                let edge = &self.edges[*edge];

                let color = Self::ARC_COLORS[[
                    quad.cotangent + 1,
                    quad.tangent,
                    quad.cotangent,
                    quad.tangent + 1,
                ][quad_edge_index]]; //[*arc_type.min(&6)];

                let u = quad.pos.as_vec3() * vec3(2.0, 3.0, 2.0) * 0.25;
                let v = edge.pos.as_vec3() * vec3(2.0, 3.0, 2.0) * 0.5;
                let normal = (u - v).normalize();

                arc_vertex_positions.extend([u, v, u, v, v, u]);
                arc_vertex_normals.extend([
                    Vec3::ZERO,
                    Vec3::ZERO,
                    normal,
                    Vec3::ZERO,
                    normal,
                    normal,
                ]);

                arc_vertex_uvs.extend([
                    Vec2::ZERO,
                    (v - u).length() * Vec2::X,
                    Vec2::Y,
                    (v - u).length() * Vec2::X,
                    (v - u).length() * Vec2::X + Vec2::Y,
                    Vec2::Y,
                ]);

                arc_vertex_colors.extend([color; 6])
            }
        }

        let mut edges = Mesh::new(bevy::render::render_resource::PrimitiveTopology::TriangleList);
        edges.insert_attribute(Mesh::ATTRIBUTE_POSITION, arc_vertex_positions);
        edges.insert_attribute(Mesh::ATTRIBUTE_NORMAL, arc_vertex_normals);
        edges.insert_attribute(Mesh::ATTRIBUTE_UV_0, arc_vertex_uvs);
        edges.insert_attribute(Mesh::ATTRIBUTE_COLOR, arc_vertex_colors);
        edges
    }
}
