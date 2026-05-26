use crate::GameState;
use crate::sim::dynamic::{DynamicState, TestResult};
use crate::sim::fem::StructureStatus;
use bevy::prelude::*;

const COLOR_RED: Color = Color::srgb(1.0, 0.45, 0.45);
const COLOR_GREEN: Color = Color::srgb(0.45, 1.0, 0.45);
const COLOR_YELLOW: Color = Color::srgb(1.0, 0.9, 0.4);

#[derive(Component)]
struct ModeLabel;

#[derive(Component)]
struct StatusLabel;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_hud)
            .add_systems(Update, (update_mode_label, update_status_label));
    }
}

fn spawn_hud(mut commands: Commands) {
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Px(36.0),
            padding: UiRect::axes(Val::Px(16.0), Val::Px(6.0)),
            column_gap: Val::Px(24.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .insert(BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)))
        .with_children(|root| {
            root.spawn((
                Text::new("EDIT"),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(COLOR_GREEN),
                ModeLabel,
            ));
            root.spawn((
                Text::new(""),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(COLOR_RED),
                StatusLabel,
            ));
        });
}

fn update_mode_label(state: Res<State<GameState>>, mut q: Query<&mut Text, With<ModeLabel>>) {
    if !state.is_changed() {
        return;
    }
    let label = match state.get() {
        GameState::Menu => "MENU",
        GameState::Edit => "EDIT",
        GameState::Test => "TEST",
        GameState::Result => "RESULT",
    };
    for mut text in &mut q {
        **text = label.into();
    }
}

fn update_status_label(
    state: Res<State<GameState>>,
    structure: Res<StructureStatus>,
    dynamic: Res<DynamicState>,
    mut q: Query<(&mut Text, &mut TextColor), With<StatusLabel>>,
) {
    let (label, color) = match state.get() {
        GameState::Edit => match *structure {
            StructureStatus::Ok => ("", COLOR_RED),
            StructureStatus::Unstable => ("UNSTABLE", COLOR_RED),
        },
        GameState::Test => match dynamic.result {
            TestResult::Running => ("TESTING…  [ESC]", COLOR_YELLOW),
            TestResult::Won => ("WON!  [ESC]", COLOR_GREEN),
            TestResult::Lost => ("FAILED  [ESC]", COLOR_RED),
        },
        _ => ("", COLOR_RED),
    };
    for (mut text, mut text_color) in &mut q {
        **text = label.into();
        text_color.0 = color;
    }
}
