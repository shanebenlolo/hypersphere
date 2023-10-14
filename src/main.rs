use learn_wgpu::run;
use pollster;

fn main() {
    pollster::block_on(run());
}
