use bevy::{
    asset::RenderAssetUsages, mesh::Indices, prelude::*, render::render_resource::PrimitiveTopology,
};

use crate::ExtrusionBuffers;

pub fn extrusion_buffers_to_mesh(buffers: &ExtrusionBuffers) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, buffers.positions.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, buffers.normals.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, buffers.uvs.clone());
    if !buffers.indices.is_empty() {
        mesh.insert_indices(Indices::U32(buffers.indices.clone()));
    }
    let _ = mesh.generate_tangents();
    mesh
}
