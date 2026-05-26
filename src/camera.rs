use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;

const MIN_ZOOM: f32 = 0.2;
const MAX_ZOOM: f32 = 5.0;
const ZOOM_STEP: f32 = 0.1;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera)
            .add_systems(Update, (pan_camera, zoom_camera));
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn pan_camera(
    buttons: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    mut cameras: Query<(&mut Transform, &Projection), With<Camera2d>>,
) {
    let dragging = buttons.pressed(MouseButton::Middle);
    if !dragging {
        motion.clear();
        return;
    }
    let Ok((mut transform, projection)) = cameras.single_mut() else {
        return;
    };
    let scale = match projection {
        Projection::Orthographic(o) => o.scale,
        _ => 1.0,
    };
    for ev in motion.read() {
        transform.translation.x -= ev.delta.x * scale;
        transform.translation.y += ev.delta.y * scale;
    }
}

fn zoom_camera(
    mut wheel: MessageReader<MouseWheel>,
    mut cameras: Query<&mut Projection, With<Camera2d>>,
) {
    let Ok(mut projection) = cameras.single_mut() else {
        return;
    };
    let Projection::Orthographic(ortho) = projection.as_mut() else {
        return;
    };
    for ev in wheel.read() {
        ortho.scale = (ortho.scale * (1.0 - ev.y * ZOOM_STEP)).clamp(MIN_ZOOM, MAX_ZOOM);
    }
}
