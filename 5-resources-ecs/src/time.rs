use std::time::Instant;

use anyhow::Result;
use bevy_ecs::{
    schedule::Schedule,
    system::{Res, ResMut, Resource},
    world::World,
};

use crate::gpu::GpuContext;

pub fn setup_time(world: &mut World, schedule: &mut Schedule) -> Result<()> {
    let time = TimeContext::new();
    world.insert_resource(time);
    let time_history = TimeHistory::new();
    world.insert_resource(time_history);

    schedule.add_systems(time_system);

    Ok(())
}

#[derive(Resource)]
pub struct TimeContext {
    last_frame: Instant,
    frame_time_history: Vec<f32>,
    pub delta: f32,
    pub total: f32,
}
impl TimeContext {
    pub fn new() -> Self {
        Self {
            last_frame: Instant::now(),
            frame_time_history: Vec::new(),
            delta: 0.0,
            total: 0.0,
        }
    }
    pub fn update(&mut self) {
        let now = Instant::now();
        let delta = now.duration_since(self.last_frame).as_secs_f32();
        self.delta = delta;
        self.total += delta;
        self.last_frame = now;
    }
}

#[derive(Resource)]
pub struct TimeHistory {
    pub frame_times: Vec<f32>,
}
impl TimeHistory {
    pub fn new() -> Self {
        Self {
            frame_times: Vec::new(),
        }
    }
    pub fn update(&mut self, delta: f32) {
        // Calculate the average frame time
        self.frame_times.push(delta);
        if self.frame_times.len() > 500 {
            self.frame_times.remove(0);
        }
    }
    pub fn average_frame_time(&self) -> f32 {
        self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32
    }
    pub fn percentile(&self, percentile: f32) -> f32 {
        let mut sorted_times = self.frame_times.clone();
        sorted_times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx = ((self.frame_times.len() as f32 * percentile) as usize).max(0);
        sorted_times.get(idx).copied().unwrap_or(0.0)
    }
}

pub fn time_system(
    mut time: ResMut<TimeContext>,
    mut time_history: ResMut<TimeHistory>,
    gpu: Res<GpuContext>,
) {
    time.update();
    time_history.update(time.delta);

    let average_frame_time = time_history.average_frame_time();
    let percentile_95 = time_history.percentile(0.95);
    let percentile_99 = time_history.percentile(0.99);
    gpu.window.set_title(&format!(
        "Frame time: {:.2}ms (95th: {:.2}ms, 99th: {:.2}ms)",
        average_frame_time * 1000.0,
        percentile_95 * 1000.0,
        percentile_99 * 1000.0
    ));
}
