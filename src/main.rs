#![feature(array_zip)]
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

use image::{ImageBuffer, RgbImage};
use rand::prelude::*;

type Color = [u8; 3];
type Location = [usize; 2];

#[derive(Debug)]
struct VecMap<K, V>
where
    K: Copy + Eq + Hash + Debug,
    V: Debug,
{
    vec: Vec<K>,
    map: HashMap<K, V>,
}

impl<K, V> VecMap<K, V>
where
    K: Copy + Eq + Hash + Debug,
    V: Debug,
{
    fn new() -> VecMap<K, V> {
        VecMap {
            vec: vec![],
            map: HashMap::new(),
        }
    }
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        let old_value = self.map.insert(key, value);
        if old_value.is_none() {
            self.vec.push(key);
        }
        old_value
    }
    fn is_empty(&self) -> bool {
        assert_eq!(self.vec.is_empty(), self.map.is_empty());
        self.vec.is_empty()
    }
    fn rand_remove<R: Rng>(&mut self, rng: &mut R, fuzz: f64) -> Option<(K, V)> {
        let mut index = rng.gen_range(0..self.vec.len());
        if rng.gen::<f64>() < fuzz {
            index = self.vec.len() - 1;
        }
        let key = self.vec.swap_remove(index);
        self.map.remove(&key).map(|value| (key, value))
    }
    fn insert_modify<F>(&mut self, key: K, value: V, modify: F)
    where
        F: FnOnce(&mut V),
    {
        if let Some(old_value) = self.map.get_mut(&key) {
            modify(old_value);
        } else {
            self.map.insert(key, value);
            self.vec.push(key);
        }
    }
}

fn make_image(
    size: usize,
    num_seeds: usize,
    max: u8,
    long: u8,
    halving: f64,
    smoothing: isize,
    fuzz: f64,
    seed: u64,
) -> RgbImage {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut locations_to_colors: HashMap<Location, Color> = HashMap::new();
    let mut boundary: VecMap<Location, (Color, usize)> = VecMap::new();
    for _ in 0..num_seeds {
        let location: Location = [rng.gen_range(0..size), rng.gen_range(0..size)];
        let color: Color = rng.gen();
        boundary.insert(location, (color, 0));
    }
    let mut count = 0;
    loop {
        if count % ((size * size) / 10) == 0 {
            println!("{}: {}/{}", count, locations_to_colors.len(), size * size);
        }
        count += 1;
        if boundary.is_empty() {
            break;
        }
        let (location, (color, steps)) = boundary
            .rand_remove(&mut rng, fuzz)
            .expect("Checked nonempty");
        locations_to_colors
            .entry(location)
            .and_modify(|c| *c = c.zip(color).map(|(c1, c2)| c1 / 2 + c2 / 2 + (c1 & c2 & 1)))
            .or_insert(color);
        for direction_offset in vec![[-1, 0], [0, -1], [1, 0], [0, 1]] {
            if rng.gen::<f64>() > 0.5 {
                continue;
            }
            let maybe_new_location = location
                .map(|l| l as isize)
                .zip(direction_offset)
                .map(|(l, d)| l + d);
            if maybe_new_location
                .iter()
                .any(|&l| l < 0 || l >= size as isize)
            {
                continue;
            }
            let new_location = maybe_new_location.map(|l| l as usize);
            if locations_to_colors.contains_key(&new_location) {
                let mut found_empty = false;
                for off in -smoothing..=smoothing {
                    for (dr, dc) in vec![(0, off), (off, 0), (off, off), (-off, off)] {
                        let nr = dr + new_location[0] as isize;
                        let nc = dc + new_location[1] as isize;
                        if nr < 0 || nr >= size as isize || nc < 0 || nc >= size as isize {
                            continue;
                        }
                        let nr = nr as usize;
                        let nc = nc as usize;
                        if !locations_to_colors.contains_key(&[nr, nc]) {
                            found_empty = true;
                            break;
                        }
                    }
                }
                if !found_empty {
                    continue;
                }
            }
            let fdiffusion =
                (max - long) as f64 * 2.0f64.powf(-(steps as f64) / halving) + long as f64;
            let diffusion = fdiffusion as i16;
            let color_offset = [
                rng.gen_range(-diffusion..=diffusion),
                rng.gen_range(-diffusion..=diffusion),
                rng.gen_range(-diffusion..=diffusion),
            ];
            let new_color = color
                .map(|c| c as i16)
                .zip(color_offset)
                .map(|(c, off)| (c + off).clamp(0, 255) as u8);
            boundary.insert_modify(
                new_location,
                (new_color, steps + 1),
                |(old_color, old_steps)| {
                    *old_color = old_color
                        .zip(new_color)
                        .map(|(c1, c2)| c1 / 2 + c2 / 2 + (c1 & c2 & 1));
                    *old_steps = (*old_steps + steps + 1) / 2;
                },
            );
        }
    }
    let mut img: RgbImage = ImageBuffer::new(size as u32, size as u32);
    for (location, color) in locations_to_colors {
        img.put_pixel(location[0] as u32, location[1] as u32, image::Rgb(color))
    }
    img
}

fn main() {
    let size = 1000;
    let num_seeds = 10;
    let max = 255;
    let long = 6;
    let halving = 4;
    let smoothing = 4;
    let fuzz = 0.8;
    let seed = 1;
    let filename = format!(
        "img-{}-{}-{}-{}-{}-{}-{}-{}.png",
        size, num_seeds, max, long, halving, smoothing, fuzz, seed
    );
    println!("Start {}", filename);
    let img = make_image(
        size,
        num_seeds,
        max,
        long,
        halving as f64,
        smoothing,
        fuzz,
        seed,
    );
    img.save(&filename).unwrap();
}
