use dive_deco::Depth;

fn main() {
    let depth_1 = Depth::from_meters(10.);
    println!("{}m", depth_1.as_meters()); // 10m
    println!("{}ft", depth_1.as_feet()); // 32.80ft

    let depth_2 = Depth::from_feet(100.);
    println!("{}m", depth_2.as_meters()); // 30.48m
    println!("{}ft", depth_2.as_feet()); // 100ft

    let depths_sum = depth_1 + depth_2;
    println!(
        "{}m + {}m = {}m / {}",
        depth_1.as_meters(),
        depth_2.as_feet(),
        depths_sum.as_meters(),
        depths_sum.as_feet()
    ); // 10m + 100ft = 40.48m / 132.80ft
}
