use bevy::prelude::*;

#[derive(Resource)]
pub struct IconAtlas {
    pub image: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
}

#[derive(Clone, Copy)]
pub enum GameIcon {
    Power = 0,
    Compute = 1,
    Workers = 2,
    Score = 3,
    Build = 4,
    Workflows = 5,
    SpawnWorker = 6,
    FactoryInfo = 7,
}

pub fn spawn_icon(
    commands: &mut Commands,
    icon_atlas: &IconAtlas,
    icon: GameIcon,
    size: f32,
) -> Entity {
    commands
        .spawn((
            ImageNode {
                image: icon_atlas.image.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: icon_atlas.layout.clone(),
                    index: icon as usize,
                }),
                ..default()
            },
            Node {
                width: Val::Px(size),
                height: Val::Px(size),
                ..default()
            },
        ))
        .id()
}

fn load_icon_atlas(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
) {
    let image = asset_server.load("icons.png");
    let layout = TextureAtlasLayout::from_grid(UVec2::new(16, 16), 8, 1, None, None);
    let layout_handle = texture_atlases.add(layout);

    commands.insert_resource(IconAtlas {
        image,
        layout: layout_handle,
    });
}

pub struct IconPlugin;

impl Plugin for IconPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_icon_atlas);
    }
}
