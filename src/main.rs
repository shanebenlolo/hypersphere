use hypersphere::run;
use pollster;

#[tokio::main]
async fn main() {
    pollster::block_on(run());
}
