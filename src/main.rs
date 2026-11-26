fn main() -> anyhow::Result<()> {
    // Create application and runner
    let app = voxel_game_engine::app::App::new("Voxel Game Engine");
    let mut runner = voxel_game_engine::app::AppRunner::builder(app)
        .with_logger(log::LevelFilter::Info)
        .build();

    // Start application
    runner.run()
}
