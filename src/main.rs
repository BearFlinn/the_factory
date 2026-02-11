use bevy::prelude::*;
use the_factory::camera::CameraPlugin;
use the_factory::configure_system_sets;
use the_factory::grid::GridPlugin;
use the_factory::materials::MaterialsPlugin;
use the_factory::resources::ResourcesPlugin;
use the_factory::structures::BuildingsPlugin;
use the_factory::systems::SystemsPlugin;
use the_factory::ui::UIPlugin;
use the_factory::workers::WorkersPlugin;

fn main() {
    let mut app = App::new();
    configure_system_sets(&mut app);
    app.add_plugins(DefaultPlugins)
        .add_plugins((
            GridPlugin,
            ResourcesPlugin,
            MaterialsPlugin,
            SystemsPlugin,
            BuildingsPlugin,
            WorkersPlugin,
            CameraPlugin,
            UIPlugin,
        ))
        .run();
}
