use bevy::{asset::AssetPlugin, prelude::*, render::render_resource::PrimitiveTopology};

use crate::{
    RibbonExtrusion, SplineEditCommand, SplineEditRequest, SplineExtrusion, SplineExtrusionShape,
    SplineMeshTarget, SplinePath, SplineToolsPlugin,
};

fn empty_mesh(meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
    meshes.add(Mesh::new(
        PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::MAIN_WORLD | bevy::asset::RenderAssetUsages::RENDER_WORLD,
    ))
}

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(SplineToolsPlugin::default());
    app
}

#[test]
fn spawned_spline_rebuilds_cache_and_mesh() {
    let mut app = test_app();
    let handle = {
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        empty_mesh(&mut meshes)
    };
    let entity = app
        .world_mut()
        .spawn((
            SplinePath::default(),
            SplineMeshTarget::new(
                handle.clone(),
                SplineExtrusion {
                    shape: SplineExtrusionShape::Ribbon(RibbonExtrusion {
                        half_width: 0.6,
                        thickness: 0.0,
                        use_control_point_width: true,
                    }),
                    ..default()
                },
            ),
        ))
        .id();

    app.update();
    app.update();

    let diagnostics = app.world().get::<crate::SplineDiagnostics>(entity).unwrap();
    assert!(diagnostics.curve_revision > 0);
    assert!(diagnostics.last_vertex_count > 0);
    assert!(diagnostics.total_length > 0.0);
    assert!(
        app.world()
            .resource::<Assets<Mesh>>()
            .get(&handle)
            .is_some()
    );
}

#[test]
fn edit_request_only_rebuilds_the_target_spline() {
    let mut app = test_app();
    let (handle_a, handle_b) = {
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        (empty_mesh(&mut meshes), empty_mesh(&mut meshes))
    };

    let entity_a = app
        .world_mut()
        .spawn((
            SplinePath::default(),
            SplineMeshTarget::new(handle_a, SplineExtrusion::default()),
        ))
        .id();
    let entity_b = app
        .world_mut()
        .spawn((
            SplinePath::default(),
            SplineMeshTarget::new(handle_b, SplineExtrusion::default()),
        ))
        .id();

    app.update();
    app.update();

    let before_a = app
        .world()
        .get::<crate::SplineDiagnostics>(entity_a)
        .unwrap()
        .curve_revision;
    let before_b = app
        .world()
        .get::<crate::SplineDiagnostics>(entity_b)
        .unwrap()
        .curve_revision;

    app.world_mut()
        .resource_mut::<Messages<SplineEditRequest>>()
        .write(SplineEditRequest {
            entity: entity_a,
            command: SplineEditCommand::MovePoint {
                index: 1,
                position: Vec3::new(4.0, 0.0, 0.0),
            },
        });

    app.update();
    app.update();

    let after_a = app
        .world()
        .get::<crate::SplineDiagnostics>(entity_a)
        .unwrap()
        .curve_revision;
    let after_b = app
        .world()
        .get::<crate::SplineDiagnostics>(entity_b)
        .unwrap()
        .curve_revision;
    assert!(after_a > before_a);
    assert_eq!(after_b, before_b);
}
