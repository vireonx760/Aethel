#[path = "demo/app.rs"]
mod app;
#[path = "demo/render.rs"]
mod render;
#[path = "demo/sim.rs"]
mod sim;
#[path = "demo/ui.rs"]
mod ui;

fn main() {
    app::run();
}
