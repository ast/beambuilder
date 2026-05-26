use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;

const MIN_ZOOM: f32 = 0.2;
const MAX_ZOOM: f32 = 5.0;
const ZOOM_STEP: f32 = 0.1;

/// Squared distance (in screen pixels) the cursor must travel during a right-button
/// press before it is treated as a drag rather than a click.
const DRAG_THRESHOLD_SQ: f32 = 36.0; // 6 px

/// Tracks whether the current right-mouse press has moved past the drag threshold.
/// Read by both the camera pan system (to enable panning) and by the delete
/// system (to suppress delete when the click was actually a pan).
#[derive(Resource, Default, Debug)]
pub struct RmbDrag {
    pub cumulative: Vec2,
    pub held: bool,
}

impl RmbDrag {
    /// True if the current (or most recent) right-mouse press has been moved
    /// far enough to be considered a drag.
    pub fn is_drag(&self) -> bool {
        self.cumulative.length_squared() > DRAG_THRESHOLD_SQ
    }
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RmbDrag>()
            .add_systems(Startup, spawn_camera)
            .add_systems(
                Update,
                (track_rmb_drag, pan_camera, zoom_camera).chain(),
            );
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn track_rmb_drag(
    buttons: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    mut drag: ResMut<RmbDrag>,
) {
    if buttons.just_pressed(MouseButton::Right) {
        drag.cumulative = Vec2::ZERO;
        drag.held = true;
    }
    if drag.held {
        for ev in motion.read() {
            drag.cumulative += ev.delta;
        }
    } else {
        motion.clear();
    }
    if buttons.just_released(MouseButton::Right) {
        drag.held = false;
    }
}

fn pan_camera(
    buttons: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    mut cameras: Query<(&mut Transform, &Projection), With<Camera2d>>,
    drag: Res<RmbDrag>,
) {
    let active = buttons.pressed(MouseButton::Right) && drag.is_drag();
    if !active {
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
